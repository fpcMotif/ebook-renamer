use anyhow::Result;
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};

pub struct DownloadRecovery {
    target_dir: PathBuf,
    auto_cleanup: bool,
}

#[derive(Debug)]
pub struct RecoveryResult {
    pub extracted_files: Vec<PathBuf>,
    pub cleaned_folders: Vec<PathBuf>,
    pub errors: Vec<String>,
}

impl DownloadRecovery {
    pub fn new(target_dir: &Path, auto_cleanup: bool) -> Self {
        Self {
            target_dir: target_dir.to_path_buf(),
            auto_cleanup,
        }
    }

    pub fn recover_downloads(&self) -> Result<RecoveryResult> {
        let mut result = RecoveryResult {
            extracted_files: Vec::new(),
            cleaned_folders: Vec::new(),
            errors: Vec::new(),
        };

        info!("Scanning for download folders in {:?}", self.target_dir);
        
        // Find all .download and .crdownload directories
        for entry in fs::read_dir(&self.target_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".download") || filename.ends_with(".crdownload") {
                        debug!("Processing download folder: {:?}", path);
                        match self.process_download_folder(&path, &mut result) {
                            Ok(_) => info!("Successfully processed: {:?}", filename),
                            Err(e) => {
                                let error_msg = format!("Failed to process {:?}: {}", path, e);
                                debug!("{}", error_msg);
                                result.errors.push(error_msg);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Download recovery completed: {} files extracted, {} folders cleaned, {} errors",
            result.extracted_files.len(),
            result.cleaned_folders.len(),
            result.errors.len()
        );

        Ok(result)
    }

    fn process_download_folder(&self, download_folder: &Path, result: &mut RecoveryResult) -> Result<()> {
        // Find PDF files inside the download folder
        let mut pdf_files = Vec::new();
        
        for entry in fs::read_dir(download_folder)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                    if extension.to_lowercase() == "pdf" {
                        pdf_files.push(path);
                    }
                }
            }
        }

        if pdf_files.is_empty() {
            debug!("No PDF files found in download folder: {:?}", download_folder);
            return Ok(());
        }

        // Extract each PDF file
        for pdf_file in pdf_files {
            let new_name = self.clean_filename(pdf_file.file_name().unwrap().to_str().unwrap());
            let new_path = self.target_dir.join(&new_name);
            
            // Move PDF to target directory
            fs::rename(&pdf_file, &new_path)?;
            info!("Extracted PDF: {:?} -> {:?}", pdf_file.file_name().unwrap(), new_name);
            result.extracted_files.push(new_path);
        }

        // Clean up empty download folder if auto_cleanup is enabled
        if self.auto_cleanup {
            match fs::remove_dir(download_folder) {
                Ok(_) => {
                    info!("Removed empty download folder: {:?}", download_folder);
                    result.cleaned_folders.push(download_folder.to_path_buf());
                }
                Err(e) => {
                    debug!("Failed to remove download folder {:?}: {}", download_folder, e);
                }
            }
        }

        Ok(())
    }

    fn clean_filename(&self, original: &str) -> String {
        // Remove common suffixes like " (Z-Library)", " (Anna's Archive)", etc.
        let mut cleaned = original.to_string();
        
        // Remove .pdf extension temporarily
        let has_pdf = cleaned.to_lowercase().ends_with(".pdf");
        if has_pdf {
            cleaned = cleaned[..cleaned.len() - 4].to_string();
        }
        
        let suffixes_to_remove = [
            " (Z-Library)",
            " (z-Library)",
            " (Anna's Archive)",
            " (libgen.li)",
            " (libgen.lc)",
            " (Library Genesis)",
        ];
        
        for suffix in &suffixes_to_remove {
            if cleaned.ends_with(suffix) {
                cleaned = cleaned[..cleaned.len() - suffix.len()].to_string();
                break;
            }
        }
        
        // Ensure it ends with .pdf
        if !cleaned.to_lowercase().ends_with(".pdf") {
            cleaned.push_str(".pdf");
        }
        
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_filename() {
        let recovery = DownloadRecovery::new(Path::new("/tmp"), false);
        
        assert_eq!(
            recovery.clean_filename("Test Book (Z-Library).pdf"),
            "Test Book.pdf"
        );
        
        assert_eq!(
            recovery.clean_filename("Math Book (Anna's Archive).pdf"),
            "Math Book.pdf"
        );
        
        assert_eq!(
            recovery.clean_filename("Science Book.pdf"),
            "Science Book.pdf"
        );
        
        assert_eq!(
            recovery.clean_filename("No Extension (Z-Library)"),
            "No Extension.pdf"
        );
    }

    #[test]
    fn test_recover_downloads_empty_dir() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let recovery = DownloadRecovery::new(tmp_dir.path(), true);
        let result = recovery.recover_downloads()?;
        
        assert!(result.extracted_files.is_empty());
        assert!(result.cleaned_folders.is_empty());
        assert!(result.errors.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_recover_downloads_with_folder() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        
        // Create a .download folder with a PDF inside
        let download_folder = tmp_dir.path().join("test.pdf.download");
        fs::create_dir(&download_folder)?;
        
        let pdf_inside = download_folder.join("Test Book (Z-Library).pdf");
        fs::write(&pdf_inside, "dummy pdf content")?;
        
        let recovery = DownloadRecovery::new(tmp_dir.path(), true);
        let result = recovery.recover_downloads()?;
        
        assert_eq!(result.extracted_files.len(), 1);
        assert!(result.extracted_files[0].file_name().unwrap() == "Test Book.pdf");
        assert_eq!(result.cleaned_folders.len(), 1);
        assert!(result.errors.is_empty());
        
        // Verify the extracted file exists
        assert!(tmp_dir.path().join("Test Book.pdf").exists());
        assert!(!download_folder.exists());
        
        Ok(())
    }
}
