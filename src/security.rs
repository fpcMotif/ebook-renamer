#![allow(dead_code)]

use std::path::{Path, PathBuf, Component};
use std::fs;
use anyhow::{anyhow, Result};
use regex::Regex;
use crate::scanner::FileInfo;

const MAX_FILENAME_LENGTH: usize = 255;
const MAX_DEPTH: usize = 1000;
const MAX_HASH_FILE_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Path traversal attempt detected: {0}")]
    PathTraversal(String),
    #[error("Invalid filename: {0}")]
    InvalidFilename(String),
    #[error("Filename too long: {0} > {1}")]
    FilenameTooLong(String, usize),
    #[error("Invalid path component: {0}")]
    InvalidPathComponent(String),
    #[error("File too large for processing: {0} bytes")]
    FileTooLarge(u64),
    #[error("Depth limit exceeded: {0} > {1}")]
    DepthTooLarge(usize, usize),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub fn sanitize_filename(input: &str) -> Result<String> {
    if input.contains('\0') {
        return Err(anyhow!(SecurityError::InvalidFilename("Null byte detected".to_string())));
    }
    let dangerous_chars = Regex::new(r"[\x00-\x1F\x7F<>:\x22/\\|?*\n\r\t]")?;
    let sanitized = dangerous_chars.replace_all(input, "_");
    if sanitized.contains("..") {
        return Err(anyhow!(SecurityError::PathTraversal("Path traversal in filename".to_string())));
    }
    if sanitized.len() > MAX_FILENAME_LENGTH {
        return Err(anyhow!(SecurityError::FilenameTooLong(sanitized.to_string(), MAX_FILENAME_LENGTH)));
    }
    if sanitized.trim().is_empty() {
        return Err(anyhow!(SecurityError::InvalidFilename("Empty filename".to_string())));
    }
    Ok(sanitized.to_string())
}

pub fn validate_path_traversal(path: &Path, base: &Path) -> Result<PathBuf> {
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let resolved = resolve_path_manually(path, base)?;
            return validate_path_traversal(&resolved, base);
        }
    };
    let canonical_base = match base.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err(anyhow!(SecurityError::InvalidInput("Base directory not accessible".to_string())));
        }
    };
    if !canonical_path.starts_with(&canonical_base) {
        return Err(anyhow!(SecurityError::PathTraversal(format!(
            "Path {:?} escapes base directory {:?}", path, base
        ))));
    }
    Ok(canonical_path)
}

fn resolve_path_manually(path: &Path, base: &Path) -> Result<PathBuf> {
    let mut result = base.to_path_buf();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !result.pop() {
                    return Err(anyhow!(SecurityError::PathTraversal("Path traversal beyond root".to_string())));
                }
            }
            Component::CurDir => {}
            Component::Normal(name) => {
                let name_str = name.to_string_lossy();
                let sanitized = sanitize_filename(&name_str)?;
                result.push(sanitized);
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!(SecurityError::InvalidPathComponent("Absolute paths not allowed".to_string())));
            }
        }
    }
    Ok(result)
}

pub fn validate_max_depth(depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(anyhow!(SecurityError::DepthTooLarge(depth, MAX_DEPTH)));
    }
    Ok(())
}

pub fn validate_file_size(size: u64) -> Result<()> {
    if size > MAX_HASH_FILE_SIZE {
        return Err(anyhow!(SecurityError::FileTooLarge(size)));
    }
    Ok(())
}

pub fn shell_escape(arg: &str) -> String {
    if arg.is_empty() {
        return String::from("''");
    }
    let mut escaped = String::new();
    escaped.push('\'');
    for ch in arg.chars() {
        if ch == '\'' {
            escaped.push_str("'\\''");
        } else if ch == '\n' {
            escaped.push_str("\\n");
        } else if ch == '\r' {
            escaped.push_str("\\r");
        } else if ch == '\t' {
            escaped.push_str("\\t");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

pub fn shell_escape_safe(arg: &str) -> String {
    if arg.is_empty() {
        return String::from("''");
    }
    let safe = Regex::new(r"^[a-zA-Z0-9_./-]+$").unwrap();
    if safe.is_match(arg) {
        return arg.to_string();
    }
    shell_escape(arg)
}

pub fn validate_extensions(extensions: &[String]) -> Result<()> {
    let allowed_extensions = vec![
        ".pdf", ".epub", ".txt", ".mobi", ".azw3", ".azw",
        ".download", ".crdownload", ".tar.gz"
    ];
    for ext in extensions {
        let ext = ext.trim().to_lowercase();
        if ext.is_empty() { continue; }
        let normalized_ext = if ext.starts_with('.') { ext.to_string() } else { format!(".{}", ext) };
        if !allowed_extensions.contains(&normalized_ext.as_str()) {
            return Err(anyhow!(SecurityError::InvalidInput(format!("Extension '{}' not allowed", ext))));
        }
    }
    Ok(())
}

pub fn sanitize_path_for_logging(path: &Path) -> String {
    path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "[invalid_path]".to_string())
}

#[allow(dead_code)]
pub fn create_file_info_atomically(path: &Path, base: &Path) -> Result<FileInfo> {
    use std::fs::OpenOptions;
    let validated_path = validate_path_traversal(path, base)?;
    let file = OpenOptions::new().read(true).open(&validated_path)?;
    let metadata = file.metadata()?;
    let original_name = validated_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let extension = validated_path.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
    Ok(FileInfo {
        original_path: validated_path.clone(),
        original_name, extension, size: metadata.len(),
        modified_time: metadata.modified()?,
        is_failed_download: false, is_too_small: false,
        new_name: None, new_path: validated_path,
    })
}

pub fn atomic_rename(from: &Path, to: &Path) -> Result<()> {
    if to.exists() {
        return Err(anyhow!(SecurityError::InvalidInput(format!(
            "Refusing to overwrite existing path: {}",
            to.display()
        ))));
    }
    std::fs::rename(from, to)?;
    Ok(())
}

pub fn safe_delete(path: &Path) -> Result<()> {
    if !path.exists() { return Ok(()); }
    if path.is_dir() { return Err(anyhow!("Cannot delete directory with safe_delete")); }
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_valid() {
        assert_eq!(sanitize_filename("test.pdf").unwrap(), "test.pdf");
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("test"), "'test'");
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_validate_max_depth() {
        assert!(validate_max_depth(10).is_ok());
        assert!(validate_max_depth(10000).is_err());
    }

    #[test]
    fn test_safe_delete_nonexistent() {
        let path = Path::new("/nonexistent/file.txt");
        assert!(safe_delete(path).is_ok());
    }
}
