use crate::scanner::FileInfo;
use crate::security::atomic_rename;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn resolve_rename_collisions(files: &mut [FileInfo]) -> Result<()> {
    let moving_from_paths: HashSet<PathBuf> = files
        .iter()
        .filter(|f| f.new_name.is_some() && f.original_path != f.new_path)
        .map(|f| f.original_path.clone())
        .collect();

    let mut indices: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.new_name.is_some())
        .map(|(i, _)| i)
        .collect();

    indices.sort_by(|&a, &b| {
        files[a]
            .new_path
            .cmp(&files[b].new_path)
            .then_with(|| files[a].original_path.cmp(&files[b].original_path))
    });

    let mut used_targets: HashSet<PathBuf> = HashSet::new();

    for idx in indices {
        let file = &mut files[idx];
        let Some(original_new_name) = file.new_name.clone() else {
            continue;
        };

        let parent_dir = file
            .new_path
            .parent()
            .ok_or_else(|| anyhow!("New path has no parent: {:?}", file.new_path))?
            .to_path_buf();

        let (base, ext) = split_basename_and_extension(&original_new_name, &file.extension);

        let mut suffix = 0usize;
        loop {
            let candidate_name = if suffix == 0 {
                original_new_name.clone()
            } else {
                format!("{} ({}){}", base, suffix, ext)
            };
            let candidate_path = parent_dir.join(&candidate_name);

            let candidate_taken_by_plan = used_targets.contains(&candidate_path);
            let candidate_exists_on_disk = candidate_path.exists();
            let candidate_is_moving_source = moving_from_paths.contains(&candidate_path);
            let candidate_is_self = candidate_path == file.original_path;

            let is_available = !candidate_taken_by_plan
                && (!candidate_exists_on_disk || candidate_is_moving_source || candidate_is_self);

            if is_available {
                file.new_name = Some(candidate_name);
                file.new_path = candidate_path.clone();
                used_targets.insert(candidate_path);
                break;
            }

            suffix += 1;
        }
    }

    Ok(())
}

pub fn execute_renames(files: &[FileInfo]) -> Result<()> {
    let mut remaining: HashMap<PathBuf, PathBuf> = HashMap::new();

    for file in files {
        if file.new_name.is_none() || file.original_path == file.new_path {
            continue;
        }

        if remaining
            .insert(file.original_path.clone(), file.new_path.clone())
            .is_some()
        {
            return Err(anyhow!(
                "Duplicate rename source path: {:?}",
                file.original_path
            ));
        }
    }

    if remaining.is_empty() {
        return Ok(());
    }

    let mut reserved: HashSet<PathBuf> = HashSet::new();
    for (from, to) in &remaining {
        reserved.insert(from.clone());
        reserved.insert(to.clone());
    }

    let mut temp_counter = 0usize;

    while !remaining.is_empty() {
        let mut ready_sources: Vec<PathBuf> = remaining
            .iter()
            .filter(|(_, to)| !remaining.contains_key(*to))
            .map(|(from, _)| from.clone())
            .collect();

        ready_sources.sort();

        if let Some(from) = ready_sources.into_iter().next() {
            let to = remaining
                .remove(&from)
                .ok_or_else(|| anyhow!("Missing rename mapping for {:?}", from))?;
            atomic_rename(&from, &to)?;
            reserved.remove(&from);
            reserved.insert(to);
            continue;
        }

        // Cycle detected: move one file out of the way with a temporary name.
        let mut cycle_sources: Vec<PathBuf> = remaining.keys().cloned().collect();
        cycle_sources.sort();
        let from = cycle_sources
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Cycle detected with no remaining sources"))?;
        let to = remaining
            .remove(&from)
            .ok_or_else(|| anyhow!("Missing rename mapping for {:?}", from))?;

        let temp_path = make_temp_path(&from, &mut temp_counter, &reserved)?;
        atomic_rename(&from, &temp_path)?;
        reserved.remove(&from);
        reserved.insert(temp_path.clone());

        remaining.insert(temp_path, to);
    }

    Ok(())
}

fn make_temp_path(from: &Path, counter: &mut usize, reserved: &HashSet<PathBuf>) -> Result<PathBuf> {
    let parent_dir = from.parent().unwrap_or_else(|| Path::new(""));
    let file_name = from
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Invalid filename for temp rename: {:?}", from))?;

    loop {
        let candidate_name = format!(".ebook-renamer-tmp-{}-{}", *counter, file_name);
        *counter += 1;

        let candidate_path = parent_dir.join(candidate_name);
        if reserved.contains(&candidate_path) {
            continue;
        }
        if candidate_path.exists() {
            continue;
        }
        return Ok(candidate_path);
    }
}

fn split_basename_and_extension(name_with_ext: &str, extension: &str) -> (String, String) {
    if extension.is_empty() {
        return (name_with_ext.to_string(), String::new());
    }

    if let Some(base) = name_with_ext.strip_suffix(extension) {
        (base.to_string(), extension.to_string())
    } else {
        // Fallback: try last '.' split for safety
        if let Some(dot_idx) = name_with_ext.rfind('.') {
            (
                name_with_ext[..dot_idx].to_string(),
                name_with_ext[dot_idx..].to_string(),
            )
        } else {
            (name_with_ext.to_string(), String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn file_info(path: &Path, new_name: &str) -> FileInfo {
        let original_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        FileInfo {
            original_path: path.to_path_buf(),
            original_name,
            extension: ".pdf".to_string(),
            size: 1,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some(new_name.to_string()),
            new_path: path.with_file_name(new_name),
        }
    }

    #[test]
    fn test_resolve_rename_collisions_adds_suffixes() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let a = tmp_dir.path().join("a.pdf");
        let b = tmp_dir.path().join("b.pdf");
        fs::write(&a, "a")?;
        fs::write(&b, "b")?;

        let mut files = vec![file_info(&a, "Same.pdf"), file_info(&b, "Same.pdf")];
        resolve_rename_collisions(&mut files)?;

        let mut new_names: Vec<String> = files
            .iter()
            .map(|f| f.new_name.clone().unwrap_or_default())
            .collect();
        new_names.sort();

        assert_eq!(new_names, vec!["Same (1).pdf".to_string(), "Same.pdf".to_string()]);
        Ok(())
    }

    #[test]
    fn test_resolve_rename_collisions_avoids_overwriting_existing_file() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let existing = tmp_dir.path().join("Target.pdf");
        fs::write(&existing, "existing")?;

        let a = tmp_dir.path().join("a.pdf");
        fs::write(&a, "a")?;

        let mut files = vec![file_info(&a, "Target.pdf")];
        resolve_rename_collisions(&mut files)?;

        assert_eq!(files[0].new_name.as_deref(), Some("Target (1).pdf"));
        assert_eq!(files[0].new_path, tmp_dir.path().join("Target (1).pdf"));
        Ok(())
    }

    #[test]
    fn test_execute_renames_handles_simple_cycle() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let a = tmp_dir.path().join("a.pdf");
        let b = tmp_dir.path().join("b.pdf");
        fs::write(&a, "content-a")?;
        fs::write(&b, "content-b")?;

        let files = vec![
            FileInfo {
                original_path: a.clone(),
                original_name: "a.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 1,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("b.pdf".to_string()),
                new_path: b.clone(),
            },
            FileInfo {
                original_path: b.clone(),
                original_name: "b.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 1,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("a.pdf".to_string()),
                new_path: a.clone(),
            },
        ];

        execute_renames(&files)?;

        assert_eq!(fs::read_to_string(&a)?, "content-b");
        assert_eq!(fs::read_to_string(&b)?, "content-a");
        Ok(())
    }
}
