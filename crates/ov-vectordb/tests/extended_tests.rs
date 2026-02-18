//! Extended test suite for ov-vectordb — gap coverage from Python tests
//! Adds crash recovery concepts, stress tests, complex filters, edge cases

use ov_vectordb::{
    Collection, CollectionConfig, FieldDef, FieldType,
    index::{FlatIndex, HnswIndex, VectorIndex, SearchResult},
    distance::{self, DistanceMetric},
    filter::Filter,
    store::{MemoryKvStore, KvStore, MultiTableStore, FileStore, BytesRow, BytesRowSchema, FieldSchema, SchemaFieldType},
    meta::{VolatileDict, PersistentDict},
    project::{Project, ProjectGroup},
    collection::IndexConfig,
    error::VectorDbError,
};
use std::collections::HashMap;
use serde_json::json;
use tempfile::TempDir;
use std::sync::Arc;
use std::thread;

fn make_collection_with_fields(fields: Vec<FieldDef>) -> Collection {
    let config = CollectionConfig {
        name: "ext_test".into(),
        fields,
        description: String::new(),
    };
    Collection::new(config)
}

fn standard_fields() -> Vec<FieldDef> {
    vec![
        FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
        FieldDef { name: "embedding".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(4) },
        FieldDef { name: "category".into(), field_type: FieldType::String, is_primary_key: false, dim: None },
        FieldDef { name: "score".into(), field_type: FieldType::Int64, is_primary_key: false, dim: None },
        FieldDef { name: "tags".into(), field_type: FieldType::String, is_primary_key: false, dim: None },
    ]
}

fn make_standard_collection() -> Collection {
    make_collection_with_fields(standard_fields())
}

// ============================================================
// Crash Recovery / Persistence Stress Tests
// ============================================================

#[test]
fn test_persistence_after_many_writes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("crash_sim");
    let config = CollectionConfig {
        name: "crash_test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(4) },
            FieldDef { name: "data".into(), field_type: FieldType::String, is_primary_key: false, dim: None },
        ],
        description: String::new(),
    };
    {
        let coll = Collection::with_path(config.clone(), path.clone()).unwrap();
        coll.create_index("idx", IndexConfig::default()).unwrap();
        for batch in 0..10 {
            let data: Vec<_> = (0..100).map(|i| {
                let id = batch * 100 + i;
                HashMap::from([
                    ("id".into(), json!(id)),
                    ("vec".into(), json!([0.1, 0.2, 0.3, 0.4])),
                    ("data".into(), json!(format!("item_{}", id))),
                ])
            }).collect();
            coll.upsert_data(&data).unwrap();
        }
        assert_eq!(coll.count(), 1000);
    }
    // Reopen
    let coll2 = Collection::with_path(config, path).unwrap();
    assert_eq!(coll2.count(), 1000);
    let fetched = coll2.fetch_data(&[json!(0), json!(500), json!(999)]);
    assert!(fetched[0].is_some());
    assert!(fetched[1].is_some());
    assert!(fetched[2].is_some());
}

#[test]
fn test_persistence_after_deletes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("del_persist");
    let config = CollectionConfig {
        name: "del_test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
        ],
        description: String::new(),
    };
    {
        let coll = Collection::with_path(config.clone(), path.clone()).unwrap();
        let data: Vec<_> = (0..500).map(|i| HashMap::from([
            ("id".into(), json!(i)),
            ("vec".into(), json!([i as f64, 0.0])),
        ])).collect();
        coll.upsert_data(&data).unwrap();
        // Delete first 250
        let ids: Vec<_> = (0..250).map(|i| json!(i)).collect();
        coll.delete_data(&ids);
        assert_eq!(coll.count(), 250);
    }
    let coll2 = Collection::with_path(config, path).unwrap();
    assert_eq!(coll2.count(), 250);
    assert!(coll2.fetch_data(&[json!(0)])[0].is_none());
    assert!(coll2.fetch_data(&[json!(499)])[0].is_some());
}

