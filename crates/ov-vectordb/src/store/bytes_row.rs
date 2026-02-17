//! Binary serialization for structured rows (port of C++ BytesRow).

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaFieldType {
    Int64,
    Uint64,
    Float32,
    String,
    Binary,
    Boolean,
    ListInt64,
    ListString,
    ListFloat32,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub data_type: SchemaFieldType,
    pub id: usize,
    pub default_value: Option<Value>,
}

/// Schema for BytesRow serialization.
#[derive(Debug, Clone)]
pub struct BytesRowSchema {
    pub fields: Vec<FieldSchema>,
    name_to_idx: HashMap<String, usize>,
}

impl BytesRowSchema {
    pub fn new(fields: Vec<FieldSchema>) -> Self {
        let name_to_idx: HashMap<String, usize> = fields.iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), i))
            .collect();
        Self { fields, name_to_idx }
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.name_to_idx.get(name).copied()
    }
}

/// BytesRow serializer/deserializer.
pub struct BytesRow {
    schema: BytesRowSchema,
}

impl BytesRow {
    pub fn new(schema: BytesRowSchema) -> Self {
        Self { schema }
    }

    /// Serialize a map of field values to bytes.
    pub fn serialize(&self, data: &HashMap<String, Value>) -> Vec<u8> {
        let mut buf = Vec::new();
        for field in &self.schema.fields {
            let val = data.get(&field.name)
                .or(field.default_value.as_ref())
                .cloned()
                .unwrap_or(Value::Null);
            self.write_field(&mut buf, &field.data_type, &val);
        }
        buf
    }

    /// Deserialize bytes to a map of field values.
    pub fn deserialize(&self, data: &[u8]) -> HashMap<String, Value> {
        let mut cursor = Cursor::new(data);
        let mut result = HashMap::new();
        for field in &self.schema.fields {
            if let Some(val) = self.read_field(&mut cursor, &field.data_type) {
                result.insert(field.name.clone(), val);
            }
        }
        result
    }

    /// Deserialize a single field by name.
    pub fn deserialize_field(&self, data: &[u8], field_name: &str) -> Option<Value> {
        let result = self.deserialize(data);
        result.get(field_name).cloned()
    }

    fn write_field(&self, buf: &mut Vec<u8>, dt: &SchemaFieldType, val: &Value) {
        match dt {
            SchemaFieldType::Int64 => {
                let v = val.as_i64().unwrap_or(0);
                buf.write_i64::<LittleEndian>(v).unwrap();
            }
            SchemaFieldType::Uint64 => {
                let v = val.as_u64().unwrap_or(0);
                buf.write_u64::<LittleEndian>(v).unwrap();
            }
            SchemaFieldType::Float32 => {
                let v = val.as_f64().unwrap_or(0.0) as f32;
                buf.write_f32::<LittleEndian>(v).unwrap();
            }
            SchemaFieldType::Boolean => {
                let v = val.as_bool().unwrap_or(false);
                buf.push(if v { 1 } else { 0 });
            }
            SchemaFieldType::String => {
                let s = val.as_str().unwrap_or("");
                let bytes = s.as_bytes();
                buf.write_u16::<LittleEndian>(bytes.len() as u16).unwrap();
                buf.write_all(bytes).unwrap();
            }
            SchemaFieldType::Binary => {
                let s = val.as_str().unwrap_or("");
                let bytes = s.as_bytes();
                buf.write_u32::<LittleEndian>(bytes.len() as u32).unwrap();
                buf.write_all(bytes).unwrap();
            }
            SchemaFieldType::ListInt64 => {
                let arr = val.as_array();
                let items: Vec<i64> = arr.map(|a| a.iter().filter_map(|v| v.as_i64()).collect()).unwrap_or_default();
                buf.write_u32::<LittleEndian>(items.len() as u32).unwrap();
                for item in items {
                    buf.write_i64::<LittleEndian>(item).unwrap();
                }
            }
            SchemaFieldType::ListFloat32 => {
                let arr = val.as_array();
                let items: Vec<f32> = arr.map(|a| a.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect()).unwrap_or_default();
                buf.write_u32::<LittleEndian>(items.len() as u32).unwrap();
                for item in items {
                    buf.write_f32::<LittleEndian>(item).unwrap();
                }
            }
            SchemaFieldType::ListString => {
                let arr = val.as_array();
                let items: Vec<&str> = arr.map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
                buf.write_u32::<LittleEndian>(items.len() as u32).unwrap();
                for item in items {
                    let bytes = item.as_bytes();
                    buf.write_u16::<LittleEndian>(bytes.len() as u16).unwrap();
                    buf.write_all(bytes).unwrap();
                }
            }
        }
    }

    fn read_field(&self, cursor: &mut Cursor<&[u8]>, dt: &SchemaFieldType) -> Option<Value> {
        match dt {
            SchemaFieldType::Int64 => {
                cursor.read_i64::<LittleEndian>().ok().map(|v| Value::from(v))
            }
            SchemaFieldType::Uint64 => {
                cursor.read_u64::<LittleEndian>().ok().map(|v| Value::from(v))
            }
            SchemaFieldType::Float32 => {
                cursor.read_f32::<LittleEndian>().ok().map(|v| Value::from(v as f64))
            }
            SchemaFieldType::Boolean => {
                cursor.read_u8().ok().map(|v| Value::from(v != 0))
            }
            SchemaFieldType::String => {
                let len = cursor.read_u16::<LittleEndian>().ok()? as usize;
                let mut buf = vec![0u8; len];
                std::io::Read::read_exact(cursor, &mut buf).ok()?;
                String::from_utf8(buf).ok().map(Value::from)
            }
            SchemaFieldType::Binary => {
                let len = cursor.read_u32::<LittleEndian>().ok()? as usize;
                let mut buf = vec![0u8; len];
                std::io::Read::read_exact(cursor, &mut buf).ok()?;
                String::from_utf8(buf).ok().map(Value::from)
            }
            SchemaFieldType::ListInt64 => {
                let count = cursor.read_u32::<LittleEndian>().ok()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(Value::from(cursor.read_i64::<LittleEndian>().ok()?));
                }
                Some(Value::Array(items))
            }
            SchemaFieldType::ListFloat32 => {
                let count = cursor.read_u32::<LittleEndian>().ok()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(Value::from(cursor.read_f32::<LittleEndian>().ok()? as f64));
                }
                Some(Value::Array(items))
            }
            SchemaFieldType::ListString => {
                let count = cursor.read_u32::<LittleEndian>().ok()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    let len = cursor.read_u16::<LittleEndian>().ok()? as usize;
                    let mut buf = vec![0u8; len];
                    std::io::Read::read_exact(cursor, &mut buf).ok()?;
                    items.push(Value::from(String::from_utf8(buf).ok()?));
                }
                Some(Value::Array(items))
            }
        }
    }
}
