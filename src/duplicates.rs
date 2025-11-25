use crate::scanner::FileInfo;
use anyhow::Result;
use log::debug;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Allowed formats to keep
const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt"];

pub fn detect_duplicates(files: Vec<FileInfo>, skip_hash: bool) -> Result<(Vec<Vec<PathBuf>>, Vec<FileInfo>)> {
    // Filter to only allowed formats first
    let filtered_files: Vec<FileInfo> = files
        .into_iter()
        .filter(|f| ALLOWED_EXTENSIONS.contains(&f.extension.as_str()))
        .collect();
    
    debug!("Filtered to {} files with allowed extensions", filtered_files.len());
    
    // If skip_hash is true, skip duplicate detection entirely
    if skip_hash {
        debug!("Skipping MD5 hash computation (cloud storage mode)");
        return Ok((Vec::new(), filtered_files));
    }
    
    // Build hash map: file_hash -> list of file infos
    let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for file_info in &filtered_files {
        if !file_info.is_failed_download && !file_info.is_too_small {
            let hash = compute_md5(&file_info.original_path)?;
            hash_map
                .entry(hash)
                .or_insert_with(Vec::new)
                .push(file_info.clone());
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
            debug!("Found duplicate group with {} files, keeping: {}", file_infos.len(), kept_file.original_name);
        }
    }

    // Return only non-duplicate files (including filtered out formats)
    let clean_files: Vec<FileInfo> = filtered_files
        .into_iter()
        .filter(|f| !duplicate_paths.contains(&f.original_path))
        .collect();

    Ok((duplicate_groups, clean_files))
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
        candidates_with_depth.iter().map(|(_, d)| *d).min().unwrap_or(usize::MAX)
    } else {
        candidates_with_depth.iter()
            .filter(|(i, _)| normalized_set.contains(i))
            .map(|(_, d)| *d)
            .min().unwrap_or(usize::MAX)
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
    use tempfile::TempDir;
    use std::time::Duration;

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
            },
        ];

        let (dup_groups, clean_files) = detect_duplicates(files, false)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1); // Only one should remain

        Ok(())
    }

    #[test]
    fn test_strip_variant_suffix() {
        assert_eq!(
            strip_variant_suffix("Book Title (1).pdf"),
            "Book Title.pdf"
        );
        assert_eq!(
            strip_variant_suffix("Another (2).epub"),
            "Another.epub"
        );
        assert_eq!(
            strip_variant_suffix("No Variant.pdf"),
            "No Variant.pdf"
        );
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
        };

        let files = vec![f1, f2];
        let kept = select_file_to_keep(&files);

        // Should keep f2 because it's newer (both have same depth and normalization status)
        assert_eq!(kept.original_name, "file2.pdf");
    }

    #[test]
    fn test_detect_duplicates_skip_hash() {
        let tmp_dir = TempDir::new().unwrap();

        let files = vec![
            FileInfo {
                original_path: tmp_dir.path().join("file1.pdf"),
                original_name: "file1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 100,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: tmp_dir.path().join("file1.pdf"),
            }
        ];

        // Even if files are present, skip_hash=true should return empty duplicate groups
        let (dup_groups, clean_files) = detect_duplicates(files.clone(), true).unwrap();

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
        };

        let files = vec![f1, f2];
        let variants = detect_name_variants(&files).unwrap();

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].len(), 2);
    }
}
