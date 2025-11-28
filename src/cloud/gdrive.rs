use anyhow::{Result, anyhow};
use super::{CloudFile, CloudProvider};
use std::time::SystemTime;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

pub struct GDriveProvider {
    access_token: String,
    client: reqwest::blocking::Client,
}

impl GDriveProvider {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            client: reqwest::blocking::Client::new(),
        }
    }

    // Helper to search files by name/parent
    // Google Drive uses ID-based system, so "path" is ambiguous.
    // For simplicity, we assume "path" is a folder ID or we search from root.
    // However, to mimic file system, we would need to traverse.
    // For this implementation, we will treat the input "path" as a Folder ID or "root".
}

impl CloudProvider for GDriveProvider {
    fn name(&self) -> &str {
        "gdrive"
    }

    fn list_files(&self, folder_id: &str) -> Result<Vec<CloudFile>> {
        let folder_id = if folder_id == "." || folder_id == "/" { "root" } else { folder_id };
        let mut files = Vec::new();
        let mut page_token = None;

        loop {
            let mut url = format!(
                "https://www.googleapis.com/drive/v3/files?q='{}' in parents and trashed = false&fields=nextPageToken,files(id,name,size,md5Checksum,modifiedTime)&pageSize=1000",
                folder_id
            );

            if let Some(ref token) = page_token {
                url.push_str(&format!("&pageToken={}", token));
            }

            let res = self.client.get(&url)
                .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
                .send()?;

            if !res.status().is_success() {
                return Err(anyhow!("Google Drive API error: {}", res.text()?));
            }

            let json: serde_json::Value = res.json()?;

            if let Some(items) = json["files"].as_array() {
                for item in items {
                    let id = item["id"].as_str().unwrap_or_default().to_string();
                    let name = item["name"].as_str().unwrap_or_default().to_string();
                    let size = item["size"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                    let hash = item["md5Checksum"].as_str().map(|s| s.to_string());

                    let modified_str = item["modifiedTime"].as_str().unwrap_or("");
                     let modified_time = chrono::DateTime::parse_from_rfc3339(modified_str)
                            .map(|dt| SystemTime::from(dt))
                            .unwrap_or(SystemTime::now());

                    files.push(CloudFile {
                        id: id.clone(), // Use ID as path-like identifier for GDrive where possible or store ID separate
                        name: name.clone(),
                        path: id, // In GDrive, operations are by ID. We store ID in path for simplicity in renaming.
                        hash,
                        size,
                        modified_time,
                        provider: "gdrive".to_string(),
                    });
                }
            }

            match json["nextPageToken"].as_str() {
                Some(token) => page_token = Some(token.to_string()),
                None => break,
            }
        }

        Ok(files)
    }

    fn rename_file(&self, file: &CloudFile, new_name: &str) -> Result<()> {
        let url = format!("https://www.googleapis.com/drive/v3/files/{}", file.id);
        let body = serde_json::json!({
            "name": new_name
        });

        let res = self.client.patch(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;

        if !res.status().is_success() {
            return Err(anyhow!("Google Drive Rename Error: {}", res.text()?));
        }
        Ok(())
    }

    fn delete_file(&self, file: &CloudFile) -> Result<()> {
        let url = format!("https://www.googleapis.com/drive/v3/files/{}", file.id);
         let res = self.client.delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()?;

        if !res.status().is_success() {
            return Err(anyhow!("Google Drive Delete Error: {}", res.text()?));
        }
        Ok(())
    }
}
