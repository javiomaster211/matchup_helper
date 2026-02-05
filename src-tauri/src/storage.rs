//! Storage module for persisting matchup data to JSON

use crate::matchup::{Match, Matchup};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Data directory not found")]
    DataDirNotFound,
}

/// The main data structure stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppData {
    pub matchups: HashMap<String, Matchup>,
    pub matches: HashMap<String, Match>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub last_updated: String,
    pub version: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            last_updated: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
        }
    }
}

/// Storage handler for reading/writing data
pub struct Storage {
    data_path: PathBuf,
}

impl Storage {
    /// Create a new storage handler
    pub fn new() -> Result<Self, StorageError> {
        let data_dir = dirs::data_dir()
            .or_else(|| dirs::config_dir())
            .ok_or(StorageError::DataDirNotFound)?
            .join("matchuphelper");

        // Create directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        let data_path = data_dir.join("data.json");

        Ok(Self { data_path })
    }

    /// Load data from disk
    pub fn load(&self) -> Result<AppData, StorageError> {
        if !self.data_path.exists() {
            return Ok(AppData::default());
        }

        let contents = fs::read_to_string(&self.data_path)?;
        let data: AppData = serde_json::from_str(&contents)?;
        Ok(data)
    }

    /// Save data to disk
    pub fn save(&self, data: &AppData) -> Result<(), StorageError> {
        let mut data = data.clone();
        data.metadata.last_updated = chrono::Utc::now().to_rfc3339();

        let contents = serde_json::to_string_pretty(&data)?;
        fs::write(&self.data_path, contents)?;
        Ok(())
    }

    /// Get the data file path
    pub fn data_path(&self) -> &PathBuf {
        &self.data_path
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matchup::Matchup;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let data_path = dir.path().join("data.json");

        let storage = Storage {
            data_path: data_path.clone(),
        };

        let mut data = AppData::default();
        let matchup = Matchup::new(
            "Darius".to_string(),
            "Garen".to_string(),
            "top".to_string(),
        );
        data.matchups.insert(matchup.id.clone(), matchup);

        storage.save(&data).unwrap();

        assert!(data_path.exists());

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.matchups.len(), 1);
    }
}
