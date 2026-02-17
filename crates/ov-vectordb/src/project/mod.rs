//! Project and ProjectGroup management.

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;

use crate::collection::{Collection, CollectionConfig};
use crate::error::{Result, VectorDbError};

/// A Project manages multiple Collections.
pub struct Project {
    name: String,
    path: Option<PathBuf>,
    collections: RwLock<HashMap<String, Collection>>,
}

impl Project {
    /// Create a volatile (in-memory) project.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: None,
            collections: RwLock::new(HashMap::new()),
        }
    }

    /// Create a persistent project.
    pub fn with_path(name: &str, path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path)?;
        let mut proj = Self {
            name: name.to_string(),
            path: Some(path.clone()),
            collections: RwLock::new(HashMap::new()),
        };
        proj.load_existing()?;
        Ok(proj)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn has_collection(&self, name: &str) -> bool {
        self.collections.read().contains_key(name)
    }

    pub fn list_collections(&self) -> Vec<String> {
        self.collections.read().keys().cloned().collect()
    }

    pub fn create_collection(&self, name: &str, config: CollectionConfig) -> Result<()> {
        let mut colls = self.collections.write();
        if colls.contains_key(name) {
            return Err(VectorDbError::CollectionAlreadyExists(name.to_string()));
        }
        let coll = if let Some(ref base) = self.path {
            let coll_path = base.join(name);
            Collection::with_path(config, coll_path)?
        } else {
            Collection::new(config)
        };
        colls.insert(name.to_string(), coll);
        Ok(())
    }

    pub fn drop_collection(&self, name: &str) {
        let mut colls = self.collections.write();
        if let Some(coll) = colls.remove(name) {
            coll.drop_collection();
        }
    }

    /// Access a collection by name.
    /// Returns a guard that provides &Collection.
    pub fn with_collection<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Collection) -> R,
    {
        let colls = self.collections.read();
        let coll = colls.get(name).ok_or_else(|| VectorDbError::CollectionNotFound(name.to_string()))?;
        Ok(f(coll))
    }

    pub fn close(&self) {
        let colls = self.collections.read();
        for coll in colls.values() {
            coll.close();
        }
    }

    fn load_existing(&mut self) -> Result<()> {
        if let Some(ref base) = self.path {
            if !base.exists() { return Ok(()); }
            for entry in std::fs::read_dir(base)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() { continue; }
                let coll_path = entry.path();
                let config_path = coll_path.join("collection_config.json");
                if !config_path.exists() { continue; }
                if let Ok(data) = std::fs::read(&config_path) {
                    if let Ok(config) = serde_json::from_slice::<CollectionConfig>(&data) {
                        let name = config.name.clone();
                        if let Ok(coll) = Collection::with_path(config, coll_path) {
                            self.collections.write().insert(name, coll);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        self.close();
    }
}

/// ProjectGroup manages multiple Projects.
pub struct ProjectGroup {
    path: Option<PathBuf>,
    projects: RwLock<HashMap<String, Project>>,
}

impl ProjectGroup {
    /// Create a volatile project group.
    pub fn new() -> Self {
        let pg = Self {
            path: None,
            projects: RwLock::new(HashMap::new()),
        };
        pg.projects.write().insert("default".to_string(), Project::new("default"));
        pg
    }

    /// Create a persistent project group.
    pub fn with_path(path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path)?;
        let mut pg = Self {
            path: Some(path.clone()),
            projects: RwLock::new(HashMap::new()),
        };
        pg.load_existing()?;
        // Ensure default project exists
        if !pg.projects.read().contains_key("default") {
            let default_path = path.join("default");
            std::fs::create_dir_all(&default_path)?;
            let proj = Project::with_path("default", default_path)?;
            pg.projects.write().insert("default".to_string(), proj);
        }
        Ok(pg)
    }

    pub fn has_project(&self, name: &str) -> bool {
        self.projects.read().contains_key(name)
    }

    pub fn list_projects(&self) -> Vec<String> {
        self.projects.read().keys().cloned().collect()
    }

    pub fn create_project(&self, name: &str) -> Result<()> {
        let mut projects = self.projects.write();
        if projects.contains_key(name) {
            return Err(VectorDbError::ProjectAlreadyExists(name.to_string()));
        }
        let proj = if let Some(ref base) = self.path {
            let proj_path = base.join(name);
            std::fs::create_dir_all(&proj_path)?;
            Project::with_path(name, proj_path)?
        } else {
            Project::new(name)
        };
        projects.insert(name.to_string(), proj);
        Ok(())
    }

    pub fn delete_project(&self, name: &str) {
        let mut projects = self.projects.write();
        if let Some(proj) = projects.remove(name) {
            proj.close();
        }
    }

    /// Access a project by name.
    pub fn with_project<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Project) -> R,
    {
        let projects = self.projects.read();
        let proj = projects.get(name).ok_or_else(|| VectorDbError::ProjectNotFound(name.to_string()))?;
        Ok(f(proj))
    }

    pub fn close(&self) {
        let projects = self.projects.read();
        for proj in projects.values() {
            proj.close();
        }
    }

    fn load_existing(&mut self) -> Result<()> {
        if let Some(ref base) = self.path {
            if !base.exists() { return Ok(()); }
            for entry in std::fs::read_dir(base)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                let proj_path = entry.path();
                if let Ok(proj) = Project::with_path(&name, proj_path) {
                    self.projects.write().insert(name, proj);
                }
            }
        }
        Ok(())
    }
}

impl Default for ProjectGroup {
    fn default() -> Self {
        Self::new()
    }
}
