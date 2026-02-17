//! Comprehensive test suite for ov-vectordb
//! Ported from Python tests + new Rust-specific tests (~100 tests total)

use ov_vectordb::{
    Collection, CollectionConfig, FieldDef, FieldType,
    index::{FlatIndex, HnswIndex, VectorIndex},
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

// ============================================================
// Distance / Metric Tests (7)
// ============================================================

#[test]
fn test_inner_product() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0, 6.0];
    assert!((distance::inner_product(&a, &b) - 32.0).abs() < 1e-6);
}

#[test]
fn test_l2_squared() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    assert!((distance::l2_squared(&a, &b) - 2.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_identical() {
    let a = vec![1.0, 2.0, 3.0];
    assert!((distance::cosine_similarity(&a, &a) - 1.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    assert!(distance::cosine_similarity(&a, &b).abs() < 1e-6);
}

#[test]
fn test_normalize_vector() {
    let mut v = vec![3.0, 4.0];
    distance::normalize_vector(&mut v);
    assert!((v[0] - 0.6).abs() < 1e-6);
    assert!((v[1] - 0.8).abs() < 1e-6);
}

#[test]
fn test_normalize_zero_vector() {
    let mut v = vec![0.0, 0.0, 0.0];
    distance::normalize_vector(&mut v);
    assert_eq!(v, vec![0.0, 0.0, 0.0]);
}

#[test]
fn test_distance_metric_from_str() {
    assert_eq!(DistanceMetric::from_str_loose("cosine"), DistanceMetric::Cosine);
    assert_eq!(DistanceMetric::from_str_loose("l2"), DistanceMetric::L2);
    assert_eq!(DistanceMetric::from_str_loose("ip"), DistanceMetric::Ip);
    assert_eq!(DistanceMetric::from_str_loose("unknown"), DistanceMetric::Cosine);
}

// ============================================================
// Flat Index Tests (12)
// ============================================================

#[test]
fn test_flat_index_basic() {
    let idx = FlatIndex::new(3, DistanceMetric::Cosine);
    idx.insert(1, &[1.0, 0.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0, 0.0]).unwrap();
    idx.insert(3, &[0.7, 0.7, 0.0]).unwrap();
    let result = idx.search(&[1.0, 0.0, 0.0], 2).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result.ids[0], 1);
}

#[test]
fn test_flat_index_insert_update() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(1, &[0.0, 1.0]).unwrap();
    assert_eq!(idx.len(), 1);
    let result = idx.search(&[0.0, 1.0], 1).unwrap();
    assert_eq!(result.ids[0], 1);
    assert!(result.scores[0] > 0.99);
}

#[test]
fn test_flat_index_delete() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0]).unwrap();
    idx.delete(1).unwrap();
    assert_eq!(idx.len(), 1);
    let result = idx.search(&[1.0, 0.0], 10).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result.ids[0], 2);
}

#[test]
fn test_flat_index_empty_search() {
    let idx = FlatIndex::new(4, DistanceMetric::Cosine);
    let result = idx.search(&[1.0, 0.0, 0.0, 0.0], 10).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_flat_index_dimension_mismatch() {
    let idx = FlatIndex::new(3, DistanceMetric::Cosine);
    assert!(idx.insert(1, &[1.0, 0.0]).is_err());
    idx.insert(1, &[1.0, 0.0, 0.0]).unwrap();
    assert!(idx.search(&[1.0, 0.0], 1).is_err());
}

#[test]
fn test_flat_index_l2() {
    let idx = FlatIndex::new(2, DistanceMetric::L2);
    idx.insert(1, &[0.0, 0.0]).unwrap();
    idx.insert(2, &[1.0, 0.0]).unwrap();
    idx.insert(3, &[10.0, 10.0]).unwrap();
    let result = idx.search(&[0.0, 0.0], 3).unwrap();
    assert_eq!(result.ids[0], 1);
    assert_eq!(result.ids[1], 2);
}

#[test]
fn test_flat_index_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("flat_test");
    let idx = FlatIndex::new(3, DistanceMetric::Cosine);
    idx.insert(1, &[1.0, 0.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0, 0.0]).unwrap();
    idx.save(&path).unwrap();

    let mut idx2 = FlatIndex::new(1, DistanceMetric::Cosine);
    idx2.load(&path).unwrap();
    assert_eq!(idx2.len(), 2);
    let result = idx2.search(&[1.0, 0.0, 0.0], 1).unwrap();
    assert_eq!(result.ids[0], 1);
}