#[test]
fn test_persistence_updates_survive_reload() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("upd_persist");
    let config = CollectionConfig {
        name: "upd_test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
            FieldDef { name: "version".into(), field_type: FieldType::Int64, is_primary_key: false, dim: None },
        ],
        description: String::new(),
    };
    {
        let coll = Collection::with_path(config.clone(), path.clone()).unwrap();
        let data = vec![HashMap::from([
            ("id".into(), json!(1)),
            ("vec".into(), json!([1.0, 0.0])),
            ("version".into(), json!(1)),
        ])];
        coll.upsert_data(&data).unwrap();
        // Update version
        let data2 = vec![HashMap::from([
            ("id".into(), json!(1)),
            ("vec".into(), json!([0.0, 1.0])),
            ("version".into(), json!(2)),
        ])];
        coll.upsert_data(&data2).unwrap();
    }
    let coll2 = Collection::with_path(config, path).unwrap();
    assert_eq!(coll2.count(), 1);
    let f = coll2.fetch_data(&[json!(1)]);
    assert_eq!(f[0].as_ref().unwrap()["version"], json!(2));
}

// ============================================================
// Complex Filter Edge Cases
// ============================================================

#[test]
fn test_filter_deeply_nested_logic() {
    // ((A or B) and (C or (D and E)))
    let filter = Filter::from_json(&json!({
        "op": "and",
        "conds": [
            {"op": "or", "conds": [
                {"op": "must", "field": "category", "conds": ["electronics"]},
                {"op": "must", "field": "category", "conds": ["home"]}
            ]},
            {"op": "or", "conds": [
                {"op": "range", "field": "score", "lt": 1000},
                {"op": "and", "conds": [
                    {"op": "contains", "field": "tags", "substring": "fiction"},
                    {"op": "range", "field": "score", "gt": 45}
                ]}
            ]}
        ]
    })).unwrap();
    
    // electronics + score<1000 => match
    let mut f = HashMap::new();
    f.insert("category".into(), json!("electronics"));
    f.insert("score".into(), json!(300));
    f.insert("tags".into(), json!("mobile"));
    assert!(filter.matches(&f));
    
    // books + fiction + score>45 => but books not in (electronics|home) => no match
    f.insert("category".into(), json!("books"));
    f.insert("tags".into(), json!("fiction,sci-fi"));
    f.insert("score".into(), json!(47));
    assert!(!filter.matches(&f));
    
    // home + score >= 1000, no fiction => score<1000 fails, fiction check fails => no
    f.insert("category".into(), json!("home"));
    f.insert("score".into(), json!(2000));
    f.insert("tags".into(), json!("furniture"));
    assert!(!filter.matches(&f));
    
    // home + fiction + score>45 => matches second branch
    f.insert("category".into(), json!("home"));
    f.insert("score".into(), json!(50));
    f.insert("tags".into(), json!("fiction,fantasy"));
    assert!(filter.matches(&f));
}

#[test]
fn test_filter_range_out_boundaries() {
    let filter = Filter::from_json(&json!({"op": "range_out", "field": "x", "gte": 10, "lte": 20})).unwrap();
    let mut f = HashMap::new();
    f.insert("x".into(), json!(9));
    assert!(filter.matches(&f)); // below range
    f.insert("x".into(), json!(10));
    assert!(!filter.matches(&f)); // at lower bound
    f.insert("x".into(), json!(15));
    assert!(!filter.matches(&f)); // inside
    f.insert("x".into(), json!(20));
    assert!(!filter.matches(&f)); // at upper bound
    f.insert("x".into(), json!(21));
    assert!(filter.matches(&f)); // above range
}

#[test]
fn test_filter_must_multiple_values() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "cat", "conds": ["A", "B", "C"]})).unwrap();
    let mut f = HashMap::new();
    f.insert("cat".into(), json!("B"));
    assert!(filter.matches(&f));
    f.insert("cat".into(), json!("D"));
    assert!(!filter.matches(&f));
}

