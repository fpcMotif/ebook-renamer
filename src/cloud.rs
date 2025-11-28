use std::path::Path;
use log::debug;

/// Detects if a path is within a cloud storage directory
pub fn is_cloud_storage_path(path: &Path) -> Option<CloudProvider> {
    let path_str = path.to_str()?;

    // Check for common cloud storage paths
    if path_str.contains("Dropbox") {
        debug!("Detected Dropbox path: {}", path_str);
        return Some(CloudProvider::Dropbox);
    }

    if path_str.contains("Google Drive") || path_str.contains("GoogleDrive") {
        debug!("Detected Google Drive path: {}", path_str);
        return Some(CloudProvider::GoogleDrive);
    }

    if path_str.contains("OneDrive") {
        debug!("Detected OneDrive path: {}", path_str);
        return Some(CloudProvider::OneDrive);
    }

    // macOS CloudStorage paths
    if path_str.contains("Library/CloudStorage/Dropbox") {
        debug!("Detected macOS CloudStorage Dropbox path: {}", path_str);
        return Some(CloudProvider::Dropbox);
    }

    if path_str.contains("Library/CloudStorage/GoogleDrive") {
        debug!("Detected macOS CloudStorage Google Drive path: {}", path_str);
        return Some(CloudProvider::GoogleDrive);
    }

    if path_str.contains("Library/CloudStorage/OneDrive") {
        debug!("Detected macOS CloudStorage OneDrive path: {}", path_str);
        return Some(CloudProvider::OneDrive);
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Dropbox,
    GoogleDrive,
    OneDrive,
}

impl CloudProvider {
    pub fn name(&self) -> &'static str {
        match self {
            CloudProvider::Dropbox => "Dropbox",
            CloudProvider::GoogleDrive => "Google Drive",
            CloudProvider::OneDrive => "OneDrive",
        }
    }
}

pub fn cloud_mode_warning(provider: CloudProvider) -> String {
    format!(
        "⚠️  Detected {} storage. Using metadata-only mode to avoid downloading files.\n\
         Duplicate detection based on filename similarity (≥85%) + exact size match.\n\
         This is less accurate than content-based hashing. Review carefully!",
        provider.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_dropbox() {
        let path = PathBuf::from("/Users/user/Dropbox/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::Dropbox));
    }

    #[test]
    fn test_detect_macos_dropbox() {
        let path = PathBuf::from("/Users/user/Library/CloudStorage/Dropbox/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::Dropbox));
    }

    #[test]
    fn test_detect_google_drive() {
        let path = PathBuf::from("/Users/user/Google Drive/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::GoogleDrive));
    }

    #[test]
    fn test_detect_macos_google_drive() {
        let path = PathBuf::from("/Users/user/Library/CloudStorage/GoogleDrive/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::GoogleDrive));
    }

    #[test]
    fn test_not_cloud_storage() {
        let path = PathBuf::from("/Users/user/Documents/Books");
        assert_eq!(is_cloud_storage_path(&path), None);
    }
}