#[test]
fn test_flat_index_batch_insert() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    let labels = vec![1, 2, 3];
    let vectors = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
    idx.insert_batch(&labels, &vectors).unwrap();
    assert_eq!(idx.len(), 3);
}

#[test]
fn test_flat_index_delete_nonexistent() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.delete(999).unwrap();
}

#[test]
fn test_flat_index_search_topk_zero() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    let result = idx.search(&[1.0, 0.0], 0).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_flat_index_ip_score() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(2, &[0.5, 0.5]).unwrap();
    let result = idx.search(&[1.0, 0.0], 2).unwrap();
    assert_eq!(result.ids[0], 1);
    assert!((result.scores[0] - 1.0).abs() < 1e-6);
}

#[test]
fn test_flat_index_many_deletes() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    for i in 0..100 {
        idx.insert(i, &[i as f32, 0.0]).unwrap();
    }
    for i in 0..50 {
        idx.delete(i).unwrap();
    }
    assert_eq!(idx.len(), 50);
    let result = idx.search(&[99.0, 0.0], 1).unwrap();
    assert_eq!(result.ids[0], 99);
}

// ============================================================
// HNSW Index Tests (8)
// ============================================================

#[test]
fn test_hnsw_basic() {
    let idx = HnswIndex::new(3, DistanceMetric::Cosine);
    idx.insert(1, &[1.0, 0.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0, 0.0]).unwrap();
    idx.insert(3, &[0.7, 0.7, 0.0]).unwrap();
    let result = idx.search(&[1.0, 0.0, 0.0], 2).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result.ids[0], 1);
}

#[test]
fn test_hnsw_insert_update() {
    let idx = HnswIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(1, &[0.0, 1.0]).unwrap();
    assert_eq!(idx.len(), 1);
}

#[test]
fn test_hnsw_delete() {
    let idx = HnswIndex::new(2, DistanceMetric::Ip);
    idx.insert(1, &[1.0, 0.0]).unwrap();
    idx.insert(2, &[0.0, 1.0]).unwrap();
    idx.delete(1).unwrap();
    assert_eq!(idx.len(), 1);
}

#[test]
fn test_hnsw_empty_search() {
    let idx = HnswIndex::new(4, DistanceMetric::Cosine);
    let result = idx.search(&[1.0, 0.0, 0.0, 0.0], 10).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_hnsw_dimension_mismatch() {
    let idx = HnswIndex::new(3, DistanceMetric::Cosine);
    assert!(idx.insert(1, &[1.0]).is_err());
}

#[test]
fn test_hnsw_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("hnsw_test");
    let idx = HnswIndex::new(3, DistanceMetric::Cosine);
    for i in 0u64..20 {
        let v = vec![(i as f32) / 20.0, 1.0 - (i as f32) / 20.0, 0.5];
        idx.insert(i, &v).unwrap();
    }
    idx.save(&path).unwrap();
    let mut idx2 = HnswIndex::new(3, DistanceMetric::Cosine);
    idx2.load(&path).unwrap();
    assert_eq!(idx2.len(), 20);
}

#[test]
fn test_hnsw_recall() {
    use rand::Rng;
    let dim = 32;
    let n = 200u64;
    let idx = HnswIndex::with_params(dim, DistanceMetric::Cosine, 16, 200, 100);
    let flat = FlatIndex::new(dim, DistanceMetric::Cosine);
    let mut rng = rand::thread_rng();
    for i in 0..n {
        let vec: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect();
        idx.insert(i, &vec).unwrap();
        flat.insert(i, &vec).unwrap();
    }
    let query: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect();
    let flat_result = flat.search(&query, 10).unwrap();
    let hnsw_result = idx.search(&query, 10).unwrap();
    let flat_set: std::collections::HashSet<u64> = flat_result.ids.iter().copied().collect();
    let hnsw_set: std::collections::HashSet<u64> = hnsw_result.ids.iter().copied().collect();
    let overlap = flat_set.intersection(&hnsw_set).count();
    let recall = overlap as f64 / 10.0;
    assert!(recall >= 0.5, "HNSW recall too low: {}", recall);
}

#[test]
fn test_hnsw_needs_rebuild() {
    let idx = HnswIndex::new(2, DistanceMetric::Ip);
    for i in 0u64..10 {
        idx.insert(i, &[i as f32, 0.0]).unwrap();
    }
    assert!(!idx.needs_rebuild());
    for i in 0u64..8 {
        idx.delete(i).unwrap();
    }
    assert!(idx.needs_rebuild());
}

