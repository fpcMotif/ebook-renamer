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
    pub deleted_corrupted_files: Vec<PathBuf>,
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
            deleted_corrupted_files: Vec::new(),
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
            "Download recovery completed: {} files extracted, {} folders cleaned, {} corrupted files deleted, {} errors",
            result.extracted_files.len(),
            result.cleaned_folders.len(),
            result.deleted_corrupted_files.len(),
            result.errors.len()
        );

        Ok(result)
    }

    fn process_download_folder(&self, download_folder: &Path, result: &mut RecoveryResult) -> Result<()> {
        // Find all files inside the download folder
        let mut pdf_files = Vec::new();
        let mut other_files = Vec::new();
        
        for entry in fs::read_dir(download_folder)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let metadata = fs::metadata(&path).ok();
                let size = metadata.map(|m| m.len()).unwrap_or(0);
                
                if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                    if extension.to_lowercase() == "pdf" {
                        // Check if PDF is valid and not too small
                        if size < 1024 {
                            // File too small, mark for deletion
                            debug!("Found corrupted PDF (too small): {:?}", path);
                            if let Err(e) = fs::remove_file(&path) {
                                debug!("Failed to delete corrupted file {:?}: {}", path, e);
                            } else {
                                info!("Deleted corrupted PDF (too small): {:?}", path.file_name().unwrap());
                                result.deleted_corrupted_files.push(path.clone());
                            }
                        } else if let Err(_) = self.validate_pdf_header(&path) {
                            // Invalid PDF header, mark for deletion
                            debug!("Found corrupted PDF (invalid header): {:?}", path);
                            if let Err(e) = fs::remove_file(&path) {
                                debug!("Failed to delete corrupted file {:?}: {}", path, e);
                            } else {
                                info!("Deleted corrupted PDF (invalid header): {:?}", path.file_name().unwrap());
                                result.deleted_corrupted_files.push(path.clone());
                            }
                        } else {
                            pdf_files.push(path);
                        }
                    } else {
                        // Non-PDF files - mark for deletion if they're suspiciously small
                        if size < 100 {
                            debug!("Found suspiciously small file: {:?}", path);
                            if let Err(e) = fs::remove_file(&path) {
                                debug!("Failed to delete suspicious file {:?}: {}", path, e);
                            } else {
                                info!("Deleted suspicious file: {:?}", path.file_name().unwrap());
                                result.deleted_corrupted_files.push(path.clone());
                            }
                        } else {
                            other_files.push(path);
                        }
                    }
                } else {
                    // Files without extension - mark for deletion if suspiciously small
                    if size < 100 {
                        debug!("Found suspiciously small file without extension: {:?}", path);
                        if let Err(e) = fs::remove_file(&path) {
                            debug!("Failed to delete suspicious file {:?}: {}", path, e);
                        } else {
                            info!("Deleted suspicious file: {:?}", path.file_name().unwrap());
                            result.deleted_corrupted_files.push(path.clone());
                        }
                    } else {
                        other_files.push(path);
                    }
                }
            }
        }

        // Extract valid PDF files
        for pdf_file in pdf_files {
            let new_name = self.clean_filename(pdf_file.file_name().unwrap().to_str().unwrap());
            let new_path = self.target_dir.join(&new_name);
            
            // Move PDF to target directory
            fs::rename(&pdf_file, &new_path)?;
            info!("Extracted PDF: {:?} -> {:?}", pdf_file.file_name().unwrap(), new_name);
            result.extracted_files.push(new_path);
        }

        // Clean up empty download folder if auto_cleanup is enabled
        // Also clean up if folder only contains non-PDF files (which we've already handled)
        if self.auto_cleanup {
            // Check if folder is empty (all files have been extracted or deleted)
            let remaining_files: Vec<_> = fs::read_dir(download_folder)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect();
            
            if remaining_files.is_empty() {
                // Try to remove the folder - it should be empty now
                match fs::remove_dir(download_folder) {
                    Ok(_) => {
                        info!("Removed empty download folder: {:?}", download_folder);
                        result.cleaned_folders.push(download_folder.to_path_buf());
                    }
                    Err(e) => {
                        debug!("Failed to remove download folder {:?}: {}", download_folder, e);
                    }
                }
            } else {
                debug!("Download folder {:?} still contains {} files, not removing", 
                    download_folder, remaining_files.len());
            }
        }

        Ok(())
    }

    fn validate_pdf_header(&self, path: &Path) -> Result<()> {
        use std::io::Read;
        
        let mut file = fs::File::open(path)?;
        let mut header = [0u8; 5];
        
        // Try to read header, if file is too small, it's corrupted
        match file.read_exact(&mut header) {
            Ok(_) => {
                // PDF files should start with "%PDF-"
                if &header != b"%PDF-" {
                    return Err(anyhow::anyhow!("Invalid PDF header"));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("File too small to be valid PDF: {}", e));
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
        assert!(result.deleted_corrupted_files.is_empty());
        assert!(result.errors.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_recover_downloads_with_folder() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        
        // Create a .download folder with a valid PDF inside
        let download_folder = tmp_dir.path().join("test.pdf.download");
        fs::create_dir(&download_folder)?;
        
        // Create a valid PDF file (minimal PDF with correct header, > 1KB to avoid deletion)
        let mut pdf_content = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 1\ntrailer\n<< /Size 1 /Root 1 0 R >>\nstartxref\n100\n%%EOF".to_vec();
        // Pad to ensure it's > 1KB
        pdf_content.extend(vec![0u8; 1500 - pdf_content.len()]);
        let pdf_inside = download_folder.join("Test Book (Z-Library).pdf");
        fs::write(&pdf_inside, &pdf_content)?;
        
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

    #[test]
    fn test_recover_downloads_no_auto_cleanup() -> Result<()> {
        let tmp_dir = TempDir::new()?;

        // Create a .download folder with a valid PDF inside
        let download_folder = tmp_dir.path().join("test.pdf.download");
        fs::create_dir(&download_folder)?;

        // Create a valid PDF file (minimal PDF with correct header, > 1KB to avoid deletion)
        let mut pdf_content = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 1\ntrailer\n<< /Size 1 /Root 1 0 R >>\nstartxref\n100\n%%EOF".to_vec();
        // Pad to ensure it's > 1KB
        pdf_content.extend(vec![0u8; 1500 - pdf_content.len()]);
        let pdf_inside = download_folder.join("Test Book (Z-Library).pdf");
        fs::write(&pdf_inside, &pdf_content)?;

        let recovery = DownloadRecovery::new(tmp_dir.path(), false); // auto_cleanup = false
        let result = recovery.recover_downloads()?;

        assert_eq!(result.extracted_files.len(), 1);
        assert!(result.cleaned_folders.is_empty());

        // Verify folder still exists
        assert!(download_folder.exists());

        Ok(())
    }

    #[test]
    fn test_recover_downloads_with_crdownload() -> Result<()> {
        let tmp_dir = TempDir::new()?;

        // Create a .crdownload folder with a valid PDF inside
        let download_folder = tmp_dir.path().join("test.pdf.crdownload");
        fs::create_dir(&download_folder)?;

        // Create a valid PDF file (minimal PDF with correct header, > 1KB to avoid deletion)
        let mut pdf_content = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 1\ntrailer\n<< /Size 1 /Root 1 0 R >>\nstartxref\n100\n%%EOF".to_vec();
        // Pad to ensure it's > 1KB
        pdf_content.extend(vec![0u8; 1500 - pdf_content.len()]);
        let pdf_inside = download_folder.join("Test Book.pdf");
        fs::write(&pdf_inside, &pdf_content)?;

        let recovery = DownloadRecovery::new(tmp_dir.path(), true);
        let result = recovery.recover_downloads()?;

        assert_eq!(result.extracted_files.len(), 1);
        assert_eq!(result.cleaned_folders.len(), 1);

        Ok(())
    }
}