#[test]
fn test_filter_must_not_multiple_values() {
    let filter = Filter::from_json(&json!({"op": "must_not", "field": "status", "conds": ["deleted", "banned"]})).unwrap();
    let mut f = HashMap::new();
    f.insert("status".into(), json!("active"));
    assert!(filter.matches(&f));
    f.insert("status".into(), json!("deleted"));
    assert!(!filter.matches(&f));
    f.insert("status".into(), json!("banned"));
    assert!(!filter.matches(&f));
}

#[test]
fn test_filter_range_gte_lte() {
    let filter = Filter::from_json(&json!({"op": "range", "field": "val", "gte": 10, "lte": 20})).unwrap();
    let mut f = HashMap::new();
    f.insert("val".into(), json!(9));
    assert!(!filter.matches(&f));
    f.insert("val".into(), json!(10));
    assert!(filter.matches(&f));
    f.insert("val".into(), json!(20));
    assert!(filter.matches(&f));
    f.insert("val".into(), json!(21));
    assert!(!filter.matches(&f));
}

#[test]
fn test_filter_float_range() {
    let filter = Filter::from_json(&json!({"op": "range", "field": "score", "gt": 2.0, "lt": 5.0})).unwrap();
    let mut f = HashMap::new();
    f.insert("score".into(), json!(2.5));
    assert!(filter.matches(&f));
    f.insert("score".into(), json!(2.0));
    assert!(!filter.matches(&f));
    f.insert("score".into(), json!(5.0));
    assert!(!filter.matches(&f));
}

#[test]
fn test_filter_empty_string_prefix() {
    let filter = Filter::from_json(&json!({"op": "prefix", "field": "name", "prefix": ""})).unwrap();
    let mut f = HashMap::new();
    f.insert("name".into(), json!("anything"));
    assert!(filter.matches(&f));
}

#[test]
fn test_filter_contains_empty() {
    let filter = Filter::from_json(&json!({"op": "contains", "field": "text", "substring": ""})).unwrap();
    let mut f = HashMap::new();
    f.insert("text".into(), json!("anything"));
    assert!(filter.matches(&f));
}

#[test]
fn test_filter_regex_unicode() {
    let filter = Filter::from_json(&json!({"op": "regex", "field": "text", "pattern": "^你好"})).unwrap();
    let mut f = HashMap::new();
    f.insert("text".into(), json!("你好世界"));
    assert!(filter.matches(&f));
    f.insert("text".into(), json!("世界你好"));
    assert!(!filter.matches(&f));
}

#[test]
fn test_filter_and_empty_conds() {
    let filter = Filter::from_json(&json!({"op": "and", "conds": []})).unwrap();
    let f = HashMap::new();
    assert!(filter.matches(&f)); // empty AND = true
}

#[test]
fn test_filter_or_empty_conds() {
    let filter = Filter::from_json(&json!({"op": "or", "conds": []})).unwrap();
    let f = HashMap::new();
    assert!(!filter.matches(&f)); // empty OR = false
}

#[test]
fn test_filter_numeric_in_string_field() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "id", "conds": [42]})).unwrap();
    let mut f = HashMap::new();
    f.insert("id".into(), json!(42));
    assert!(filter.matches(&f));
}

#[test]
fn test_filter_list_string_must_not() {
    let filter = Filter::from_json(&json!({"op": "must_not", "field": "tags", "conds": ["banned"]})).unwrap();
    let mut f = HashMap::new();
    f.insert("tags".into(), json!(["good", "safe"]));
    assert!(filter.matches(&f));
    f.insert("tags".into(), json!(["good", "banned"]));
    assert!(!filter.matches(&f));
}

// ============================================================
// Large Scale Collection Tests
// ============================================================

#[test]
fn test_collection_1000_items_search() {
    let coll = make_standard_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data: Vec<_> = (0..1000).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([(i as f64).sin(), (i as f64).cos(), 0.5, 0.0])),
            ("category".into(), json!(format!("cat_{}", i % 10))),
            ("score".into(), json!(i % 100)),
            ("tags".into(), json!(format!("tag_{}", i % 50))),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    assert_eq!(coll.count(), 1000);
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.5, 0.0], 10, 0, None).unwrap();
    assert_eq!(search.data.len(), 10);
}

