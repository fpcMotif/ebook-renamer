use crate::cli::CloudMode;
use crate::scanner::{CloudMetadata, FileInfo};
use anyhow::Result;
use log::{debug, warn};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Allowed formats to keep
const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt"];

pub fn detect_duplicates(
    files: Vec<FileInfo>,
    cloud_mode: CloudMode,
) -> Result<(Vec<Vec<PathBuf>>, Vec<FileInfo>)> {
    // Filter to only allowed formats first
    let filtered_files: Vec<FileInfo> = files
        .into_iter()
        .filter(|f| ALLOWED_EXTENSIONS.contains(&f.extension.as_str()))
        .collect();

    debug!(
        "Filtered to {} files with allowed extensions",
        filtered_files.len()
    );

    // Build hash map: key -> list of file infos
    let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();

    if matches!(cloud_mode, CloudMode::Metadata) {
        warn!("Cloud metadata mode enabled: grouping duplicates by normalized name + size only");
        for file_info in &filtered_files {
            if let Some(key) = metadata_key(file_info) {
                hash_map
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(file_info.clone());
            }
        }
    } else {
        for file_info in &filtered_files {
            match duplicate_key_for_file(file_info, cloud_mode) {
                Ok(Some(key)) => {
                    hash_map
                        .entry(key)
                        .or_insert_with(Vec::new)
                        .push(file_info.clone());
                }
                Ok(None) => {}
                Err(e) => {
                    debug!(
                        "Failed to compute duplicate key for {}: {}",
                        file_info.original_path.display(),
                        e
                    );
                }
            }
        }
    }

    // Group duplicates by hash and apply retention strategy
    let mut duplicate_groups: Vec<Vec<PathBuf>> = Vec::new();
    let mut duplicate_paths = std::collections::HashSet::new();

    for (_hash, file_infos) in hash_map {
        if file_infos.len() > 1 {
            // Multiple files with same hash - apply retention strategy
            let kept_file = select_file_to_keep(&file_infos);

            let mut group_paths: Vec<PathBuf> = Vec::new();
            group_paths.push(kept_file.original_path.clone());

            for file_info in &file_infos {
                if file_info.original_path != kept_file.original_path {
                    duplicate_paths.insert(file_info.original_path.clone());
                    group_paths.push(file_info.original_path.clone());
                }
            }

            duplicate_groups.push(group_paths);
            debug!(
                "Found duplicate group with {} files, keeping: {}",
                file_infos.len(),
                kept_file.original_name
            );
        }
    }

    // Return only non-duplicate files (including filtered out formats)
    let clean_files: Vec<FileInfo> = filtered_files
        .into_iter()
        .filter(|f| !duplicate_paths.contains(&f.original_path))
        .collect();

    Ok((duplicate_groups, clean_files))
}

fn duplicate_key_for_file(file_info: &FileInfo, cloud_mode: CloudMode) -> Result<Option<String>> {
    if file_info.is_failed_download || file_info.is_too_small {
        return Ok(None);
    }

    if matches!(cloud_mode, CloudMode::Api | CloudMode::Hybrid) {
        if let Some(hash) = provider_hash(&file_info.cloud_metadata) {
            debug!(
                "Using provider hash for {} (mode {:?})",
                file_info.original_path.display(),
                cloud_mode
            );
            return Ok(Some(format!("hash:{}", hash)));
        } else if matches!(cloud_mode, CloudMode::Api) {
            warn!(
                "API mode requested but no provider hash was found for {} – falling back to local hash",
                file_info.original_path.display()
            );
        }
    }

    if matches!(cloud_mode, CloudMode::Hybrid) && file_info.cloud_metadata.is_virtual {
        if let Some(meta_key) = metadata_key(file_info) {
            warn!(
                "Hybrid mode: using metadata duplicate key for virtual file {} to avoid local hashing",
                file_info.original_path.display()
            );
            return Ok(Some(meta_key));
        }
    }

    match compute_md5(&file_info.original_path) {
        Ok(hash) => Ok(Some(format!("hash:{}", hash))),
        Err(e) => {
            debug!(
                "Failed to compute hash for {}: {}",
                file_info.original_path.display(),
                e
            );
            Ok(None)
        }
    }
}