// ============================================================
// Filter Tests (14)
// ============================================================

#[test]
fn test_filter_must() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "status", "conds": ["active"]})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), json!("active"));
    assert!(filter.matches(&fields));
    fields.insert("status".to_string(), json!("inactive"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_must_not() {
    let filter = Filter::from_json(&json!({"op": "must_not", "field": "status", "conds": ["deleted"]})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("status".to_string(), json!("active"));
    assert!(filter.matches(&fields));
    fields.insert("status".to_string(), json!("deleted"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_range_int() {
    let filter = Filter::from_json(&json!({"op": "range", "field": "count", "gt": 10, "lte": 50})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("count".to_string(), json!(20));
    assert!(filter.matches(&fields));
    fields.insert("count".to_string(), json!(10));
    assert!(!filter.matches(&fields));
    fields.insert("count".to_string(), json!(50));
    assert!(filter.matches(&fields));
    fields.insert("count".to_string(), json!(51));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_range_out() {
    let filter = Filter::from_json(&json!({"op": "range_out", "field": "price", "gte": 100, "lte": 200})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("price".to_string(), json!(50));
    assert!(filter.matches(&fields));
    fields.insert("price".to_string(), json!(150));
    assert!(!filter.matches(&fields));
    fields.insert("price".to_string(), json!(250));
    assert!(filter.matches(&fields));
}

#[test]
fn test_filter_prefix() {
    let filter = Filter::from_json(&json!({"op": "prefix", "field": "name", "prefix": "app"})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("apple"));
    assert!(filter.matches(&fields));
    fields.insert("name".to_string(), json!("banana"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_contains() {
    let filter = Filter::from_json(&json!({"op": "contains", "field": "tags", "substring": "er"})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("tags".to_string(), json!("cherry"));
    assert!(filter.matches(&fields));
    fields.insert("tags".to_string(), json!("apple"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_regex_suffix() {
    let filter = Filter::from_json(&json!({"op": "regex", "field": "tag", "pattern": "999$"})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("tag".to_string(), json!("tag_999"));
    assert!(filter.matches(&fields));
    fields.insert("tag".to_string(), json!("tag_998"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_regex_prefix_alt() {
    let filter = Filter::from_json(&json!({"op": "regex", "field": "name", "pattern": "^(a|d)"})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("apple"));
    assert!(filter.matches(&fields));
    fields.insert("name".to_string(), json!("date"));
    assert!(filter.matches(&fields));
    fields.insert("name".to_string(), json!("banana"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_and() {
    let filter = Filter::from_json(&json!({
        "op": "and",
        "conds": [
            {"op": "must", "field": "category", "conds": ["electronics"]},
            {"op": "range", "field": "price", "gt": 100}
        ]
    })).unwrap();
    let mut fields = HashMap::new();
    fields.insert("category".to_string(), json!("electronics"));
    fields.insert("price".to_string(), json!(200));
    assert!(filter.matches(&fields));
    fields.insert("price".to_string(), json!(50));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_or() {
    let filter = Filter::from_json(&json!({
        "op": "or",
        "conds": [
            {"op": "must", "field": "category", "conds": ["books"]},
            {"op": "must", "field": "category", "conds": ["clothing"]}
        ]
    })).unwrap();
    let mut fields = HashMap::new();
    fields.insert("category".to_string(), json!("books"));
    assert!(filter.matches(&fields));
    fields.insert("category".to_string(), json!("electronics"));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_nested_and_or() {
    let filter = Filter::from_json(&json!({
        "op": "and",
        "conds": [
            {"op": "must", "field": "category", "conds": ["electronics"]},
            {
                "op": "or",
                "conds": [
                    {"op": "contains", "field": "tags", "substring": "apple"},
                    {"op": "range", "field": "rating", "gt": 48}
                ]
            }
        ]
    })).unwrap();
    let mut fields = HashMap::new();
    fields.insert("category".to_string(), json!("electronics"));
    fields.insert("tags".to_string(), json!("mobile,apple,new"));
    fields.insert("rating".to_string(), json!(45));
    assert!(filter.matches(&fields));

    fields.insert("tags".to_string(), json!("laptop,windows"));
    fields.insert("rating".to_string(), json!(49));
    assert!(filter.matches(&fields));

    fields.insert("rating".to_string(), json!(40));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_missing_field() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "nonexistent", "conds": ["x"]})).unwrap();
    let fields = HashMap::new();
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_must_list_field() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "tags", "conds": ["b"]})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("tags".to_string(), json!(["a", "b", "c"]));
    assert!(filter.matches(&fields));
    fields.insert("tags".to_string(), json!(["x", "y"]));
    assert!(!filter.matches(&fields));
}

#[test]
fn test_filter_must_numeric() {
    let filter = Filter::from_json(&json!({"op": "must", "field": "score", "conds": [99]})).unwrap();
    let mut fields = HashMap::new();
    fields.insert("score".to_string(), json!(99));
    assert!(filter.matches(&fields));
    fields.insert("score".to_string(), json!(98));
    assert!(!filter.matches(&fields));
}

// ============================================================
// KV Store Tests (5)
// ============================================================

#[test]
fn test_memory_kv_store() {
    let store = MemoryKvStore::new();
    store.put("key1", b"value1".to_vec());
    store.put("key2", b"value2".to_vec());
    assert_eq!(store.get("key1").unwrap(), b"value1");
    assert_eq!(store.len(), 2);
    store.delete("key1");
    assert!(store.get("key1").is_none());
    assert_eq!(store.len(), 1);
    store.clear();
    assert!(store.is_empty());
}

#[test]
fn test_multi_table_store() {
    let store = MultiTableStore::new();
    store.write(
        &["a".to_string(), "b".to_string()],
        &[b"va".to_vec(), b"vb".to_vec()],
        "table1",
    );
    let result = store.read(&["a".to_string(), "b".to_string(), "c".to_string()], "table1");
    assert_eq!(result[0].as_deref(), Some(b"va".as_slice()));
    assert_eq!(result[1].as_deref(), Some(b"vb".as_slice()));
    assert!(result[2].is_none());
}

#[test]
fn test_multi_table_store_seek() {
    let store = MultiTableStore::new();
    store.write(
        &["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()],
        &[b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()],
        "t",
    );
    let after = store.seek_to_end("3", "t");
    assert_eq!(after.len(), 2);
    let before = store.begin_to_seek("2", "t");
    assert_eq!(before.len(), 2);
}

#[test]
fn test_file_store() {
    let dir = TempDir::new().unwrap();
    let store = FileStore::new(Some(dir.path().to_path_buf()));
    assert!(store.put("test.bin", b"hello world"));
    assert_eq!(store.get("test.bin").unwrap(), b"hello world");
    assert!(store.exists("test.bin"));
    assert!(store.delete("test.bin"));
    assert!(store.get("test.bin").is_none());
}

#[test]
fn test_file_store_nested() {
    let dir = TempDir::new().unwrap();
    let store = FileStore::new(Some(dir.path().to_path_buf()));
    assert!(store.put("a/b/c.bin", b"nested"));
    assert_eq!(store.get("a/b/c.bin").unwrap(), b"nested");
}

// ============================================================
// BytesRow Tests (5)
// ============================================================

#[test]
fn test_bytes_row_basic() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "id".into(), data_type: SchemaFieldType::Int64, id: 0, default_value: None },
        FieldSchema { name: "score".into(), data_type: SchemaFieldType::Float32, id: 1, default_value: None },
        FieldSchema { name: "name".into(), data_type: SchemaFieldType::String, id: 2, default_value: None },
        FieldSchema { name: "active".into(), data_type: SchemaFieldType::Boolean, id: 3, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("id".into(), json!(42));
    data.insert("score".into(), json!(0.95));
    data.insert("name".into(), json!("viking"));
    data.insert("active".into(), json!(true));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["id"], json!(42));
    assert!((result["score"].as_f64().unwrap() - 0.95).abs() < 0.01);
    assert_eq!(result["name"], json!("viking"));
    assert_eq!(result["active"], json!(true));
}

#[test]
fn test_bytes_row_lists() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "tags".into(), data_type: SchemaFieldType::ListString, id: 0, default_value: None },
        FieldSchema { name: "embedding".into(), data_type: SchemaFieldType::ListFloat32, id: 1, default_value: None },
        FieldSchema { name: "counts".into(), data_type: SchemaFieldType::ListInt64, id: 2, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("tags".into(), json!(["AI", "Vector"]));
    data.insert("embedding".into(), json!([0.1, 0.2, 0.3]));
    data.insert("counts".into(), json!([1, 10, 100]));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["tags"].as_array().unwrap().len(), 2);
    assert_eq!(result["counts"].as_array().unwrap().len(), 3);
}

#[test]
fn test_bytes_row_unicode() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "text".into(), data_type: SchemaFieldType::String, id: 0, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("text".into(), json!("你好世界🌍"));
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["text"], json!("你好世界🌍"));
}

#[test]
fn test_bytes_row_defaults() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "id".into(), data_type: SchemaFieldType::Int64, id: 0, default_value: Some(json!(999)) },
        FieldSchema { name: "name".into(), data_type: SchemaFieldType::String, id: 1, default_value: Some(json!("default")) },
    ]);
    let row = BytesRow::new(schema);
    let data = HashMap::new();
    let bytes = row.serialize(&data);
    let result = row.deserialize(&bytes);
    assert_eq!(result["id"], json!(999));
    assert_eq!(result["name"], json!("default"));
}

#[test]
fn test_bytes_row_deserialize_field() {
    let schema = BytesRowSchema::new(vec![
        FieldSchema { name: "a".into(), data_type: SchemaFieldType::Int64, id: 0, default_value: None },
        FieldSchema { name: "b".into(), data_type: SchemaFieldType::String, id: 1, default_value: None },
    ]);
    let row = BytesRow::new(schema);
    let mut data = HashMap::new();
    data.insert("a".into(), json!(123));
    data.insert("b".into(), json!("hello"));
    let bytes = row.serialize(&data);
    assert_eq!(row.deserialize_field(&bytes, "a").unwrap(), json!(123));
    assert_eq!(row.deserialize_field(&bytes, "b").unwrap(), json!("hello"));
}

// ============================================================
// Meta Tests (2)
// ============================================================

#[test]
fn test_volatile_dict() {
    let mut dict = VolatileDict::new(HashMap::new());
    dict.set("key".into(), json!("value"));
    assert_eq!(dict.get("key").unwrap(), &json!("value"));
    dict.remove("key");
    assert!(dict.get("key").is_none());
}

#[test]
fn test_persistent_dict() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("meta.json");
    {
        let mut dict = PersistentDict::new(path.clone(), HashMap::new());
        dict.set("key".into(), json!("persisted"));
    }
    let dict = PersistentDict::new(path, HashMap::new());
    assert_eq!(dict.get("key").unwrap(), &json!("persisted"));
}

// ============================================================
// Collection Tests (16)
// ============================================================

fn make_test_collection() -> Collection {
    let config = CollectionConfig {
        name: "test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "embedding".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(4) },
            FieldDef { name: "category".into(), field_type: FieldType::String, is_primary_key: false, dim: None },
            FieldDef { name: "score".into(), field_type: FieldType::Int64, is_primary_key: false, dim: None },
        ],
        description: String::new(),
    };
    Collection::new(config)
}

#[test]
fn test_collection_upsert_and_search() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
        HashMap::from([("id".into(), json!(2)), ("embedding".into(), json!([0.0, 1.0, 0.0, 0.0])), ("category".into(), json!("B")), ("score".into(), json!(20))]),
        HashMap::from([("id".into(), json!(3)), ("embedding".into(), json!([0.7, 0.7, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(30))]),
    ];
    let result = coll.upsert_data(&data).unwrap();
    assert_eq!(result.ids.len(), 3);
    assert_eq!(coll.count(), 3);
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 2, 0, None).unwrap();
    assert_eq!(search.data.len(), 2);
}

#[test]
fn test_collection_search_with_filter() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
        HashMap::from([("id".into(), json!(2)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("B")), ("score".into(), json!(20))]),
        HashMap::from([("id".into(), json!(3)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(30))]),
    ];
    coll.upsert_data(&data).unwrap();
    let filter = json!({"op": "must", "field": "category", "conds": ["A"]});
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 10, 0, Some(&filter)).unwrap();
    assert_eq!(search.data.len(), 2);
    for item in &search.data {
        assert_eq!(item.fields["category"], json!("A"));
    }
}

#[test]
fn test_collection_search_with_offset() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data: Vec<_> = (0..5).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
            ("category".into(), json!("A")),
            ("score".into(), json!(i)),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    let offset = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 2, 2, None).unwrap();
    assert_eq!(offset.data.len(), 2);
}

#[test]
fn test_collection_delete() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
        HashMap::from([("id".into(), json!(2)), ("embedding".into(), json!([0.0, 1.0, 0.0, 0.0])), ("category".into(), json!("B")), ("score".into(), json!(20))]),
    ];
    coll.upsert_data(&data).unwrap();
    coll.delete_data(&[json!(1)]);
    assert_eq!(coll.count(), 1);
}

#[test]
fn test_collection_delete_all() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
    ];
    coll.upsert_data(&data).unwrap();
    coll.delete_all_data();
    assert_eq!(coll.count(), 0);
}