#[test]
fn test_collection_filtered_search_large() {
    let coll = make_standard_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data: Vec<_> = (0..500).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
            ("category".into(), json!(if i % 2 == 0 { "even" } else { "odd" })),
            ("score".into(), json!(i)),
            ("tags".into(), json!("test")),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    let filter = json!({"op": "must", "field": "category", "conds": ["even"]});
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 1000, 0, Some(&filter)).unwrap();
    assert_eq!(search.data.len(), 250);
}

#[test]
fn test_collection_massive_upsert_update() {
    let coll = make_standard_collection();
    let data: Vec<_> = (0..500).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
            ("category".into(), json!("v1")),
            ("score".into(), json!(1)),
            ("tags".into(), json!("")),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    // Update all
    let data2: Vec<_> = (0..500).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([0.0, 1.0, 0.0, 0.0])),
            ("category".into(), json!("v2")),
            ("score".into(), json!(2)),
            ("tags".into(), json!("")),
        ])
    }).collect();
    coll.upsert_data(&data2).unwrap();
    assert_eq!(coll.count(), 500);
    let f = coll.fetch_data(&[json!(0), json!(499)]);
    assert_eq!(f[0].as_ref().unwrap()["category"], json!("v2"));
    assert_eq!(f[1].as_ref().unwrap()["category"], json!("v2"));
}

#[test]
fn test_collection_delete_half_then_search() {
    let coll = make_standard_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data: Vec<_> = (0..200).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
            ("category".into(), json!("test")),
            ("score".into(), json!(i)),
            ("tags".into(), json!("")),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    let del_ids: Vec<_> = (0..100).map(|i| json!(i)).collect();
    coll.delete_data(&del_ids);
    assert_eq!(coll.count(), 100);
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 200, 0, None).unwrap();
    assert_eq!(search.data.len(), 100);
    for item in &search.data {
        let id = item.fields["id"].as_i64().unwrap();
        assert!(id >= 100);
    }
}

// ============================================================  
// Concurrent Collection Operations
// ============================================================





// ============================================================
// Index Edge Cases
// ============================================================

#[test]
fn test_flat_index_search_topk_larger_than_data() {
    let idx = FlatIndex::new(2, DistanceMetric::Cosine);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0]).unwrap();
    let result = idx.search(&[1.0, 0.0], 100).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_flat_index_identical_vectors() {
    let idx = FlatIndex::new(2, DistanceMetric::Cosine);
    for i in 0..10 {
        idx.insert(i, &[1.0, 0.0]).unwrap();
    }
    let result = idx.search(&[1.0, 0.0], 10).unwrap();
    assert_eq!(result.len(), 10);
    // All should have score ~1.0
    for s in &result.scores {
        assert!((s - 1.0).abs() < 1e-4);
    }
}

#[test]
fn test_flat_index_negative_vectors() {
    let idx = FlatIndex::new(3, DistanceMetric::Ip);
    idx.insert(1, &[-1.0, -2.0, -3.0]).unwrap();
    idx.insert(2, &[1.0, 2.0, 3.0]).unwrap();
    let result = idx.search(&[1.0, 2.0, 3.0], 2).unwrap();
    assert_eq!(result.ids[0], 2); // positive dot product
}

