use anyhow::Result;
use std::time::SystemTime;
use crate::scanner::FileInfo;

#[derive(Debug, Clone)]
pub struct CloudFile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[allow(dead_code)]
    pub hash: Option<String>,
    pub size: u64,
    pub modified_time: SystemTime,
    #[allow(dead_code)]
    pub provider: String,
}

#[allow(dead_code)]
impl CloudFile {
    pub fn to_file_info(&self) -> FileInfo {
        let extension = std::path::Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let is_failed_download = self.name.ends_with(".download") || self.name.ends_with(".crdownload");
        let is_ebook = extension == ".pdf" || extension == ".epub";
        let is_too_small = !is_failed_download && is_ebook && self.size < 1024;

        FileInfo {
            original_path: std::path::PathBuf::from(&self.path),
            original_name: self.name.clone(),
            extension,
            size: self.size,
            modified_time: self.modified_time,
            is_failed_download,
            is_too_small,
            new_name: None,
            new_path: std::path::PathBuf::from(&self.path),
        }
    }
}

pub trait CloudProvider {
    fn list_files(&self, path: &str) -> Result<Vec<CloudFile>>;
    fn rename_file(&self, file: &CloudFile, new_name: &str) -> Result<()>;
    #[allow(dead_code)]
    fn delete_file(&self, file: &CloudFile) -> Result<()>;
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

pub mod dropbox;
pub mod gdrive;