#[test]
fn test_collection_fetch() {
    let coll = make_test_collection();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
    ];
    coll.upsert_data(&data).unwrap();
    let fetched = coll.fetch_data(&[json!(1)]);
    assert!(fetched[0].is_some());
    let fetched_miss = coll.fetch_data(&[json!(999)]);
    assert!(fetched_miss[0].is_none());
}

#[test]
fn test_collection_upsert_update() {
    let coll = make_test_collection();
    let data1 = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
    ];
    coll.upsert_data(&data1).unwrap();
    let data2 = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([0.0, 1.0, 0.0, 0.0])), ("category".into(), json!("B")), ("score".into(), json!(20))]),
    ];
    coll.upsert_data(&data2).unwrap();
    assert_eq!(coll.count(), 1);
    let fetched = coll.fetch_data(&[json!(1)]);
    let fields = fetched[0].as_ref().unwrap();
    assert_eq!(fields["category"], json!("B"));
}

#[test]
fn test_collection_dimension_mismatch() {
    let coll = make_test_collection();
    let data = vec![
        HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0])), ("category".into(), json!("A")), ("score".into(), json!(10))]),
    ];
    assert!(coll.upsert_data(&data).is_err());
}

#[test]
fn test_collection_index_not_found() {
    let coll = make_test_collection();
    assert!(coll.search_by_vector("nonexistent", &[1.0, 0.0, 0.0, 0.0], 1, 0, None).is_err());
}