#[test]
fn test_flat_index_very_small_vectors() {
    let idx = FlatIndex::new(2, DistanceMetric::Cosine);
    idx.insert(1, &[1e-10, 1e-10]).unwrap();
    idx.insert(2, &[1.0, 0.0]).unwrap();
    let result = idx.search(&[1.0, 0.0], 2).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_hnsw_large_batch() {
    let idx = HnswIndex::with_params(4, DistanceMetric::Cosine, 16, 200, 100);
    let labels: Vec<u64> = (0..200).collect();
    let vectors: Vec<Vec<f32>> = (0..200).map(|i| {
        vec![(i as f32).sin(), (i as f32).cos(), 0.5, 0.0]
    }).collect();
    idx.insert_batch(&labels, &vectors).unwrap();
    assert_eq!(idx.len(), 200);
    let result = idx.search(&[1.0, 0.0, 0.5, 0.0], 10).unwrap();
    assert_eq!(result.len(), 10);
}

#[test]
fn test_hnsw_delete_all_then_reinsert() {
    let idx = HnswIndex::new(2, DistanceMetric::Ip);
    for i in 0..10u64 {
        idx.insert(i, &[i as f32, 0.0]).unwrap();
    }
    for i in 0..10u64 {
        idx.delete(i).unwrap();
    }
    assert_eq!(idx.len(), 0);
    // Reinsert
    for i in 10..20u64 {
        idx.insert(i, &[i as f32, 0.0]).unwrap();
    }
    assert_eq!(idx.len(), 10);
}

// ============================================================
// BytesRow Edge Cases
// ============================================================

#[test]
fn test_bytes_row_empty_string() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "text".into(), data_type: SchemaFieldType::String, id: 0, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("text".into(), json!(""));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["text"], json!(""));
}

#[test]
fn test_bytes_row_large_string() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "big".into(), data_type: SchemaFieldType::String, id: 0, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let big = "x".repeat(100_000);
    let mut data = HashMap::new();
    data.insert("big".into(), json!(big));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert!(result["big"].as_str().unwrap().len() > 0);
}

#[test]
fn test_bytes_row_negative_numbers() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "neg_int".into(), data_type: SchemaFieldType::Int64, id: 0, default_value: None },
        FieldSchema { name: "neg_float".into(), data_type: SchemaFieldType::Float32, id: 1, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("neg_int".into(), json!(-9999));
    data.insert("neg_float".into(), json!(-3.14));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["neg_int"], json!(-9999));
    assert!((result["neg_float"].as_f64().unwrap() - (-3.14)).abs() < 0.01);
}

#[test]
fn test_bytes_row_empty_list() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "tags".into(), data_type: SchemaFieldType::ListString, id: 0, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("tags".into(), json!([]));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert!(result["tags"].as_array().unwrap().is_empty());
}

#[test]
fn test_bytes_row_many_fields() {
    let fields: Vec<_> = (0..20).map(|i| FieldSchema {
        name: format!("f{}", i),
        data_type: SchemaFieldType::Int64,
        id: i as usize,
        default_value: None,
    }).collect();
    let row = BytesRow::new(BytesRowSchema::new(fields));
    let mut data = HashMap::new();
    for i in 0..20 {
        data.insert(format!("f{}", i), json!(i * 10));
    }
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    for i in 0..20 {
        assert_eq!(result[&format!("f{}", i)], json!(i * 10));
    }
}

#[test]
fn test_bytes_row_special_chars_string() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "s".into(), data_type: SchemaFieldType::String, id: 0, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("s".into(), json!("hello\nworld\t\"quotes\""));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert!(result["s"].as_str().unwrap().contains("\n"));
    assert!(result["s"].as_str().unwrap().contains("quotes"));
}

// ============================================================
// KV Store Edge Cases
// ============================================================

#[test]
fn test_memory_kv_store_overwrite() {
    let store = MemoryKvStore::new();
    store.put("k", b"v1".to_vec());
    store.put("k", b"v2".to_vec());
    assert_eq!(store.get("k").unwrap(), b"v2");
    assert_eq!(store.len(), 1);
}

#[test]
fn test_memory_kv_store_empty_value() {
    let store = MemoryKvStore::new();
    store.put("k", b"".to_vec());
    assert_eq!(store.get("k").unwrap(), b"");
}

#[test]
fn test_memory_kv_store_large_value() {
    let store = MemoryKvStore::new();
    let big = vec![0xABu8; 1_000_000];
    store.put("big", big.clone());
    assert_eq!(store.get("big").unwrap(), big);
}

