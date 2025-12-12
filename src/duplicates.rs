use crate::scanner::FileInfo;
use anyhow::Result;
use log::{debug, warn};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use strsim::jaro_winkler;

// Allowed formats to keep
const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt"];

pub fn detect_duplicates(files: Vec<FileInfo>, skip_hash: bool) -> Result<(Vec<Vec<PathBuf>>, Vec<FileInfo>, Vec<(FileInfo, String)>)> {
    // Filter to only allowed formats first
    let filtered_files: Vec<FileInfo> = files
        .into_iter()
        .filter(|f| ALLOWED_EXTENSIONS.contains(&f.extension.as_str()))
        .collect();
    
    debug!("Filtered to {} files with allowed extensions", filtered_files.len());
    
    // Build hash map: key -> list of file infos
    // Key is either MD5 hash or normalized filename depending on skip_hash
    let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();
    let mut hash_errors: Vec<(FileInfo, String)> = Vec::new();

    if skip_hash {
        debug!("Skipping MD5 hash computation, using fuzzy filename matching + size comparison");

        // Group by size first (exact size match required)
        let mut size_groups: HashMap<u64, Vec<FileInfo>> = HashMap::new();
        for file_info in &filtered_files {
            if !file_info.is_failed_download && !file_info.is_too_small {
                size_groups
                    .entry(file_info.size)
                    .or_insert_with(Vec::new)
                    .push(file_info.clone());
            }
        }

        // Within each size group, use fuzzy matching
        const SIMILARITY_THRESHOLD: f64 = 0.85;
        let mut group_id = 0;

        for (_size, files_with_same_size) in size_groups {
            if files_with_same_size.len() < 2 {
                // Only one file with this size, cannot be duplicate
                continue;
            }

            // Compare all pairs within this size group
            let mut already_grouped: Vec<usize> = Vec::new();

            for i in 0..files_with_same_size.len() {
                if already_grouped.contains(&i) {
                    continue;
                }

                let mut current_group = vec![i];
                let name_i = files_with_same_size[i].new_name.clone()
                    .unwrap_or_else(|| files_with_same_size[i].original_name.clone());

                for j in (i + 1)..files_with_same_size.len() {
                    if already_grouped.contains(&j) {
                        continue;
                    }

                    let name_j = files_with_same_size[j].new_name.clone()
                        .unwrap_or_else(|| files_with_same_size[j].original_name.clone());

                    let similarity = jaro_winkler(&name_i, &name_j);

                    if similarity >= SIMILARITY_THRESHOLD {
                        current_group.push(j);
                        already_grouped.push(j);
                        debug!("Fuzzy match: '{}' ~ '{}' (similarity: {:.2})", name_i, name_j, similarity);
                    }
                }

                // If we found matches, add to hash_map
                if current_group.len() > 1 {
                    let group_key = format!("fuzzy_group_{}", group_id);
                    group_id += 1;

                    for idx in current_group {
                        hash_map
                            .entry(group_key.clone())
                            .or_insert_with(Vec::new)
                            .push(files_with_same_size[idx].clone());
                    }
                }

                already_grouped.push(i);
            }
        }
    } else {
        // Optimization: Group by size first
        // Only compute MD5 for files that have the same size
        let mut size_map: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
        
        for file_info in &filtered_files {
            if !file_info.is_failed_download && !file_info.is_too_small {
                size_map
                    .entry(file_info.size)
                    .or_insert_with(Vec::new)
                    .push(file_info);
            }
        }

        debug!("Grouped {} files into {} size groups", filtered_files.len(), size_map.len());

        for (size, files) in size_map {
            // If only one file has this size, it cannot be a duplicate (unless size is 0, but we filter those)
            if files.len() == 1 {
                continue;
            }

            debug!("Size {} has {} potential duplicates, computing hashes...", size, files.len());
            
            for file_info in files {
                match compute_md5(&file_info.original_path) {
                    Ok(hash) => {
                        hash_map
                            .entry(hash)
                            .or_insert_with(Vec::new)
                            .push(file_info.clone());
                    },
                    Err(e) => {
                        warn!("Failed to compute hash for {}: {}", file_info.original_path.display(), e);
                        hash_errors.push((file_info.clone(), e.to_string()));
                    }
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
            debug!("Found duplicate group with {} files, keeping: {}", file_infos.len(), kept_file.original_name);
        }
    }

    // Return only non-duplicate files (including filtered out formats)
    let clean_files: Vec<FileInfo> = filtered_files
        .into_iter()
        .filter(|f| !duplicate_paths.contains(&f.original_path))
        // Also exclude files that failed hash computation so we don't treat them as unique/clean
        .filter(|f| !hash_errors.iter().any(|(err_f, _)| err_f.original_path == f.original_path))
        .collect();

    Ok((duplicate_groups, clean_files, hash_errors))
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

        let (dup_groups, clean_files, errors) = detect_duplicates(files, false)?;

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 2);
        assert_eq!(clean_files.len(), 1); // Only one should remain
        assert!(errors.is_empty());

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
        let (dup_groups, clean_files, _) = detect_duplicates(files.clone(), true).unwrap();

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
        };

        let files = vec![f1, f2];

        // When skip_hash is true, we expect it to find duplicates based on new_name
        let (dup_groups, clean_files, _) = detect_duplicates(files, true).unwrap();

        assert_eq!(dup_groups.len(), 1, "Should find 1 duplicate group");
        assert_eq!(dup_groups[0].len(), 2, "Group should have 2 files");
        assert_eq!(clean_files.len(), 1, "Should keep 1 file");
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_detect_duplicates_empty_list() {
        let files: Vec<FileInfo> = vec![];
        let (dup_groups, clean_files, errors) = detect_duplicates(files, false).unwrap();

        assert!(dup_groups.is_empty());
        assert!(clean_files.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_detect_duplicates_single_file() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("single.pdf");
        fs::write(&file_path, "unique content").unwrap();

        let files = vec![FileInfo {
            original_path: file_path.clone(),
            original_name: "single.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 14,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: file_path,
        }];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // Single file cannot be a duplicate
        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 1);
    }

    #[test]
    fn test_detect_duplicates_mobi_excluded() {
        // MOBI files should NOT be included in duplicate detection
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.mobi");
        let file2 = tmp_dir.path().join("book2.mobi");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.mobi".to_string(),
                extension: ".mobi".to_string(),
                size: 17,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1,
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.mobi".to_string(),
                extension: ".mobi".to_string(),
                size: 17,
                modified_time: std::time::SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2,
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // MOBI files should be filtered out entirely
        assert!(dup_groups.is_empty());
        assert!(clean_files.is_empty()); // Filtered out
    }

    #[test]
    fn test_detect_duplicates_three_way_duplicate() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf");
        let file3 = tmp_dir.path().join("book3.pdf");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();
        fs::write(&file3, "identical content").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Book.pdf".to_string()),
                new_path: tmp_dir.path().join("Book.pdf"),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
            FileInfo {
                original_path: file3.clone(),
                original_name: "book3.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file3.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        assert_eq!(dup_groups.len(), 1);
        assert_eq!(dup_groups[0].len(), 3); // All three are duplicates
        assert_eq!(clean_files.len(), 1); // Only one kept
    }

    #[test]
    fn test_detect_duplicates_same_size_different_content() {
        let tmp_dir = TempDir::new().unwrap();

        // Two files with same size but different content
        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf");
        fs::write(&file1, "content A here").unwrap(); // 14 bytes
        fs::write(&file2, "content B here").unwrap(); // 14 bytes

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 14,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 14,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // Same size but different hash = not duplicates
        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 2);
    }

    #[test]
    fn test_detect_duplicates_different_sizes() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf");
        fs::write(&file1, "short").unwrap();
        fs::write(&file2, "much longer content here").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 5,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 24,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // Different sizes = cannot be duplicates, no hash computed
        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 2);
    }

    #[test]
    fn test_detect_duplicates_failed_download_excluded() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf.download");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf.download".to_string(),
                extension: ".download".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: true,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // .download files are filtered out by extension
        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 1); // Only the .pdf remains
    }

    #[test]
    fn test_detect_duplicates_too_small_excluded() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.pdf");
        let file2 = tmp_dir.path().join("book2.pdf");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: true, // Marked as too small
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // Too small files should be excluded from duplicate detection
        assert!(dup_groups.is_empty());
        assert_eq!(clean_files.len(), 2); // Both remain, but one couldn't be compared
    }

    #[test]
    fn test_select_file_to_keep_combined_priority() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();
        let older = now - Duration::from_secs(3600);

        // File 1: Deep path, not normalized, newer
        let f1 = FileInfo {
            original_path: tmp_dir.path().join("a").join("b").join("c").join("deep.pdf"),
            original_name: "deep.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("a").join("b").join("c").join("deep.pdf"),
        };

        // File 2: Shallow path, not normalized, older
        let f2 = FileInfo {
            original_path: tmp_dir.path().join("shallow.pdf"),
            original_name: "shallow.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: older,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("shallow.pdf"),
        };

        // File 3: Deep path, normalized, older
        let f3 = FileInfo {
            original_path: tmp_dir.path().join("a").join("b").join("normalized.pdf"),
            original_name: "normalized.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: older,
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Clean Name.pdf".to_string()),
            new_path: tmp_dir.path().join("a").join("b").join("Clean Name.pdf"),
        };

        let files = vec![f1, f2, f3];
        let kept = select_file_to_keep(&files);

        // Should prefer normalized (f3) over shallowest non-normalized (f2)
        assert!(kept.new_name.is_some());
    }

    #[test]
    fn test_strip_variant_suffix_edge_cases() {
        // No parentheses
        assert_eq!(strip_variant_suffix("Book Title.pdf"), "Book Title.pdf");

        // Text in parentheses that's not a number
        assert_eq!(strip_variant_suffix("Book (Author).pdf"), "Book (Author).pdf");

        // Multiple variant markers - only last one stripped
        assert_eq!(strip_variant_suffix("Book (1) (2).pdf"), "Book (1).pdf");

        // No extension
        assert_eq!(strip_variant_suffix("Book (1)"), "Book");

        // Higher numbers
        assert_eq!(strip_variant_suffix("Book (99).pdf"), "Book.pdf");
    }

    #[test]
    fn test_detect_duplicates_epub_included() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("book1.epub");
        let file2 = tmp_dir.path().join("book2.epub");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "book1.epub".to_string(),
                extension: ".epub".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "book2.epub".to_string(),
                extension: ".epub".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // EPUB should be included
        assert_eq!(dup_groups.len(), 1);
        assert_eq!(clean_files.len(), 1);
    }

    #[test]
    fn test_detect_duplicates_txt_included() {
        let tmp_dir = TempDir::new().unwrap();

        let file1 = tmp_dir.path().join("notes1.txt");
        let file2 = tmp_dir.path().join("notes2.txt");
        fs::write(&file1, "identical content").unwrap();
        fs::write(&file2, "identical content").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: file1.clone(),
                original_name: "notes1.txt".to_string(),
                extension: ".txt".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file1.clone(),
            },
            FileInfo {
                original_path: file2.clone(),
                original_name: "notes2.txt".to_string(),
                extension: ".txt".to_string(),
                size: 17,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: file2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // TXT should be included
        assert_eq!(dup_groups.len(), 1);
        assert_eq!(clean_files.len(), 1);
    }

    #[test]
    fn test_multiple_duplicate_groups() {
        let tmp_dir = TempDir::new().unwrap();

        // Group 1: identical A
        let a1 = tmp_dir.path().join("a1.pdf");
        let a2 = tmp_dir.path().join("a2.pdf");
        fs::write(&a1, "content A").unwrap();
        fs::write(&a2, "content A").unwrap();

        // Group 2: identical B
        let b1 = tmp_dir.path().join("b1.pdf");
        let b2 = tmp_dir.path().join("b2.pdf");
        fs::write(&b1, "content B").unwrap();
        fs::write(&b2, "content B").unwrap();

        let now = std::time::SystemTime::now();
        let files = vec![
            FileInfo {
                original_path: a1.clone(),
                original_name: "a1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9, // "content A".len()
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: a1.clone(),
            },
            FileInfo {
                original_path: a2.clone(),
                original_name: "a2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: a2.clone(),
            },
            FileInfo {
                original_path: b1.clone(),
                original_name: "b1.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9, // Same size but different content
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: b1.clone(),
            },
            FileInfo {
                original_path: b2.clone(),
                original_name: "b2.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 9,
                modified_time: now,
                is_failed_download: false,
                is_too_small: false,
                new_name: None,
                new_path: b2.clone(),
            },
        ];

        let (dup_groups, clean_files, _) = detect_duplicates(files, false).unwrap();

        // Should find 2 groups
        assert_eq!(dup_groups.len(), 2);
        // Should keep 2 files (one from each group)
        assert_eq!(clean_files.len(), 2);
    }

    #[test]
    fn test_select_file_to_keep_same_depth_different_time() {
        let tmp_dir = TempDir::new().unwrap();
        let now = std::time::SystemTime::now();
        let older = now - Duration::from_secs(7200); // 2 hours ago
        let oldest = now - Duration::from_secs(14400); // 4 hours ago

        let f1 = FileInfo {
            original_path: tmp_dir.path().join("file1.pdf"),
            original_name: "file1.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: oldest,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file1.pdf"),
        };

        let f2 = FileInfo {
            original_path: tmp_dir.path().join("file2.pdf"),
            original_name: "file2.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: now, // Newest
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file2.pdf"),
        };

        let f3 = FileInfo {
            original_path: tmp_dir.path().join("file3.pdf"),
            original_name: "file3.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: older,
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("file3.pdf"),
        };

        let files = vec![f1, f2, f3];
        let kept = select_file_to_keep(&files);

        // All same depth and none normalized, should keep newest (f2)
        assert_eq!(kept.original_name, "file2.pdf");
    }
}
