use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ebook-renamer",
    about = "Batch rename and organize downloaded books and arXiv files",
    version = "0.1.0"
)]
pub struct Args {
    /// Target directory to scan and rename
    #[arg(
        value_name = "PATH",
        default_value = ".",
        help = "Directory to process (defaults to current directory)"
    )]
    pub path: PathBuf,

    /// Only show what would be done, don't make changes
    #[arg(
        long,
        short = 'd',
        help = "Perform dry run: show changes without applying them (Note: todo.md is always written, even in dry-run mode)"
    )]
    pub dry_run: bool,

    /// Maximum recursion depth (default: unlimited)
    #[arg(
        long,
        value_name = "DEPTH",
        default_value = "18446744073709551615",
        help = "Maximum directory depth to traverse (default: unlimited)"
    )]
    pub max_depth: usize,

    /// Disable recursive scanning, only process top-level directory
    #[arg(
        long,
        help = "Only scan the top-level directory, no recursion"
    )]
    pub no_recursive: bool,

    /// Custom file extensions to process
    #[arg(
        long,
        value_name = "EXT1,EXT2",
        help = "Comma-separated extensions to process (default: pdf,epub,txt)"
    )]
    pub extensions: Option<String>,

    /// Don't delete duplicate files, only report
    #[arg(
        long,
        help = "Don't delete duplicates, only list them"
    )]
    pub no_delete: bool,

    /// Custom path for todo.md
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to write todo.md (default: <target-dir>/todo.md)"
    )]
    pub todo_file: Option<PathBuf>,

    /// Path for detailed operation log
    #[arg(
        long,
        value_name = "PATH",
        help = "Optional path to write detailed operation log"
    )]
    pub log_file: Option<PathBuf>,

    /// Preserve non-Latin character titles as-is
    #[arg(
        long,
        help = "Preserve original non-Latin script (CJK, etc.) without modification"
    )]
    pub preserve_unicode: bool,

    /// Fetch arXiv metadata (placeholder for future implementation)
    #[arg(
        long,
        help = "Fetch arXiv metadata via API (not implemented yet)"
    )]
    pub fetch_arxiv: bool,

    /// Verbose output
    #[arg(long, short = 'v', help = "Enable verbose logging")]
    pub verbose: bool,

    /// Automatically delete small/corrupted files (< 1KB)
    #[arg(
        long,
        help = "Delete small/corrupted files (< 1KB) instead of adding to todo list"
    )]
    pub delete_small: bool,

    /// Clean up failed/broken downloads after logging them to todo.md
    #[arg(
        long,
        help = "Delete failed/broken downloads and small files after logging them to todo.md"
    )]
    pub clean_failed: bool,

    /// Output results in JSON format (for testing)
    #[arg(
        long,
        help = "Output operations in JSON format instead of human-readable text"
    )]
    pub json: bool,

    /// Skip MD5 hash computation (for cloud storage to avoid downloading files)
    #[arg(
        long,
        help = "Skip MD5 hash computation for duplicate detection (useful for cloud storage like Dropbox to avoid triggering file downloads)"
    )]
    pub skip_cloud_hash: bool,

    /// Automatically clean up .download/.crdownload folders after extracting PDFs
    #[arg(
        long,
        help = "Automatically remove empty .download/.crdownload folders after extracting PDFs"
    )]
    pub cleanup_downloads: bool,

    /// Cloud provider to use (dropbox, gdrive)
    #[arg(
        long,
        value_name = "PROVIDER",
        help = "Cloud provider to use (dropbox, gdrive). If set, operates on cloud files instead of local."
    )]
    pub cloud_provider: Option<String>,

    /// Access token or credentials file for cloud provider
    #[arg(
        long,
        value_name = "TOKEN/FILE",
        help = "Access token (Dropbox) or credentials file (Google Drive). If not provided, will look for environment variables."
    )]
    pub cloud_secret: Option<String>,
}

impl Args {
    #[allow(dead_code)]
    pub fn get_extensions(&self) -> Vec<String> {
        if let Some(ref exts) = self.extensions {
            exts.split(',')
                .map(|s| format!(".{}", s.trim().trim_start_matches('.')))
                .collect()
        } else {
            vec![
                ".pdf".to_string(),
                ".epub".to_string(),
                ".txt".to_string(),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_extensions() {
        let args = Args {
            path: PathBuf::from("."),
            dry_run: false,
            max_depth: 0,
            no_recursive: false,
            extensions: None,
            no_delete: false,
            todo_file: None,
            log_file: None,
            preserve_unicode: false,
            fetch_arxiv: false,
            verbose: false,
            delete_small: false,
            clean_failed: false,
            json: false,
            skip_cloud_hash: false,
            cleanup_downloads: false,
            cloud_provider: None,
            cloud_secret: None,
        };

        let exts = args.get_extensions();
        assert_eq!(exts.len(), 3);
        assert!(exts.contains(&".pdf".to_string()));
        assert!(exts.contains(&".epub".to_string()));
        assert!(exts.contains(&".txt".to_string()));
    }

    #[test]
    fn test_custom_extensions() {
        let args = Args {
            path: PathBuf::from("."),
            dry_run: false,
            max_depth: 0,
            no_recursive: false,
            extensions: Some("mobi, azw3".to_string()),
            no_delete: false,
            todo_file: None,
            log_file: None,
            preserve_unicode: false,
            fetch_arxiv: false,
            verbose: false,
            delete_small: false,
            clean_failed: false,
            json: false,
            skip_cloud_hash: false,
            cleanup_downloads: false,
            cloud_provider: None,
            cloud_secret: None,
        };

        let exts = args.get_extensions();
        assert_eq!(exts.len(), 2);
        assert!(exts.contains(&".mobi".to_string()));
        assert!(exts.contains(&".azw3".to_string()));
    }

    #[test]
    fn test_custom_extensions_with_dots() {
        let args = Args {
            path: PathBuf::from("."),
            dry_run: false,
            max_depth: 0,
            no_recursive: false,
            extensions: Some(".mobi, .azw3".to_string()),
            no_delete: false,
            todo_file: None,
            log_file: None,
            preserve_unicode: false,
            fetch_arxiv: false,
            verbose: false,
            delete_small: false,
            clean_failed: false,
            json: false,
            skip_cloud_hash: false,
            cleanup_downloads: false,
            cloud_provider: None,
            cloud_secret: None,
        };

        let exts = args.get_extensions();
        assert_eq!(exts.len(), 2);
        assert!(exts.contains(&".mobi".to_string()));
        assert!(exts.contains(&".azw3".to_string()));
    }
}