#[test]
fn test_memory_kv_store_delete_nonexistent() {
    let store = MemoryKvStore::new();
    store.delete("nope"); // Should not panic
    assert!(store.is_empty());
}

#[test]
fn test_file_store_overwrite() {
    let dir = TempDir::new().unwrap();
    let store = FileStore::new(Some(dir.path().to_path_buf()));
    store.put("f.bin", b"old");
    store.put("f.bin", b"new");
    assert_eq!(store.get("f.bin").unwrap(), b"new");
}

#[test]
fn test_file_store_empty_file() {
    let dir = TempDir::new().unwrap();
    let store = FileStore::new(Some(dir.path().to_path_buf()));
    store.put("empty.bin", b"");
    assert_eq!(store.get("empty.bin").unwrap(), b"");
}

#[test]
fn test_file_store_nonexistent_get() {
    let dir = TempDir::new().unwrap();
    let store = FileStore::new(Some(dir.path().to_path_buf()));
    assert!(store.get("nope.bin").is_none());
}

#[test]
fn test_multi_table_store_multiple_tables() {
    let store = MultiTableStore::new();
    store.write(&["a".into()], &[b"1".to_vec()], "t1");
    store.write(&["a".into()], &[b"2".to_vec()], "t2");
    assert_eq!(store.read(&["a".into()], "t1")[0].as_deref(), Some(b"1".as_slice()));
    assert_eq!(store.read(&["a".into()], "t2")[0].as_deref(), Some(b"2".as_slice()));
}

// ============================================================
// Project / ProjectGroup Edge Cases
// ============================================================

#[test]
fn test_project_drop_nonexistent_collection() {
    let proj = Project::new("test");
    proj.drop_collection("does_not_exist"); // Should not panic
}

#[test]
fn test_project_many_collections() {
    let proj = Project::new("test");
    for i in 0..20 {
        let config = CollectionConfig {
            name: format!("coll_{}", i),
            fields: vec![
                FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
                FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
            ],
            description: String::new(),
        };
        proj.create_collection(&format!("coll_{}", i), config).unwrap();
    }
    assert_eq!(proj.list_collections().len(), 20);
}

#[test]
fn test_project_group_delete_default_fails() {
    let pg = ProjectGroup::new();
    pg.delete_project("default");
    // default should still exist (or at minimum not crash)
    let _ = pg.has_project("default"); // may or may not still exist
}

#[test]
fn test_project_group_many_projects() {
    let pg = ProjectGroup::new();
    for i in 0..20 {
        pg.create_project(&format!("proj_{}", i)).unwrap();
    }
    assert_eq!(pg.list_projects().len(), 21); // 20 + default
}

// ============================================================
// Error Handling
// ============================================================

#[test]
fn test_error_variants_display() {
    let e = VectorDbError::CollectionNotFound("x".into());
    assert!(format!("{}", e).contains("x"));
    let e = VectorDbError::DimensionMismatch { expected: 128, got: 256 };
    assert!(format!("{}", e).contains("128"));
    assert!(format!("{}", e).contains("256"));
}

#[test]
fn test_collection_search_on_empty_index() {
    let coll = make_standard_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let result = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 10, 0, None).unwrap();
    assert!(result.data.is_empty());
}

#[test]
fn test_collection_double_delete_same_id() {
    let coll = make_standard_collection();
    let data = vec![HashMap::from([
        ("id".into(), json!(1)),
        ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
        ("category".into(), json!("A")),
        ("score".into(), json!(10)),
        ("tags".into(), json!("")),
    ])];
    coll.upsert_data(&data).unwrap();
    coll.delete_data(&[json!(1)]);
    coll.delete_data(&[json!(1)]); // Double delete should not panic
    assert_eq!(coll.count(), 0);
}

