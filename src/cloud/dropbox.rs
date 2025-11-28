use anyhow::{Result, anyhow};
use super::{CloudFile, CloudProvider};
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value;

pub struct DropboxProvider {
    access_token: String,
    client: reqwest::blocking::Client,
}

impl DropboxProvider {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn request(&self, endpoint: &str, body: &Value) -> Result<Value> {
        let res = self.client.post(format!("https://api.dropboxapi.com/2/files/{}", endpoint))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()?;

        if !res.status().is_success() {
            return Err(anyhow!("Dropbox API error: {}", res.text()?));
        }

        let json: Value = res.json()?;
        Ok(json)
    }
}

impl CloudProvider for DropboxProvider {
    fn name(&self) -> &str {
        "dropbox"
    }

    fn list_files(&self, path: &str) -> Result<Vec<CloudFile>> {
        let mut files = Vec::new();
        let mut has_more = true;
        let mut cursor = None;

        let path = if path == "." || path == "/" { "" } else { path };

        while has_more {
            let body = if let Some(ref c) = cursor {
                 serde_json::json!({ "cursor": c })
            } else {
                 serde_json::json!({
                    "path": path,
                    "recursive": true,
                    "include_media_info": false,
                    "include_deleted": false,
                    "include_has_explicit_shared_members": false
                })
            };

            let endpoint = if cursor.is_some() { "list_folder/continue" } else { "list_folder" };
            let json = self.request(endpoint, &body)?;

            if let Some(entries) = json["entries"].as_array() {
                for entry in entries {
                    if entry[".tag"] == "file" {
                        let name = entry["name"].as_str().unwrap_or_default().to_string();
                        let path_display = entry["path_display"].as_str().unwrap_or_default().to_string();
                        let id = entry["id"].as_str().unwrap_or_default().to_string();
                        let size = entry["size"].as_u64().unwrap_or(0);
                        let hash = entry["content_hash"].as_str().map(|s| s.to_string());

                        // Parse client_modified
                        let modified_str = entry["client_modified"].as_str().unwrap_or("");
                        let modified_time = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", modified_str))
                            .map(|dt| SystemTime::from(dt))
                            .unwrap_or(SystemTime::now());

                        files.push(CloudFile {
                            id,
                            name,
                            path: path_display,
                            hash,
                            size,
                            modified_time,
                            provider: "dropbox".to_string(),
                        });
                    }
                }
            }

            has_more = json["has_more"].as_bool().unwrap_or(false);
            cursor = json["cursor"].as_str().map(|s| s.to_string());
        }

        Ok(files)
    }

    fn rename_file(&self, file: &CloudFile, new_name: &str) -> Result<()> {
        // Calculate new path
        let parent = std::path::Path::new(&file.path).parent().unwrap_or(std::path::Path::new(""));
        let new_path = parent.join(new_name).to_str().unwrap().to_string();

        // Dropbox paths must start with /
        let new_path = if !new_path.starts_with('/') {
            format!("/{}", new_path)
        } else {
            new_path
        };

        let body = serde_json::json!({
            "from_path": file.path,
            "to_path": new_path,
            "autorename": false
        });

        self.request("move_v2", &body)?;
        Ok(())
    }

    fn delete_file(&self, file: &CloudFile) -> Result<()> {
        let body = serde_json::json!({
            "path": file.path
        });
        self.request("delete_v2", &body)?;
        Ok(())
    }
}
