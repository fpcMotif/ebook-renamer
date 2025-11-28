use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, info};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{CloudFileInfo, CloudStorage};

/// Google Drive API client
pub struct GoogleDriveBackend {
    access_token: String,
    client: Client,
}

// Google Drive API structures
#[derive(Debug, Deserialize)]
struct FileListResponse {
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    size: Option<String>,
    #[serde(rename = "modifiedTime")]
    modified_time: String,
    #[serde(rename = "md5Checksum")]
    md5_checksum: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateFileRequest {
    name: String,
}

impl GoogleDriveBackend {
    pub async fn new(access_token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            access_token,
            client,
        })
    }

    fn parse_drive_time(time_str: &str) -> Result<SystemTime> {
        // Google Drive uses RFC 3339 format: "2024-01-15T12:34:56.000Z"
        let dt = chrono::DateTime::parse_from_rfc3339(time_str)?;
        let timestamp = dt.timestamp() as u64;
        Ok(UNIX_EPOCH + Duration::from_secs(timestamp))
    }

    fn get_extension(filename: &str) -> String {
        if filename.ends_with(".tar.gz") {
            ".tar.gz".to_string()
        } else if filename.ends_with(".download") {
            ".download".to_string()
        } else if filename.ends_with(".crdownload") {
            ".crdownload".to_string()
        } else {
            std::path::Path::new(filename)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default()
        }
    }

    fn is_ebook_extension(ext: &str) -> bool {
        matches!(ext, ".pdf" | ".epub" | ".txt" | ".mobi" | ".djvu" | ".download" | ".crdownload")
    }

    fn is_folder_mime_type(mime_type: &str) -> bool {
        mime_type == "application/vnd.google-apps.folder"
    }

    async fn get_folder_id(&self, folder_path: &str) -> Result<String> {
        // Special case: root folder
        if folder_path.is_empty() || folder_path == "/" {
            return Ok("root".to_string());
        }

        // Split path and traverse to find folder ID
        let parts: Vec<&str> = folder_path.trim_matches('/').split('/').collect();
        let mut current_parent = "root".to_string();

        for part in parts {
            let query = format!("name='{}' and '{}' in parents and mimeType='application/vnd.google-apps.folder' and trashed=false", part, current_parent);
            let url = format!(
                "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id)",
                urlencoding::encode(&query)
            );

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await?;

            if response.status() != StatusCode::OK {
                let error_text = response.text().await?;
                return Err(anyhow!("Google Drive API error: {}", error_text));
            }

            let list: FileListResponse = response.json().await?;
            if list.files.is_empty() {
                return Err(anyhow!("Folder not found: {}", part));
            }

            current_parent = list.files[0].id.clone();
        }

        Ok(current_parent)
    }
}

#[async_trait]
impl CloudStorage for GoogleDriveBackend {
    async fn list_files(&self, path: &str, _max_depth: Option<usize>) -> Result<Vec<CloudFileInfo>> {
        // Get folder ID from path
        let folder_id = self.get_folder_id(path).await?;

        // Build query to list all files recursively
        let query = format!(
            "'{}' in parents and trashed=false",
            folder_id
        );

        let mut all_files = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!(
                "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType,size,modifiedTime,md5Checksum),nextPageToken&pageSize=1000",
                urlencoding::encode(&query)
            );

            if let Some(token) = &page_token {
                url.push_str(&format!("&pageToken={}", token));
            }

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await?;

            if response.status() != StatusCode::OK {
                let error_text = response.text().await?;
                return Err(anyhow!("Google Drive API error: {}", error_text));
            }

            let list: FileListResponse = response.json().await?;

            for file in list.files {
                // Skip folders and non-ebook files
                if Self::is_folder_mime_type(&file.mime_type) {
                    continue;
                }

                let extension = Self::get_extension(&file.name);
                if !Self::is_ebook_extension(&extension) {
                    continue;
                }

                let size = file.size.as_ref()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let is_failed_download = file.name.ends_with(".download") || file.name.ends_with(".crdownload");
                let is_ebook = extension == ".pdf" || extension == ".epub" || extension == ".djvu";
                let is_too_small = !is_failed_download && is_ebook && size < 1024;

                let modified_time = Self::parse_drive_time(&file.modified_time)?;

                // Use file ID as path for Google Drive (we'll store a mapping)
                let path_display = format!("{}/{}", path, file.name);

                all_files.push(CloudFileInfo {
                    path: path_display,
                    name: file.name,
                    extension,
                    size,
                    modified_time,
                    is_failed_download,
                    is_too_small,
                });
            }

            page_token = list.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        info!("Found {} files in Google Drive path: {}", all_files.len(), path);
        Ok(all_files)
    }

    async fn rename_file(&self, from_path: &str, to_path: &str) -> Result<()> {
        // Extract file ID from path (we'd need a mapping in practice)
        // For now, assume from_path contains the file ID
        let file_id = from_path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid path"))?;

        let new_name = to_path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid destination path"))?;

        let url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);

        let request = UpdateFileRequest {
            name: new_name.to_string(),
        };

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Google Drive rename error: {}", error_text));
        }

        debug!("Renamed {} -> {}", from_path, to_path);
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<()> {
        // Extract file ID from path
        let file_id = path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid path"))?;

        let url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if response.status() != StatusCode::NO_CONTENT && response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Google Drive delete error: {}", error_text));
        }

        debug!("Deleted {}", path);
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let file_id = path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid path"))?;

        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id",
            file_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }

    async fn get_metadata(&self, path: &str) -> Result<CloudFileInfo> {
        let file_id = path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid path"))?;

        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size,modifiedTime,md5Checksum",
            file_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Google Drive metadata error: {}", error_text));
        }

        let file: DriveFile = response.json().await?;

        let extension = Self::get_extension(&file.name);
        let size = file.size.as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let is_failed_download = file.name.ends_with(".download") || file.name.ends_with(".crdownload");
        let is_ebook = extension == ".pdf" || extension == ".epub" || extension == ".djvu";
        let is_too_small = !is_failed_download && is_ebook && size < 1024;

        let modified_time = Self::parse_drive_time(&file.modified_time)?;

        Ok(CloudFileInfo {
            path: path.to_string(),
            name: file.name,
            extension,
            size,
            modified_time,
            is_failed_download,
            is_too_small,
        })
    }

    async fn get_md5_hash(&self, path: &str) -> Result<Option<String>> {
        let file_id = path.split('/').last()
            .ok_or_else(|| anyhow!("Invalid path"))?;

        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=md5Checksum",
            file_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            return Ok(None);
        }

        let file: DriveFile = response.json().await?;
        Ok(file.md5_checksum)
    }
}