#[test]
fn test_collection_duplicate_index() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    assert!(coll.create_index("idx", IndexConfig::default()).is_err());
}

#[test]
fn test_collection_list_and_drop_index() {
    let coll = make_test_collection();
    coll.create_index("idx1", IndexConfig::default()).unwrap();
    coll.create_index("idx2", IndexConfig::default()).unwrap();
    assert_eq!(coll.list_indexes().len(), 2);
    coll.drop_index("idx1");
    assert!(!coll.has_index("idx1"));
    assert!(coll.has_index("idx2"));
}

#[test]
fn test_collection_empty_operations() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let search = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 10, 0, None).unwrap();
    assert!(search.data.is_empty());
    coll.delete_data(&[json!(1)]);
    coll.delete_all_data();
}

#[test]
fn test_collection_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("coll_test");
    {
        let config = CollectionConfig {
            name: "persist_test".into(),
            fields: vec![
                FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
                FieldDef { name: "embedding".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(3) },
            ],
            description: String::new(),
        };
        let coll = Collection::with_path(config, path.clone()).unwrap();
        let data = vec![
            HashMap::from([("id".into(), json!(1)), ("embedding".into(), json!([1.0, 0.0, 0.0]))]),
            HashMap::from([("id".into(), json!(2)), ("embedding".into(), json!([0.0, 1.0, 0.0]))]),
        ];
        coll.upsert_data(&data).unwrap();
    }
    let config = CollectionConfig {
        name: "persist_test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "embedding".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(3) },
        ],
        description: String::new(),
    };
    let coll = Collection::with_path(config, path).unwrap();
    assert_eq!(coll.count(), 2);
}

