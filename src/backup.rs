use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use log::info;

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new(backup_dir: &Path) -> Result<Self> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let dir = backup_dir.join(format!("backup_{}", timestamp));
        fs::create_dir_all(&dir)?;
        
        Ok(BackupManager {
            backup_dir: dir,
        })
    }
    
    pub fn backup_file(&mut self, original_path: &Path) -> Result<PathBuf> {
        let file_name = original_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("No filename"))?;
        
        let backup_path = self.backup_dir.join(file_name);
        
        let final_path = if backup_path.exists() {
            let stem = backup_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = backup_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let mut counter = 1;
            loop {
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, counter)
                } else {
                    format!("{}_{}.{}", stem, counter, ext)
                };
                let new_path = self.backup_dir.join(new_name);
                if !new_path.exists() {
                    break new_path;
                }
                counter += 1;
            }
        } else {
            backup_path
        };
        
        fs::copy(original_path, &final_path)?;
        info!("Backed up {} to {}", original_path.display(), final_path.display());
        
        Ok(final_path)
    }
    
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}
