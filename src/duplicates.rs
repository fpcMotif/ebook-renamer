use crate::scanner::FileInfo;
use anyhow::Result;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use strsim::normalized_levenshtein;

// Allowed formats to keep
const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt"];
// Files smaller than this are considered "dangerously small" for ebooks
const DANGER_ZONE_SIZE: u64 = 500 * 1024; // 500 KB

pub fn detect_duplicates(
    files: Vec<FileInfo>,
    skip_hash: bool,
    fuzzy: bool,
    cloud_threshold: Option<u64>,
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

    // Build groups of potential duplicates
    let mut groups: Vec<Vec<FileInfo>> = Vec::new();

    if skip_hash {
        debug!("Skipping MD5 hash computation (cloud mode)");

        // 1. Separate files into "Small (safe to hash)" and "Large (unsafe to hash)"
        let threshold = cloud_threshold.unwrap_or(0);
        let mut small_files = Vec::new();
        let mut large_files = Vec::new();

        for file in &filtered_files {
            if !file.is_failed_download && !file.is_too_small {
                if threshold > 0 && file.size < threshold {
                    small_files.push(file.clone());
                } else {
                    large_files.push(file.clone());
                }
            }
        }

        // 2. Process Small Files (Hash-based)
        if !small_files.is_empty() {
            debug!("Hashing {} small files (< {})", small_files.len(), threshold);
            let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();
            for file in small_files {
                match compute_md5(&file.original_path) {
                    Ok(hash) => {
                        hash_map
                            .entry(hash)
                            .or_insert_with(Vec::new)
                            .push(file);
                    }
                    Err(e) => {
                        debug!("Failed to hash {}: {}", file.original_path.display(), e);
                    }
                }
            }
            for group in hash_map.into_values() {
                if group.len() > 1 {
                    groups.push(group);
                }
            }
        }

        // 3. Process Large Files (Name-based)
        // Group by EXACT normalized name first
        let mut name_map: HashMap<String, Vec<FileInfo>> = HashMap::new();
        for file in &large_files {
            let key = file
                .new_name
                .clone()
                .unwrap_or_else(|| file.original_name.clone());
            name_map.entry(key).or_insert_with(Vec::new).push(file.clone());
        }

        // If Fuzzy matching is enabled, merge similar groups
        if fuzzy {
            debug!("Performing fuzzy matching on filenames...");
            let keys: Vec<String> = name_map.keys().cloned().collect();
            let mut visited_keys: HashSet<String> = HashSet::new();
            let mut merged_groups: Vec<Vec<FileInfo>> = Vec::new();

            for i in 0..keys.len() {
                if visited_keys.contains(&keys[i]) {
                    continue;
                }
                
                let mut current_group = name_map.get(&keys[i]).unwrap().clone();
                visited_keys.insert(keys[i].clone());

                for j in (i + 1)..keys.len() {
                    if visited_keys.contains(&keys[j]) {
                        continue;
                    }

                    // Check similarity (normalized levenshtein returns 0.0 to 1.0)
                    // We only check if extensions match to avoid mixing .pdf and .epub in same group (unless that's desired? usually duplicates should be same format or handled carefully. existing logic assumes same content. Let's assume fuzzy match is strictly name similarity)
                    // Note: Normalized name usually includes extension if not stripped. The normalizer logic should handle extensions.
                    // Let's compare names without extensions for better fuzzy match? Or whole string?
                    // User said "name_similarity > 85%".

                    let sim = normalized_levenshtein(&keys[i], &keys[j]);
                    if sim > 0.85 {
                        debug!("Fuzzy match: '{}' ~= '{}' ({:.2})", keys[i], keys[j], sim);
                        let mut other_group = name_map.get(&keys[j]).unwrap().clone();
                        current_group.append(&mut other_group);
                        visited_keys.insert(keys[j].clone());
                    }
                }
                if current_group.len() > 1 {
                    merged_groups.push(current_group);
                }
            }
            groups.extend(merged_groups);
        } else {
            // Strict name match only
            for group in name_map.into_values() {
                if group.len() > 1 {
                    groups.push(group);
                }
            }
        }

    } else {
        // Standard Hash-Based Logic (existing)
        let mut size_map: HashMap<u64, Vec<&FileInfo>> = HashMap::new();
        for file_info in &filtered_files {
            if !file_info.is_failed_download && !file_info.is_too_small {
                size_map
                    .entry(file_info.size)
                    .or_insert_with(Vec::new)
                    .push(file_info);
            }
        }

        for (size, files) in size_map {
            if files.len() == 1 {
                continue;
            }
            debug!("Size {} has {} potential duplicates, computing hashes...", size, files.len());
            let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();
            
            for file_info in files {
                match compute_md5(&file_info.original_path) {
                    Ok(hash) => {
                        hash_map
                            .entry(hash)
                            .or_insert_with(Vec::new)
                            .push(file_info.clone());
                    },
                    Err(e) => {
                        debug!("Failed to compute hash for {}: {}", file_info.original_path.display(), e);
                    }
                }
            }
            for group in hash_map.into_values() {
                if group.len() > 1 {
                    groups.push(group);
                }
            }
        }
    }

    // Apply retention strategy to each group
    let mut duplicate_groups: Vec<Vec<PathBuf>> = Vec::new();
    let mut duplicate_paths: HashSet<PathBuf> = HashSet::new();

    for file_infos in groups {
        if file_infos.len() > 1 {
            let (kept_file, files_to_remove) = select_file_to_keep(&file_infos, skip_hash);
            
            // If select_file_to_keep decided we shouldn't delete anything (e.g. Danger Zone),
            // files_to_remove will be empty (or less than expected).
            
            if !files_to_remove.is_empty() {
                let mut group_paths: Vec<PathBuf> = Vec::new();
                group_paths.push(kept_file.original_path.clone());

                for remove_file in files_to_remove {
                    duplicate_paths.insert(remove_file.original_path.clone());
                    group_paths.push(remove_file.original_path.clone());
                }

                duplicate_groups.push(group_paths);
                debug!("Resolved duplicate group, keeping: {}", kept_file.original_name);
            } else {
                 debug!("Duplicate group found but safely kept all files (Danger Zone or indeterminate)");
            }
        }
    }

    // Return only non-duplicate files
    let clean_files: Vec<FileInfo> = filtered_files
        .into_iter()
        .filter(|f| !duplicate_paths.contains(&f.original_path))
        .collect();

    Ok((duplicate_groups, clean_files))
}