#[test]
fn test_collection_hnsw_index() {
    let coll = make_test_collection();
    let cfg = IndexConfig {
        index_type: "hnsw".to_string(),
        distance: DistanceMetric::Cosine,
        scalar_index_fields: vec![],
    };
    coll.create_index("hnsw_idx", cfg).unwrap();
    let data: Vec<_> = (0..50).map(|i| {
        HashMap::from([
            ("id".into(), json!(i)),
            ("embedding".into(), json!([(i as f64) / 50.0, 1.0 - (i as f64) / 50.0, 0.5, 0.0])),
            ("category".into(), json!("test")),
            ("score".into(), json!(i)),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    let search = coll.search_by_vector("hnsw_idx", &[1.0, 0.0, 0.5, 0.0], 5, 0, None).unwrap();
    assert_eq!(search.data.len(), 5);
}

#[test]
fn test_collection_range_filter() {
    let coll = make_test_collection();
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let data: Vec<_> = (0..5).map(|i| {
        HashMap::from([
            ("id".into(), json!(i + 1)),
            ("embedding".into(), json!([1.0, 0.0, 0.0, 0.0])),
            ("category".into(), json!("test")),
            ("score".into(), json!((i + 1) * 10)),
        ])
    }).collect();
    coll.upsert_data(&data).unwrap();
    let filter = json!({"op": "range", "field": "score", "gt": 20, "lte": 40});
    let result = coll.search_by_vector("idx", &[1.0, 0.0, 0.0, 0.0], 10, 0, Some(&filter)).unwrap();
    assert_eq!(result.data.len(), 2);
}

// ============================================================
// Project Tests (4)
// ============================================================

#[test]
fn test_project_volatile() {
    let proj = Project::new("test");
    let config = CollectionConfig {
        name: "coll1".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
        ],
        description: String::new(),
    };
    proj.create_collection("coll1", config).unwrap();
    assert!(proj.has_collection("coll1"));
    assert_eq!(proj.list_collections().len(), 1);
    proj.drop_collection("coll1");
    assert!(!proj.has_collection("coll1"));
}

#[test]
fn test_project_persistent() {
    let dir = TempDir::new().unwrap();
    {
        let proj = Project::with_path("test", dir.path().to_path_buf()).unwrap();
        let config = CollectionConfig {
            name: "coll1".into(),
            fields: vec![
                FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
                FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
            ],
            description: String::new(),
        };
        proj.create_collection("coll1", config).unwrap();
        proj.with_collection("coll1", |c| {
            c.upsert_data(&[HashMap::from([("id".into(), json!(1)), ("vec".into(), json!([1.0, 0.0]))])]).unwrap();
        }).unwrap();
    }
    let proj = Project::with_path("test", dir.path().to_path_buf()).unwrap();
    assert!(proj.has_collection("coll1"));
    proj.with_collection("coll1", |c| {
        assert_eq!(c.count(), 1);
    }).unwrap();
}

#[test]
fn test_project_duplicate_collection() {
    let proj = Project::new("test");
    let config = CollectionConfig { name: "c".into(), fields: vec![], description: String::new() };
    proj.create_collection("c", config.clone()).unwrap();
    assert!(proj.create_collection("c", config).is_err());
}

#[test]
fn test_project_collection_not_found() {
    let proj = Project::new("test");
    assert!(proj.with_collection("nonexistent", |_| {}).is_err());
}

// ============================================================
// ProjectGroup Tests (3)
// ============================================================

#[test]
fn test_project_group_volatile() {
    let pg = ProjectGroup::new();
    assert!(pg.has_project("default"));
    pg.create_project("proj1").unwrap();
    assert!(pg.has_project("proj1"));
    assert_eq!(pg.list_projects().len(), 2);
    pg.delete_project("proj1");
    assert!(!pg.has_project("proj1"));
}

#[test]
fn test_project_group_persistent() {
    let dir = TempDir::new().unwrap();
    {
        let pg = ProjectGroup::with_path(dir.path().to_path_buf()).unwrap();
        pg.create_project("p1").unwrap();
    }
    let pg = ProjectGroup::with_path(dir.path().to_path_buf()).unwrap();
    assert!(pg.has_project("p1"));
}

#[test]
fn test_project_group_duplicate() {
    let pg = ProjectGroup::new();
    assert!(pg.create_project("default").is_err());
}

// ============================================================
// Concurrent Safety Tests (2)
// ============================================================

#[test]
fn test_concurrent_flat_insert() {
    use std::sync::Arc;
    use std::thread;
    let idx = Arc::new(FlatIndex::new(2, DistanceMetric::Ip));
    let mut handles = vec![];
    for t in 0u64..4 {
        let idx = Arc::clone(&idx);
        handles.push(thread::spawn(move || {
            for i in 0u64..100 {
                let label = t * 100 + i;
                idx.insert(label, &[label as f32, 0.0]).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(idx.len(), 400);
}

#[test]
fn test_concurrent_collection_upsert() {
    use std::sync::Arc;
    use std::thread;
    let config = CollectionConfig {
        name: "concurrent".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(2) },
        ],
        description: String::new(),
    };
    let coll = Arc::new(Collection::new(config));
    coll.create_index("idx", IndexConfig::default()).unwrap();
    let mut handles = vec![];
    for t in 0..4 {
        let coll = Arc::clone(&coll);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let id = t * 50 + i;
                let data = vec![HashMap::from([
                    ("id".into(), json!(id)),
                    ("vec".into(), json!([id as f64, 0.0])),
                ])];
                coll.upsert_data(&data).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(coll.count(), 200);
}

// ============================================================
// Large Scale Tests (4)
// ============================================================

#[test]
fn test_large_scale_flat() {
    let idx = FlatIndex::new(4, DistanceMetric::Cosine);
    for i in 0u64..1000 {
        let v = vec![(i as f32).sin(), (i as f32).cos(), 0.5, 0.0];
        idx.insert(i, &v).unwrap();
    }
    assert_eq!(idx.len(), 1000);
    let result = idx.search(&[1.0, 0.0, 0.5, 0.0], 10).unwrap();
    assert_eq!(result.len(), 10);
}

#[test]
fn test_large_scale_hnsw() {
    let idx = HnswIndex::with_params(8, DistanceMetric::Cosine, 16, 100, 50);
    for i in 0u64..500 {
        let v: Vec<f32> = (0..8).map(|d| ((i * 7 + d) as f32).sin()).collect();
        idx.insert(i, &v).unwrap();
    }
    assert_eq!(idx.len(), 500);
    let result = idx.search(&[0.5; 8], 10).unwrap();
    assert_eq!(result.len(), 10);
}

#[test]
fn test_large_dimension_2048() {
    let dim = 2048;
    let idx = FlatIndex::new(dim, DistanceMetric::Cosine);
    let v: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.01).sin()).collect();
    idx.insert(1, &v).unwrap();
    let result = idx.search(&v, 1).unwrap();
    assert_eq!(result.ids[0], 1);
    assert!(result.scores[0] > 0.99);
}

#[test]
fn test_large_dimension_4096() {
    let dim = 4096;
    let idx = FlatIndex::new(dim, DistanceMetric::Ip);
    let v: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.001).cos()).collect();
    idx.insert(1, &v).unwrap();
    let result = idx.search(&v, 1).unwrap();
    assert_eq!(result.ids[0], 1);
}

// ============================================================
// Edge Case Tests (7)
// ============================================================

#[test]
fn test_single_dimension() {
    let idx = FlatIndex::new(1, DistanceMetric::L2);
    idx.insert(1, &[5.0]).unwrap();
    idx.insert(2, &[10.0]).unwrap();
    let result = idx.search(&[6.0], 1).unwrap();
    assert_eq!(result.ids[0], 1);
}

#[test]
fn test_duplicate_id_handling() {
    let idx = FlatIndex::new(2, DistanceMetric::Ip);
    idx.insert(42, &[1.0, 0.0]).unwrap();
    idx.insert(42, &[0.0, 1.0]).unwrap();
    assert_eq!(idx.len(), 1);
    let result = idx.search(&[0.0, 1.0], 1).unwrap();
    assert_eq!(result.ids[0], 42);
    assert!(result.scores[0] > 0.99);
}

#[test]
fn test_error_display() {
    let e = VectorDbError::CollectionNotFound("test".into());
    assert!(e.to_string().contains("test"));
    let e = VectorDbError::DimensionMismatch { expected: 3, got: 5 };
    assert!(e.to_string().contains("3"));
}

#[test]
fn test_field_type_from_str() {
    assert_eq!(FieldType::from_str_loose("int64"), FieldType::Int64);
    assert_eq!(FieldType::from_str_loose("vector"), FieldType::Vector);
    assert_eq!(FieldType::from_str_loose("list<string>"), FieldType::ListString);
    assert_eq!(FieldType::from_str_loose("unknown_type"), FieldType::String);
}

#[test]
fn test_search_result_struct() {
    let sr = ov_vectordb::index::SearchResult::empty();
    assert!(sr.is_empty());
    assert_eq!(sr.len(), 0);
}

#[test]
fn test_collection_config_helpers() {
    let config = CollectionConfig {
        name: "test".into(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::Int64, is_primary_key: true, dim: None },
            FieldDef { name: "vec".into(), field_type: FieldType::Vector, is_primary_key: false, dim: Some(128) },
        ],
        description: "test desc".into(),
    };
    assert_eq!(config.primary_key(), Some("id"));
    assert_eq!(config.dimension(), 128);
    assert!(config.vector_field().is_some());
}

#[test]
fn test_multi_table_clear() {
    let store = MultiTableStore::new();
    store.write(&["k1".into()], &[b"v1".to_vec()], "t1");
    store.write(&["k2".into()], &[b"v2".to_vec()], "t2");
    store.clear();
    assert!(store.read(&["k1".into()], "t1")[0].is_none());
    assert!(store.read(&["k2".into()], "t2")[0].is_none());
}
