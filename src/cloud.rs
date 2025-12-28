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

    // iCloud Drive detection
    if path_str.contains("iCloud Drive") || path_str.contains("Mobile Documents") {
        debug!("Detected iCloud Drive path: {}", path_str);
        return Some(CloudProvider::ICloudDrive);
    }

    if path_str.contains("Library/Mobile Documents/com~apple~CloudDocs") {
        debug!("Detected macOS iCloud Drive path: {}", path_str);
        return Some(CloudProvider::ICloudDrive);
    }

    // Nextcloud detection
    if path_str.contains("Nextcloud") {
        debug!("Detected Nextcloud path: {}", path_str);
        return Some(CloudProvider::Nextcloud);
    }

    // MEGA detection
    if path_str.contains("MEGA") || path_str.contains("MEGAsync") {
        debug!("Detected MEGA path: {}", path_str);
        return Some(CloudProvider::Mega);
    }

    // pCloud detection
    if path_str.contains("pCloud") || path_str.contains("pCloudDrive") {
        debug!("Detected pCloud path: {}", path_str);
        return Some(CloudProvider::PCloud);
    }

    // Box detection
    if path_str.contains("/Box/") || path_str.contains("Box Sync") {
        debug!("Detected Box path: {}", path_str);
        return Some(CloudProvider::Box);
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Dropbox,
    GoogleDrive,
    OneDrive,
    ICloudDrive,
    Nextcloud,
    Mega,
    PCloud,
    Box,
}

impl CloudProvider {
    pub fn name(&self) -> &'static str {
        match self {
            CloudProvider::Dropbox => "Dropbox",
            CloudProvider::GoogleDrive => "Google Drive",
            CloudProvider::OneDrive => "OneDrive",
            CloudProvider::ICloudDrive => "iCloud Drive",
            CloudProvider::Nextcloud => "Nextcloud",
            CloudProvider::Mega => "MEGA",
            CloudProvider::PCloud => "pCloud",
            CloudProvider::Box => "Box",
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

    #[test]
    fn test_detect_icloud_drive() {
        let path = PathBuf::from("/Users/user/Library/Mobile Documents/com~apple~CloudDocs/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::ICloudDrive));

        let path2 = PathBuf::from("/Users/user/iCloud Drive/Books");
        assert_eq!(is_cloud_storage_path(&path2), Some(CloudProvider::ICloudDrive));
    }

    #[test]
    fn test_detect_nextcloud() {
        let path = PathBuf::from("/Users/user/Nextcloud/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::Nextcloud));
    }

    #[test]
    fn test_detect_mega() {
        let path = PathBuf::from("/Users/user/MEGA/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::Mega));

        let path2 = PathBuf::from("/Users/user/MEGAsync/Books");
        assert_eq!(is_cloud_storage_path(&path2), Some(CloudProvider::Mega));
    }

    #[test]
    fn test_detect_pcloud() {
        let path = PathBuf::from("/Users/user/pCloud Drive/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::PCloud));
    }

    #[test]
    fn test_detect_box() {
        let path = PathBuf::from("/Users/user/Box/Books");
        assert_eq!(is_cloud_storage_path(&path), Some(CloudProvider::Box));

        let path2 = PathBuf::from("/Users/user/Box Sync/Books");
        assert_eq!(is_cloud_storage_path(&path2), Some(CloudProvider::Box));
    }
}
