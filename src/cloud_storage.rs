use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::SystemTime;

pub mod dropbox;
pub mod google_drive;

/// Cloud file metadata that mirrors local FileInfo
#[derive(Debug, Clone)]
pub struct CloudFileInfo {
    pub path: String,              // Cloud path (e.g., "/Books/foo.pdf")
    pub name: String,              // Filename only
    pub extension: String,         // e.g., ".pdf"
    pub size: u64,
    pub modified_time: SystemTime,
    pub is_failed_download: bool,
    pub is_too_small: bool,
}

/// Trait for cloud storage operations
#[async_trait]
pub trait CloudStorage: Send + Sync {
    /// List all files in a directory (recursive)
    async fn list_files(&self, path: &str, max_depth: Option<usize>) -> Result<Vec<CloudFileInfo>>;

    /// Rename a file
    async fn rename_file(&self, from_path: &str, to_path: &str) -> Result<()>;

    /// Delete a file
    async fn delete_file(&self, path: &str) -> Result<()>;

    /// Check if file exists
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Get file metadata
    async fn get_metadata(&self, path: &str) -> Result<CloudFileInfo>;

    /// Compute MD5 hash of file (if supported without downloading)
    /// Returns None if hash computation requires downloading the file
    async fn get_md5_hash(&self, path: &str) -> Result<Option<String>>;
}

/// Cloud provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Dropbox,
    GoogleDrive,
}

impl CloudProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dropbox" => Some(CloudProvider::Dropbox),
            "google-drive" | "googledrive" | "gdrive" => Some(CloudProvider::GoogleDrive),
            _ => None,
        }
    }
}

/// Factory function to create cloud storage backend
pub async fn create_cloud_storage(
    provider: CloudProvider,
    access_token: String,
) -> Result<Box<dyn CloudStorage>> {
    match provider {
        CloudProvider::Dropbox => {
            let backend = dropbox::DropboxBackend::new(access_token)?;
            Ok(Box::new(backend))
        }
        CloudProvider::GoogleDrive => {
            let backend = google_drive::GoogleDriveBackend::new(access_token).await?;
            Ok(Box::new(backend))
        }
    }
}

/// Convert CloudFileInfo to scanner::FileInfo for processing
impl CloudFileInfo {
    pub fn to_file_info(&self) -> crate::scanner::FileInfo {
        crate::scanner::FileInfo {
            original_path: PathBuf::from(&self.path),
            original_name: self.name.clone(),
            extension: self.extension.clone(),
            size: self.size,
            modified_time: self.modified_time,
            is_failed_download: self.is_failed_download,
            is_too_small: self.is_too_small,
            new_name: None,
            new_path: PathBuf::from(&self.path),
        }
    }
}