// Select file to keep based on priority.
// Returns (FileToKeep, Vec<FilesToDelete>)
fn select_file_to_keep(files: &[FileInfo], skip_hash: bool) -> (FileInfo, Vec<FileInfo>) {
    if skip_hash {
        // Cloud/Name-based Retention Logic

        // 1. Sort by Size (Smallest first)
        // User wants to keep SMALLER file generally.
        let mut sorted_files = files.to_vec();
        sorted_files.sort_by(|a, b| a.size.cmp(&b.size));

        let smallest = &sorted_files[0];
        let largest = &sorted_files[sorted_files.len() - 1];

        // Check Danger Zone
        // If smallest is dangerously small (< 500KB) and largest is significantly bigger (not just 1 byte diff),
        // we assume the small one might be broken/bad OCR.
        // But if ALL are small, we just pick one.
        // Logic: If (smallest < DANGER) AND (largest > DANGER), Keep Both (return empty delete list).

        if smallest.size < DANGER_ZONE_SIZE && largest.size > DANGER_ZONE_SIZE {
            // "Danger Zone": Keep safe by not deleting anything
            return (smallest.clone(), Vec::new());
        }

        // If sizes are different (and not in danger zone conflict), keep smallest.
        if smallest.size < largest.size {
             // Keep smallest, delete rest
             let to_delete = sorted_files[1..].to_vec();
             return (smallest.clone(), to_delete);
        }

        // If sizes are identical (or very close? logic says strict size), check Date.
        // Sort by Age (Oldest first) because User said: "Keep OLDER file (delete newer)"
        // Note: SystemTime doesn't implement partial_cmp directly easily sometimes, but usually does.
        // modified_time: smaller value = older.

        // Refilter to only those matching the smallest size (in case there are 3 files, 2 small, 1 big)
        // But we already handled size diff. If we are here, sizes are effectively same for the candidates we care about?
        // Actually simpler: Just sort entire list by Size ASC, then Modified ASC (Older first).

        sorted_files.sort_by(|a, b| {
            if a.size != b.size {
                a.size.cmp(&b.size)
            } else {
                a.modified_time.cmp(&b.modified_time)
            }
        });

        // First one is Smallest and Oldest. Keep it.
        let kept = sorted_files[0].clone();
        let to_delete = sorted_files[1..].to_vec();

        (kept, to_delete)

    } else {
        // Standard Logic (Same Hash)
        // Priority: Normalized > Shortest Path > Newest (standard logic reversed?)
        // Standard existing logic was: Keep Newest/Best.
        // Let's preserve existing logic for standard mode, but adapted to return list.

        // Standard logic implementation from before:
        let kept_ref = select_file_to_keep_standard(files);
        let kept = kept_ref.clone();

        let to_delete: Vec<FileInfo> = files.iter()
            .filter(|f| f.original_path != kept.original_path)
            .cloned()
            .collect();

        (kept, to_delete)
    }
}

