use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, info};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{CloudFileInfo, CloudStorage};

/// Dropbox API client
pub struct DropboxBackend {
    access_token: String,
    client: Client,
}

// Dropbox API structures
#[derive(Debug, Serialize)]
struct ListFolderRequest {
    path: String,
    recursive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListFolderResponse {
    entries: Vec<DropboxEntry>,
    has_more: bool,
    cursor: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = ".tag")]
enum DropboxEntry {
    #[serde(rename = "file")]
    File {
        name: String,
        path_display: String,
        size: u64,
        client_modified: String,
        content_hash: Option<String>,
    },
    #[serde(rename = "folder")]
    Folder {
        name: String,
        path_display: String,
    },
}

#[derive(Debug, Serialize)]
struct MoveRequest {
    from_path: String,
    to_path: String,
    autorename: bool,
}

#[derive(Debug, Serialize)]
struct DeleteRequest {
    path: String,
}

impl DropboxBackend {
    pub fn new(access_token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            access_token,
            client,
        })
    }

    fn parse_dropbox_time(time_str: &str) -> Result<SystemTime> {
        // Dropbox uses ISO 8601 format: "2024-01-15T12:34:56Z"
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
}

#[async_trait]
impl CloudStorage for DropboxBackend {
    async fn list_files(&self, path: &str, _max_depth: Option<usize>) -> Result<Vec<CloudFileInfo>> {
        let url = "https://api.dropboxapi.com/2/files/list_folder";

        let request = ListFolderRequest {
            path: if path.is_empty() { "".to_string() } else { path.to_string() },
            recursive: true,
            limit: None,
        };

        let mut all_files = Vec::new();
        let mut has_more = true;
        let mut cursor = String::new();

        while has_more {
            let response = if cursor.is_empty() {
                // Initial request
                self.client
                    .post(url)
                    .header("Authorization", format!("Bearer {}", self.access_token))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await?
            } else {
                // Continuation request
                #[derive(Serialize)]
                struct ContinueRequest {
                    cursor: String,
                }
                self.client
                    .post("https://api.dropboxapi.com/2/files/list_folder/continue")
                    .header("Authorization", format!("Bearer {}", self.access_token))
                    .header("Content-Type", "application/json")
                    .json(&ContinueRequest { cursor: cursor.clone() })
                    .send()
                    .await?
            };

            if response.status() != StatusCode::OK {
                let error_text = response.text().await?;
                return Err(anyhow!("Dropbox API error: {}", error_text));
            }

            let list_response: ListFolderResponse = response.json().await?;

            for entry in list_response.entries {
                if let DropboxEntry::File {
                    name,
                    path_display,
                    size,
                    client_modified,
                    ..
                } = entry
                {
                    let extension = Self::get_extension(&name);

                    // Only include ebook files
                    if !Self::is_ebook_extension(&extension) {
                        continue;
                    }

                    let is_failed_download = name.ends_with(".download") || name.ends_with(".crdownload");
                    let is_ebook = extension == ".pdf" || extension == ".epub" || extension == ".djvu";
                    let is_too_small = !is_failed_download && is_ebook && size < 1024;

                    let modified_time = Self::parse_dropbox_time(&client_modified)?;

                    all_files.push(CloudFileInfo {
                        path: path_display,
                        name,
                        extension,
                        size,
                        modified_time,
                        is_failed_download,
                        is_too_small,
                    });
                }
            }

            has_more = list_response.has_more;
            cursor = list_response.cursor;
        }

        info!("Found {} files in Dropbox path: {}", all_files.len(), path);
        Ok(all_files)
    }

    async fn rename_file(&self, from_path: &str, to_path: &str) -> Result<()> {
        let url = "https://api.dropboxapi.com/2/files/move_v2";

        let request = MoveRequest {
            from_path: from_path.to_string(),
            to_path: to_path.to_string(),
            autorename: false,
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Dropbox rename error: {}", error_text));
        }

        debug!("Renamed {} -> {}", from_path, to_path);
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<()> {
        let url = "https://api.dropboxapi.com/2/files/delete_v2";

        let request = DeleteRequest {
            path: path.to_string(),
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Dropbox delete error: {}", error_text));
        }

        debug!("Deleted {}", path);
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let url = "https://api.dropboxapi.com/2/files/get_metadata";

        #[derive(Serialize)]
        struct MetadataRequest {
            path: String,
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&MetadataRequest {
                path: path.to_string(),
            })
            .send()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }

    async fn get_metadata(&self, path: &str) -> Result<CloudFileInfo> {
        let url = "https://api.dropboxapi.com/2/files/get_metadata";

        #[derive(Serialize)]
        struct MetadataRequest {
            path: String,
        }

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&MetadataRequest {
                path: path.to_string(),
            })
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow!("Dropbox metadata error: {}", error_text));
        }

        let entry: DropboxEntry = response.json().await?;

        match entry {
            DropboxEntry::File {
                name,
                path_display,
                size,
                client_modified,
                ..
            } => {
                let extension = Self::get_extension(&name);
                let is_failed_download = name.ends_with(".download") || name.ends_with(".crdownload");
                let is_ebook = extension == ".pdf" || extension == ".epub" || extension == ".djvu";
                let is_too_small = !is_failed_download && is_ebook && size < 1024;
                let modified_time = Self::parse_dropbox_time(&client_modified)?;

                Ok(CloudFileInfo {
                    path: path_display,
                    name,
                    extension,
                    size,
                    modified_time,
                    is_failed_download,
                    is_too_small,
                })
            }
            DropboxEntry::Folder { .. } => Err(anyhow!("Path is a folder, not a file")),
        }
    }

    async fn get_md5_hash(&self, _path: &str) -> Result<Option<String>> {
        // Dropbox uses a proprietary content_hash (not MD5)
        // We'd need to download the file to compute MD5, which defeats the purpose
        // Return None to indicate hash is not available without download
        Ok(None)
    }
}
