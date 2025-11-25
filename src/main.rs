mod cli;
mod download_recovery;
mod duplicates;
mod json_output;
mod normalizer;
mod scanner;
mod todo;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use colored::*;
use download_recovery::DownloadRecovery;
use log::{info, warn};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum CleanupReason {
    FailedDownload,
    TooSmall,
    CorruptedPdf,
}

impl CleanupReason {
    fn slug(&self) -> &'static str {
        match self {
            CleanupReason::FailedDownload => "failed_download",
            CleanupReason::TooSmall => "too_small",
            CleanupReason::CorruptedPdf => "corrupted_pdf",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            CleanupReason::FailedDownload => "未完成下载",
            CleanupReason::TooSmall => "异常小文件",
            CleanupReason::CorruptedPdf => "损坏的 PDF",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            CleanupReason::FailedDownload => "🚫",
            CleanupReason::TooSmall => "📉",
            CleanupReason::CorruptedPdf => "⚠️",
        }
    }
}

#[derive(Clone)]
struct CleanupTarget {
    path: PathBuf,
    reason: CleanupReason,
}

impl CleanupTarget {
    fn new(path: PathBuf, reason: CleanupReason) -> Self {
        Self { path, reason }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let args = Args::parse();
    info!("Starting ebook renamer with args: {:?}", args);

    // Handle --fetch-arxiv placeholder
    if args.fetch_arxiv {
        println!(
            "{} {}",
            "⚠️  Warning:".yellow().bold(),
            "--fetch-arxiv is not implemented yet. Files will be processed offline only.".yellow()
        );
    }

    // Step 1: Recover downloads from .download/.crdownload folders
    let recovery = DownloadRecovery::new(&args.path, args.cleanup_downloads);
    let recovery_result = recovery.recover_downloads()?;

    if !recovery_result.extracted_files.is_empty() {
        info!(
            "Recovered {} PDFs from download folders",
            recovery_result.extracted_files.len()
        );
        if args.dry_run && !args.json {
            println!(
                "{} Recovered {} PDFs from download folders",
                "✓".green().bold(),
                recovery_result.extracted_files.len().to_string().cyan()
            );
        }
    }

    if !recovery_result.errors.is_empty() {
        info!(
            "Encountered {} errors during download recovery",
            recovery_result.errors.len()
        );
        if args.dry_run && !args.json {
            for error in &recovery_result.errors {
                println!("{}  {}", "⚠️".yellow(), error.yellow());
            }
        }
    }

    // Handle --no-recursive by setting max_depth to 1
    let effective_max_depth = if args.no_recursive { 1 } else { args.max_depth };

    let mut scanner = scanner::Scanner::new(&args.path, effective_max_depth)?;
    let files = scanner.scan()?;
    info!("Found {} files to process", files.len());

    // Parse and normalize filenames
    let normalized = normalizer::normalize_files(files)?;
    info!("Normalized {} files", normalized.len());

    // Handle failed downloads and small files
    let mut todo_list = todo::TodoList::new(&args.todo_file, &args.path)?;
    let mut cleanup_targets: Vec<CleanupTarget> = Vec::new();
    let mut todo_items = Vec::new();

    for file_info in &normalized {
        // Add existing failed/too small files
        if file_info.is_failed_download || file_info.is_too_small {
            let reason = if file_info.is_failed_download {
                CleanupReason::FailedDownload
            } else {
                CleanupReason::TooSmall
            };

            cleanup_targets.push(CleanupTarget::new(file_info.original_path.clone(), reason));

            if args.delete_small {
                // Remove this file from todo list since user opted to delete silently
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else {
                todo_list.add_failed_download(file_info)?;
                // Collect todo item for JSON output
                let category = reason.slug();
                let message = if file_info.is_failed_download {
                    format!("重新下载: {} (未完成下载)", file_info.original_name)
                } else {
                    format!(
                        "检查并重新下载: {} (文件过小，仅 {} 字节)",
                        file_info.original_name, file_info.size
                    )
                };
                todo_items.push((
                    category.to_string(),
                    file_info.original_name.clone(),
                    message,
                ));
            }

            continue;
        }

        // Analyze file integrity for all other files
        if let Some(issue) = todo_list.analyze_file_integrity(file_info)? {
            if matches!(issue, todo::FileIssue::CorruptedPdf) {
                cleanup_targets.push(CleanupTarget::new(
                    file_info.original_path.clone(),
                    CleanupReason::CorruptedPdf,
                ));
            }
        }
    }

    // Detect duplicates (skip if cloud storage mode)
    let (duplicate_groups, clean_files) =
        duplicates::detect_duplicates(normalized, args.skip_cloud_hash)?;
    if args.skip_cloud_hash {
        info!("Skipped duplicate detection (cloud storage mode)");
    } else {
        info!("Detected {} duplicate groups", duplicate_groups.len());
    }

    // Show or execute renames
    if args.dry_run {
        if args.json {
            // Output JSON format
            let cleanup_for_json: Vec<(PathBuf, String)> = cleanup_targets
                .iter()
                .map(|entry| (entry.path.clone(), entry.reason.slug().to_string()))
                .collect();
            let operations = json_output::OperationsOutput::from_results(
                clean_files,
                duplicate_groups,
                cleanup_for_json,
                todo_items,
                &args.path,
            )?;
            println!("{}", operations.to_json()?);
        } else {
            // Human-readable output with rich text
            println!("\n{}", "═══ DRY RUN MODE ═══".bold().bright_blue());

            if !clean_files.is_empty() {
                let mut rename_count = 0;
                for file_info in &clean_files {
                    if let Some(ref new_name) = file_info.new_name {
                        println!(
                            "{} {} {} {}",
                            "RENAME:".green().bold(),
                            file_info.original_name.bright_white(),
                            "→".bright_blue().bold(),
                            new_name.bright_cyan()
                        );
                        rename_count += 1;
                    }
                }
                if rename_count > 0 {
                    println!(
                        "\n{} {} files to rename",
                        "📝".bright_white(),
                        rename_count.to_string().bright_cyan().bold()
                    );
                }
            }

            for group in &duplicate_groups {
                if group.len() > 1 {
                    println!("\n{}", "🔍 DUPLICATE GROUP:".yellow().bold());
                    for (idx, path) in group.iter().enumerate() {
                        if idx == 0 {
                            println!(
                                "  {} {}",
                                "KEEP:".bright_blue().bold(),
                                path.display().to_string().bright_white()
                            );
                        } else {
                            println!(
                                "  {} {}",
                                "DELETE:".red().bold(),
                                path.display().to_string().bright_black()
                            );
                        }
                    }
                }
            }

            if !cleanup_targets.is_empty() {
                println!("\n{}", "🧹 待清理文件:".red().bold());
                for entry in &cleanup_targets {
                    let label = format!("[{}]", entry.reason.label()).bright_yellow();
                    println!(
                        "  {} {} {}",
                        "DELETE:".red().bold(),
                        label,
                        prettify_path(&entry.path, &args.path).bright_black()
                    );
                }

                let summary = summarize_cleanup(&cleanup_targets);
                println!("\n{} {}", "ℹ️".bright_blue(), "清理概要：".bright_white());
                for (reason, count) in summary {
                    println!(
                        "  {} {} {}",
                        reason.emoji(),
                        reason.label().bright_white(),
                        format!("{} 个", count).bright_cyan()
                    );
                }
            }

            if !todo_list.items.is_empty() {
                println!("\n{}", "📋 TODO LIST:".yellow().bold());
                for item in &todo_list.items {
                    println!("  {} {}", "- [ ]".bright_yellow(), item.bright_white());
                }
            }
        }

        // Write todo.md even in dry-run mode (as requested)
        todo_list.write()?;
        if !args.json {
            println!("\n{} todo.md written (dry-run mode)", "✓".green().bold());
        }
    } else {
        // Execute renames
        for file_info in &clean_files {
            if let Some(ref new_name) = file_info.new_name {
                std::fs::rename(&file_info.original_path, &file_info.new_path)?;
                info!("Renamed: {} -> {}", file_info.original_name, new_name);
            }
        }

        // Delete duplicates
        if !args.no_delete {
            for group in &duplicate_groups {
                if group.len() > 1 {
                    for (idx, path) in group.iter().enumerate() {
                        if idx > 0 {
                            std::fs::remove_file(path)?;
                            info!("Deleted duplicate: {}", path.display());
                        }
                    }
                }
            }
        }

        // Cleanup failed/invalid files
        if !cleanup_targets.is_empty() {
            if !args.json {
                println!(
                    "\n{} {} 个问题文件需要清理...",
                    "🧹".bright_white(),
                    cleanup_targets.len().to_string().red().bold()
                );
            }
            for entry in &cleanup_targets {
                match std::fs::remove_file(&entry.path) {
                    Ok(_) => {
                        info!("Cleaned {:?} ({})", entry.path, entry.reason.slug());
                        if !args.json {
                            println!(
                                "  {} {} {}",
                                "Removed:".green().bold(),
                                format!("[{}]", entry.reason.label()).bright_yellow(),
                                prettify_path(&entry.path, &args.path).bright_black()
                            );
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Failed to remove {:?} ({}): {}",
                            entry.path,
                            entry.reason.slug(),
                            err
                        );
                        if !args.json {
                            println!(
                                "  {} 无法删除 {} ({})",
                                "⚠️".yellow(),
                                prettify_path(&entry.path, &args.path).bright_black(),
                                err
                            );
                        }
                    }
                }
            }
        }

        // Write todo.md
        todo_list.write()?;
        info!("Wrote todo.md");
    }

    if !args.json {
        println!(
            "\n{} {}",
            "✓".green().bold(),
            "Operation completed successfully!".bright_green().bold()
        );
    }
    Ok(())
}

fn prettify_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn summarize_cleanup(targets: &[CleanupTarget]) -> BTreeMap<CleanupReason, usize> {
    let mut map = BTreeMap::new();
    for entry in targets {
        *map.entry(entry.reason).or_insert(0) += 1;
    }
    map
}