fn metadata_key(file_info: &FileInfo) -> Option<String> {
    if file_info.is_failed_download || file_info.is_too_small {
        return None;
    }

    let name = file_info
        .new_name
        .clone()
        .unwrap_or_else(|| file_info.original_name.clone())
        .to_lowercase();
    Some(format!("meta:{}::{}", name, file_info.size))
}

fn provider_hash(cloud_metadata: &CloudMetadata) -> Option<String> {
    if let Some(ref hash) = cloud_metadata.dropbox_content_hash {
        return Some(format!("dropbox:{}", hash));
    }
    if let Some(ref hash) = cloud_metadata.gdrive_md5_checksum {
        return Some(format!("gdrive:{}", hash));
    }

    None
}

// Select file to keep based on priority: normalized > shortest path > newest
fn select_file_to_keep(files: &[FileInfo]) -> &FileInfo {
    // Priority 1: Already normalized files (have new_name set)
    let normalized_indices: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.new_name.is_some())
        .map(|(i, _)| i)
        .collect();

    // Use the original files slice, but remember which ones are normalized
    let normalized_set: std::collections::HashSet<usize> = normalized_indices.into_iter().collect();

    // Priority 2: Shortest path (fewest directory components) among normalized files, then all files
    let candidates_with_depth: Vec<(usize, usize)> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (i, f.original_path.components().count()))
        .collect();

    let min_depth = if normalized_set.is_empty() {
        candidates_with_depth
            .iter()
            .map(|(_, d)| *d)
            .min()
            .unwrap_or(usize::MAX)
    } else {
        candidates_with_depth
            .iter()
            .filter(|(i, _)| normalized_set.contains(i))
            .map(|(_, d)| *d)
            .min()
            .unwrap_or(usize::MAX)
    };

    let shallowest_indices: Vec<usize> = candidates_with_depth
        .into_iter()
        .filter(|(i, depth)| {
            *depth == min_depth && (normalized_set.is_empty() || normalized_set.contains(i))
        })
        .map(|(i, _)| i)
        .collect();

    // Priority 3: Newest modification time among the shallowest candidates
    let best_index = shallowest_indices
        .iter()
        .max_by(|&&a, &&b| files[a].modified_time.cmp(&files[b].modified_time))
        .copied()
        .unwrap_or_else(|| {
            // Fallback: if no shallowest indices (shouldn't happen), return first file
            if files.is_empty() {
                panic!("select_file_to_keep called with empty files slice");
            }
            0
        });

    &files[best_index]
}

#[allow(dead_code)]
pub fn detect_name_variants(files: &[FileInfo]) -> Result<Vec<Vec<usize>>> {
    // Group files by normalized name (treating (1), (2), etc. as variants)
    let mut name_groups: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, file_info) in files.iter().enumerate() {
        if let Some(ref new_name) = file_info.new_name {
            // Strip off (1), (2), etc. to find base name
            let base_name = strip_variant_suffix(new_name);
            name_groups
                .entry(base_name)
                .or_insert_with(Vec::new)
                .push(idx);
        }
    }

    // Keep only groups with duplicates
    let variants: Vec<Vec<usize>> = name_groups
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();

    Ok(variants)
}

#[allow(dead_code)]
fn strip_variant_suffix(filename: &str) -> String {
    // Match patterns like " (1)", " (2)", etc. at the end before extension
    // Use a simpler approach without look-ahead
    if let Some(dot_idx) = filename.rfind('.') {
        let (name_part, ext_part) = filename.split_at(dot_idx);
        let re = regex::Regex::new(r" \(\d+\)$").unwrap();
        let cleaned_name = re.replace(name_part, "").to_string();
        format!("{}{}", cleaned_name, ext_part)
    } else {
        let re = regex::Regex::new(r" \(\d+\)$").unwrap();
        re.replace(filename, "").to_string()
    }
}

