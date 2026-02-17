use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

/// File-based KV store with atomic writes.
pub struct FileStore {
    base_path: Option<PathBuf>,
}

impl FileStore {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        Self { base_path }
    }

    fn resolve_path(&self, key: &str) -> PathBuf {
        if let Some(ref base) = self.base_path {
            base.join(key)
        } else {
            PathBuf::from(key)
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.resolve_path(key);
        fs::read(&path).ok()
    }

    pub fn put(&self, key: &str, value: &[u8]) -> bool {
        let path = self.resolve_path(key);
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        let tmp_path = path.with_extension("tmp");
        let result = (|| -> std::io::Result<()> {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(value)?;
            f.flush()?;
            f.sync_all()?;
            fs::rename(&tmp_path, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
            return false;
        }
        true
    }

    pub fn delete(&self, key: &str) -> bool {
        let path = self.resolve_path(key);
        fs::remove_file(&path).is_ok() || !path.exists()
    }

    pub fn exists(&self, key: &str) -> bool {
        self.resolve_path(key).exists()
    }
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new(None)
    }
}