// Original standard selection logic (Hash-based)
fn select_file_to_keep_standard(files: &[FileInfo]) -> &FileInfo {
    // Priority 1: Already normalized files
    let normalized_indices: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.new_name.is_some())
        .map(|(i, _)| i)
        .collect();
    
    let normalized_set: HashSet<usize> = normalized_indices.into_iter().collect();
    
    // Priority 2: Shortest path
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
    
    // Priority 3: Newest modification time
    let best_index = shallowest_indices
        .iter()
        .max_by(|&&a, &&b| files[a].modified_time.cmp(&files[b].modified_time))
        .copied()
        .unwrap_or(0);
    
    &files[best_index]
}

#[allow(dead_code)]
pub fn detect_name_variants(files: &[FileInfo]) -> Result<Vec<Vec<usize>>> {
    let mut name_groups: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, file_info) in files.iter().enumerate() {
        if let Some(ref new_name) = file_info.new_name {
            let base_name = strip_variant_suffix(new_name);
            name_groups
                .entry(base_name)
                .or_insert_with(Vec::new)
                .push(idx);
        }
    }

    let variants: Vec<Vec<usize>> = name_groups
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();

    Ok(variants)
}

#[allow(dead_code)]
fn strip_variant_suffix(filename: &str) -> String {
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
        if bytes_read == 0 { break; }
        hasher.consume(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.compute()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;

    // Helper to create dummy file info
    fn create_info(name: &str, size: u64, age_secs: u64) -> FileInfo {
        FileInfo {
            original_path: PathBuf::from(name),
            original_name: name.to_string(),
            extension: ".pdf".to_string(),
            size,
            modified_time: std::time::SystemTime::now() - Duration::from_secs(age_secs),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some(name.to_string()),
            new_path: PathBuf::from(name),
        }
    }

    #[test]
    fn test_cloud_retention_different_sizes() {
        // Small (good) vs Big (bad) - Keep Small
        let f1 = create_info("book.pdf", 2_000_000, 100); // 2MB
        let f2 = create_info("book.pdf", 10_000_000, 100); // 10MB

        let (kept, deleted) = select_file_to_keep(&[f1.clone(), f2.clone()], true);
        assert_eq!(kept.size, 2_000_000);
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].size, 10_000_000);
    }

    #[test]
    fn test_cloud_retention_danger_zone() {
        // Danger (<500KB) vs Normal (>500KB) - Keep Both
        let f1 = create_info("book.pdf", 200_000, 100); // 200KB (Danger)
        let f2 = create_info("book.pdf", 2_000_000, 100); // 2MB

        let (_kept, deleted) = select_file_to_keep(&[f1, f2], true);
        assert!(deleted.is_empty(), "Should keep both files when one is in danger zone");
    }

    #[test]
    fn test_cloud_retention_same_size_different_time() {
        // Same size, f1 is Older (larger age_secs), f2 is Newer
        let f1 = create_info("book.pdf", 2_000_000, 5000); // Older
        let f2 = create_info("book.pdf", 2_000_000, 100); // Newer

        let (kept, deleted) = select_file_to_keep(&[f1.clone(), f2.clone()], true);
        assert_eq!(kept.modified_time, f1.modified_time, "Should keep older file");
        assert_eq!(deleted[0].modified_time, f2.modified_time);
    }

    #[test]
    fn test_fuzzy_matching() {
        let f1 = create_info("The Rust Programming Language.pdf", 1_000_000, 100);
        // Typo/Minor diff
        let f2 = create_info("The Rust Programing Language.pdf", 1_000_000, 100);

        let files = vec![f1, f2];

        // With fuzzy=true, skip_hash=true
        let (groups, _) = detect_duplicates(files.clone(), true, true, None).unwrap();
        assert_eq!(groups.len(), 1, "Should find duplicate group with fuzzy matching");

        // With fuzzy=false
        let (groups_strict, _) = detect_duplicates(files, true, false, None).unwrap();
        assert!(groups_strict.is_empty(), "Should NOT find duplicates without fuzzy matching");
    }

    #[test]
    fn test_hybrid_threshold() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let p1 = tmp_dir.path().join("small1.pdf");
        let p2 = tmp_dir.path().join("small2.pdf");
        
        fs::write(&p1, "content")?;
        fs::write(&p2, "content")?;

        let mut f1 = create_info("small1.pdf", 100, 100);
        f1.original_path = p1;
        let mut f2 = create_info("small2.pdf", 100, 100);
        f2.original_path = p2;

        // Threshold 1000 > 100 -> Should hash
        let files = vec![f1, f2];
        let (groups, _) = detect_duplicates(files, true, false, Some(1000))?;
        assert_eq!(groups.len(), 1, "Should detect duplicates via hash for small files");

        Ok(())
    }
}