fn compute_md5(path: &std::path::Path) -> Result<String> {
    use std::io::Read;

    const BUFFER_SIZE: usize = 8192;

    let mut file = fs::File::open(path)?;
    let mut hasher = md5::Context::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.consume(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.compute()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_detect_duplicates_by_hash() -> Result<()> {
        let tmp_dir = TempDir::new()?;

        // Create two identical files
        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf");
        fs::write(&file1, "identical content")?;
        fs::write(&file2, "identical content")?;

        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Book 1.pdf".to_string()),
                new_path: tmp_dir.path().join("Book 1.pdf"),
                cloud_metadata: CloudMetadata::default(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Book 2.pdf".to_string()),
                new_path: tmp_dir.path().join("Book 2.pdf"),
                cloud_metadata: CloudMetadata::default(),
            },
        ];

        let (dup_groups, clean_files) = detect_duplicates(files, CloudMode::Local)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1); // Only one should remain

        Ok(())
    }

    #[test]
    fn test_metadata_mode_groups_by_name_and_size() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let file1 = tmp_dir.path().join("paper.pdf");
        let file2 = tmp_dir.path().join("paper (1).pdf");

        fs::write(&file1, "abc")?;
        fs::write(&file2, "def")?; // Different content but same length

        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "paper.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 3,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Normalized.pdf".to_string()),
                new_path: tmp_dir.path().join("Normalized.pdf"),
                cloud_metadata: CloudMetadata::default(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "paper (1).pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 3,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Normalized.pdf".to_string()),
                new_path: tmp_dir.path().join("Normalized.pdf"),
                cloud_metadata: CloudMetadata::default(),
            },
        ];

        let (dup_groups, clean_files) = detect_duplicates(files, CloudMode::Metadata)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1);
        Ok(())
    }

    #[test]
    fn test_api_mode_prefers_provider_hash() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let file1 = tmp_dir.path().join("cloud1.pdf");
        let file2 = tmp_dir.path().join("cloud2.pdf");

        fs::write(&file1, "abc")?;
        fs::write(&file2, "xyz")?;

        let mut cloud_meta = CloudMetadata::default();
        cloud_meta.dropbox_content_hash = Some("same_hash".to_string());

        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "cloud1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 3,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Cloud.pdf".to_string()),
                new_path: file1.clone(),
                cloud_metadata: cloud_meta.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "cloud2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 3,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Cloud.pdf".to_string()),
                new_path: file2.clone(),
                cloud_metadata: cloud_meta,
            },
        ];

        let (dup_groups, clean_files) = detect_duplicates(files, CloudMode::Api)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1);
        Ok(())
    }

    #[test]
    fn test_hybrid_mode_falls_back_to_metadata_for_virtual_mounts() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let file1 = tmp_dir.path().join("v1.pdf");
        let file2 = tmp_dir.path().join("v2.pdf");

        fs::write(&file1, "same_len_a")?;
        fs::write(&file2, "same_len_b")?; // Same size, different content

        let mut meta1 = CloudMetadata::default();
        meta1.is_virtual = true;
        let mut meta2 = CloudMetadata::default();
        meta2.is_virtual = true;

        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "v1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Virtual.pdf".to_string()),
                new_path: file1.clone(),
                cloud_metadata: meta1,
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "v2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Virtual.pdf".to_string()),
                new_path: file2.clone(),
                cloud_metadata: meta2,
            },
        ];

        let (dup_groups, clean_files) = detect_duplicates(files, CloudMode::Hybrid)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1);
        Ok(())
    }

    #[test]
    fn test_strip_variant_suffix() {
        assert_eq!(strip_variant_suffix("Book Title (1).pdf"), "Book Title.pdf");
        assert_eq!(strip_variant_suffix("Another (2).epub"), "Another.epub");
        assert_eq!(strip_variant_suffix("No Variant.pdf"), "No Variant.pdf");
    }

    #[test]
    fn test_select_file_to_keep_normalized() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();

        // File 1: Not normalized
        let f1 = FileInfo {
            original_path: tmp_dir.path().join("original.pdf"),
            original_name: "original.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("original.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        // File 2: Normalized
        let f2 = FileInfo {
            original_path: tmp_dir.path().join("normalized.pdf"),
            original_name: "normalized.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Normalized Title.pdf".to_string()),
            new_path: tmp_dir.path().join("Normalized Title.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let files = vec![f1, f2];
        let kept = select_file_to_keep(&files);

        // Should keep f2 because it's normalized
        assert!(kept.new_name.is_some());
        assert_eq!(kept.original_name, "normalized.pdf");
    }

    #[test]
    fn test_select_file_to_keep_shortest_path() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();

        // File 1: Deep path
        let f1 = FileInfo {
            original_path: tmp_dir.path().join("a").join("b").join("deep.pdf"),
            original_name: "deep.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("a").join("b").join("deep.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        // File 2: Shallow path
        let f2 = FileInfo {
            original_path: tmp_dir.path().join("shallow.pdf"),
            original_name: "shallow.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("shallow.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let files = vec![f1, f2];
        let kept = select_file_to_keep(&files);

        // Should keep f2 because it has fewer path components
        assert_eq!(kept.original_name, "shallow.pdf");
    }

    #[test]
    fn test_select_file_to_keep_newest() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();
        let older = now - Duration::from_secs(3600);

        // File 1: Older
        let f1 = FileInfo {
            original_path: tmp_dir.path().join("file1.pdf"),
            original_name: "file1.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: older,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file1.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        // File 2: Newer
        let f2 = FileInfo {
            original_path: tmp_dir.path().join("file2.pdf"),
            original_name: "file2.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file2.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let files = vec![f1, f2];
        let kept = select_file_to_keep(&files);

        // Should keep f2 because it's newer (both have same depth and normalization status)
        assert_eq!(kept.original_name, "file2.pdf");
    }

    #[test]
    fn test_detect_duplicates_skip_hash() {
        let tmp_dir = TempDir::new().unwrap();

        let files = vec![FileInfo {
            original_path: tmp_dir.path().join("file1.pdf"),
            original_name: "file1.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file1.pdf"),
            cloud_metadata: CloudMetadata::default(),
        }];

        // Even if files are present, skip_hash=true should return empty duplicate groups
        let (dup_groups, clean_files) =
            detect_duplicates(files.clone(), CloudMode::Metadata).unwrap();

        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 1);
    }

    #[test]
    fn test_detect_name_variants() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();

        let f1 = FileInfo {
            original_path: tmp_dir.path().join("f1.pdf"),
            original_name: "f1.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Book.pdf".to_string()),
            new_path: tmp_dir.path().join("Book.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let f2 = FileInfo {
            original_path: tmp_dir.path().join("f2.pdf"),
            original_name: "f2.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Book (1).pdf".to_string()),
            new_path: tmp_dir.path().join("Book (1).pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let files = vec![f1, f2];
        let variants = detect_name_variants(&files).unwrap();

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].len(), 2);
    }

    #[test]
    fn test_detect_duplicates_by_name_when_skip_hash() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();

        // Two files with different original names but SAME new_name
        // And different content (simulated by not writing content, since we skip hash)

        let f1 = FileInfo {
            original_path: tmp_dir.path().join("file1.pdf"),
            original_name: "file1.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Final Name.pdf".to_string()),
            new_path: tmp_dir.path().join("Final Name.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let f2 = FileInfo {
            original_path: tmp_dir.path().join("file2.pdf"),
            original_name: "file2.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Final Name.pdf".to_string()),
            new_path: tmp_dir.path().join("Final Name.pdf"),
            cloud_metadata: CloudMetadata::default(),
        };

        let files = vec![f1, f2];

        // When skip_hash is true, we expect it to find duplicates based on new_name
        let (dup_groups, clean_files) = detect_duplicates(files, CloudMode::Metadata).unwrap();

        assert_eq!(dup_groups.len(), 1, "Should find 1 duplicate group");
        assert_eq!(dup_groups[0].len(), 2, "Group should have 2 files");
        assert_eq!(clean_files.len(), 1, "Should keep 1 file");
    }
}
