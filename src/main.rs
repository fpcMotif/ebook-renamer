mod scanner;
mod normalizer;
mod duplicates;
mod todo;
mod cli;
mod json_output;
mod download_recovery;
mod tui;
mod cloud;
mod renamer;
mod security;


use anyhow::Result;
use clap::Parser;
use cli::Args;
use log::info;
use download_recovery::DownloadRecovery;
use colored::*;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let mut args = Args::parse();
    info!("Starting ebook renamer with args: {:?}", args);

    // Auto-detect cloud storage and enable skip_cloud_hash if not explicitly set
    if !args.skip_cloud_hash {
        if let Some(provider) = cloud::is_cloud_storage_path(&args.path) {
            if !args.json {
                println!("{}", cloud::cloud_mode_warning(provider).yellow());
            }
            args.skip_cloud_hash = true;
            info!("Auto-enabled cloud mode for {} storage", provider.name());
        }
    } else {
        // User explicitly enabled cloud mode
        if !args.json {
            println!("{}", "⚠️  Cloud mode enabled: Using metadata-only duplicate detection.".yellow());
            println!("{}", "   Duplicate detection based on filename similarity (≥85%) + exact size match.".yellow());
        }
    }

    // Handle --fetch-arxiv placeholder
    if args.fetch_arxiv {
        println!("{} {}",
            "⚠️  Warning:".yellow().bold(),
            "--fetch-arxiv is not implemented yet. Files will be processed offline only.".yellow()
        );
    }

    if !args.json {
        return tui::run(args).map_err(|e| anyhow::anyhow!(e));
    }

    // Step 1: Recover downloads from .download/.crdownload folders
    let recovery = DownloadRecovery::new(&args.path, args.cleanup_downloads);
    let recovery_result = recovery.recover_downloads()?;
    
    if !recovery_result.extracted_files.is_empty() {
        info!("Recovered {} PDFs from download folders", recovery_result.extracted_files.len());
        if args.dry_run && !args.json {
            println!("{} Recovered {} PDFs from download folders", 
                "✓".green().bold(),
                recovery_result.extracted_files.len().to_string().cyan()
            );
        }
    }
    
    if !recovery_result.errors.is_empty() {
        info!("Encountered {} errors during download recovery", recovery_result.errors.len());
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
    let mut files_to_delete = Vec::new();
    let mut todo_items = Vec::new();
    
    for file_info in &normalized {
        // Add existing failed/too small files
        if file_info.is_failed_download || file_info.is_too_small {
            if args.delete_small {
                files_to_delete.push(file_info.original_path.clone());
                // Remove this file from todo list since we're deleting it
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else if args.clean_failed {
                // Log AND Delete
                todo_list.add_failed_download(file_info)?;
                files_to_delete.push(file_info.original_path.clone());

                // Collect todo item for JSON output
                let category = if file_info.is_failed_download { "failed_download" } else { "too_small" };
                let message = if file_info.is_failed_download {
                    format!("Redownload: {} (Unfinished download)", file_info.original_name)
                } else {
                    format!("Check and redownload: {} (File too small, only {} bytes)", file_info.original_name, file_info.size)
                };
                todo_items.push((category.to_string(), file_info.original_name.clone(), message));
            } else {
                todo_list.add_failed_download(file_info)?;
                // Collect todo item for JSON output
                let category = if file_info.is_failed_download { "failed_download" } else { "too_small" };
                let message = if file_info.is_failed_download {
                    format!("Redownload: {} (Unfinished download)", file_info.original_name)
                } else {
                    format!("Check and redownload: {} (File too small, only {} bytes)", file_info.original_name, file_info.size)
                };
                todo_items.push((category.to_string(), file_info.original_name.clone(), message));
            }
        } else {
            // Analyze file integrity for all other files
            todo_list.analyze_file_integrity(file_info)?;
        }
    }

    // Detect duplicates (skip if cloud storage mode)
    let (duplicate_groups, mut clean_files, hash_errors) =
        duplicates::detect_duplicates(normalized, args.skip_cloud_hash)?;

    // Ensure no two files attempt to rename to the same destination (or overwrite an existing file)
    renamer::resolve_rename_collisions(&mut clean_files)?;

    // Process hash errors
    if !hash_errors.is_empty() {
        info!("Found {} files with hash computation errors", hash_errors.len());
        for (file_info, error) in &hash_errors {
            todo_list.add_file_issue(file_info, todo::FileIssue::ReadError(error.clone()))?;
            // Add to todo_items for JSON output
            todo_items.push((
                "read_error".to_string(),
                file_info.original_name.clone(),
                format!("Check file permission: {} (Read error: {})", file_info.original_name, error)
            ));
        }
    }

    if args.skip_cloud_hash {
        info!("Skipped duplicate detection (cloud storage mode)");
    } else {
        info!("Detected {} duplicate groups", duplicate_groups.len());
    }

    // Show or execute renames
    if args.dry_run {
        if args.json {
            // Output JSON format
            let operations = json_output::OperationsOutput::from_results(
                clean_files,
                duplicate_groups,
                files_to_delete,
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
                        println!("{} {} {} {}", 
                            "RENAME:".green().bold(),
                            file_info.original_name.bright_white(),
                            "→".bright_blue().bold(),
                            new_name.bright_cyan()
                        );
                        rename_count += 1;
                    }
                }
                if rename_count > 0 {
                    println!("\n{} {} files to rename", 
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
                            println!("  {} {}", 
                                "KEEP:".bright_blue().bold(),
                                path.display().to_string().bright_white()
                            );
                        } else {
                            println!("  {} {}", 
                                "DELETE:".red().bold(),
                                path.display().to_string().bright_black()
                            );
                        }
                    }
                }
            }

            if !files_to_delete.is_empty() {
                println!("\n{}", "🗑️  SMALL/CORRUPTED/FAILED FILES TO DELETE:".red().bold());
                for path in &files_to_delete {
                    println!("  {} {}", 
                        "DELETE:".red().bold(),
                        path.display().to_string().bright_black()
                    );
                }
            }
            
            if !todo_list.items.is_empty() {
                println!("\n{}", "📋 TODO LIST:".yellow().bold());
                for item in &todo_list.items {
                    println!("  {} {}", 
                        "- [ ]".bright_yellow(),
                        item.bright_white()
                    );
                }
            }
        }
        
        // Write todo.md even in dry-run mode (as requested)
        todo_list.write()?;
        if !args.json {
            println!("\n{} todo.md written (dry-run mode)", "✓".green().bold());
        }
    } else {
        // Initialize undo log if requested
        let mut undo_log = json_output::UndoLog::new(&args.path, false);

        // Execute renames (cycle-safe)
        renamer::execute_renames(&clean_files)?;

        // Log renames to undo log
        for file_info in &clean_files {
            if file_info.new_name.is_some() && file_info.original_path != file_info.new_path {
                undo_log.add_rename(&file_info.original_path, &file_info.new_path);
            }
        }

        // Delete duplicates
        if !args.no_delete {
            for group in &duplicate_groups {
                if group.len() > 1 {
                    for (idx, path) in group.iter().enumerate() {
                        if idx > 0 {
                            // Get file size before deletion for undo log
                            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                            std::fs::remove_file(path)?;
                            info!("Deleted duplicate: {}", path.display());
                            undo_log.add_deleted_duplicate(path, size, None);
                        }
                    }
                }
            }
        }

        // Delete small/corrupted/failed files if requested
        if (args.delete_small || args.clean_failed) && !files_to_delete.is_empty() {
            println!("\n{} {} small/corrupted/failed files...",
                "🗑️".bright_white(),
                files_to_delete.len().to_string().red().bold()
            );
            for path in &files_to_delete {
                // Get file size before deletion for undo log
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                std::fs::remove_file(path)?;
                info!("Deleted small/corrupted/failed file: {}", path.display());
                println!("  {} {}",
                    "Deleted:".red().bold(),
                    path.display().to_string().bright_black()
                );
                undo_log.add_deleted_small(path, size);
            }
        }

        // Write undo log if path specified
        if let Some(ref undo_log_path) = args.undo_log {
            undo_log.write_to_file(undo_log_path)?;
            info!("Wrote undo log to {:?}", undo_log_path);
            if !args.json {
                println!("{} Undo log written to: {}",
                    "✓".green().bold(),
                    undo_log_path.display().to_string().bright_cyan()
                );
            }
        }

        // Write todo.md
        todo_list.write()?;
        info!("Wrote todo.md");
    }

    if !args.json {
        println!("\n{} {}", 
            "✓".green().bold(),
            "Operation completed successfully!".bright_green().bold()
        );
    }
    Ok(())
}