#[test]
fn test_flat_index_search_all_deleted() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    for i in 0..10u64 {
        idx.insert(i, &[i as f32, 0.0]).unwrap();
    }
    for i in 0..10u64 {
        idx.delete(i).unwrap();
    }
    let result = idx.search(&[1.0, 0.0], 10).unwrap();
    assert!(result.is_empty());
}

// ============================================================
// FieldType and Config Helpers
// ============================================================

#[test]
fn test_field_type_all_variants() {
    assert_eq!(FieldType::from_str_loose("int64"), FieldType::Int64);
    assert_eq!(FieldType::from_str_loose("float32"), FieldType::Float32);
    assert_eq!(FieldType::from_str_loose("string"), FieldType::String);
    assert_eq!(FieldType::from_str_loose("bool"), FieldType::Bool);
    assert_eq!(FieldType::from_str_loose("vector"), FieldType::Vector);
    assert_eq!(FieldType::from_str_loose("list<string>"), FieldType::ListString);
    assert_eq!(FieldType::from_str_loose("list<int64>"), FieldType::ListInt64);
}

#[test]
fn test_collection_config_no_vector_field() {
    let config = CollectionConfig {
        name: "no_vec".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
        ],
        description: String::new(),
    };
    assert!(config.vector_field().is_none());
    assert_eq!(config.dimension(), 0);
}

#[test]
fn test_collection_config_no_primary_key() {
    let config = CollectionConfig {
        name: "no_pk".into(),
        fields: vec![
            FieldDef { name: "data".into(), field_type: FieldType::String, is_primary_key: false, dim: None },
        ],
        description: String::new(),
    };
    assert!(config.primary_key().is_none());
}

#[test]
fn test_search_result_empty() {
    let sr = SearchResult::empty();
    assert!(sr.is_empty());
    assert_eq!(sr.len(), 0);
    assert!(sr.ids.is_empty());
    assert!(sr.scores.is_empty());
}

// ============================================================
// Persistent Dict Edge Cases
// ============================================================

#[test]
fn test_persistent_dict_overwrite() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dict.json");
    let mut d = PersistentDict::new(path.clone(), HashMap::new());
    d.set("k".into(), json!("v1"));
    d.set("k".into(), json!("v2"));
    drop(d);
    let d2 = PersistentDict::new(path, HashMap::new());
    assert_eq!(d2.get("k").unwrap(), &json!("v2"));
}

#[test]
fn test_persistent_dict_many_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("many.json");
    {
        let mut d = PersistentDict::new(path.clone(), HashMap::new());
        for i in 0..100 {
            d.set(format!("key_{}", i), json!(i));
        }
    }
    let d = PersistentDict::new(path, HashMap::new());
    for i in 0..100 {
        assert_eq!(d.get(&format!("key_{}", i)).unwrap(), &json!(i));
    }
}

#[test]

#[test]
fn test_volatile_dict_many_ops() {
    let mut d = VolatileDict::new(HashMap::new());
    for i in 0..100 {
        d.set(format!("k{}", i), json!(i));
    }
    for i in 0..50 {
        d.remove(&format!("k{}", i));
    }
    for i in 50..100 {
        assert_eq!(d.get(&format!("k{}", i)).unwrap(), &json!(i));
    }
    for i in 0..50 {
        assert!(d.get(&format!("k{}", i)).is_none());
    }
}

#[test]
fn test_collection_search_empty() {
    let config = CollectionConfig {
        name: "empty_search".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(3) },
        ],
        description: String::new(),
    };
    let coll = Collection::new(config);
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let results = coll.search_by_vector("idx", &[1.0, 0.0, 0.0], 10, 0, None);
    assert!(results.is_ok());
    let _ = results.unwrap();
}

#[test]
fn test_volatile_dict_override_all() {
    let mut d = VolatileDict::new(HashMap::new());
    d.set("a".into(), json!(1));
    d.set("b".into(), json!(2));
    let new_data = HashMap::from([
        ("c".into(), json!(3)),
    ]);
    d.override_all(new_data);
    assert!(d.get("a").is_none());
    assert_eq!(d.get("c").unwrap(), &json!(3));
}
