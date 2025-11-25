mod scanner;
mod normalizer;
mod duplicates;
mod todo;
mod cli;
mod json_output;
mod download_recovery;
mod cleanup;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use log::info;
use download_recovery::DownloadRecovery;
use colored::*;
use cleanup::CleanupPlan;
use scanner::FileInfo;
use std::fs;
use std::io::Read;

/// Validate PDF file integrity by checking header
fn validate_pdf_integrity(file_info: &FileInfo) -> Result<()> {
    if file_info.extension.to_lowercase() != ".pdf" {
        return Ok(());
    }

    let mut file = fs::File::open(&file_info.original_path)?;
    let mut header = [0u8; 5];
    file.read_exact(&mut header)?;
    
    // PDF files should start with "%PDF-"
    if &header != b"%PDF-" {
        return Err(anyhow::anyhow!("Invalid PDF header"));
    }
    
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let args = Args::parse();
    info!("Starting ebook renamer with args: {:?}", args);

    // Handle --fetch-arxiv placeholder
    if args.fetch_arxiv {
        println!("{} {}", 
            "⚠️  Warning:".yellow().bold(),
            "--fetch-arxiv is not implemented yet. Files will be processed offline only.".yellow()
        );
    }

    // Step 1: Recover downloads from .download/.crdownload folders
    // Enable cleanup if auto_cleanup is set
    let enable_folder_cleanup = args.auto_cleanup || args.cleanup_downloads;
    let recovery = DownloadRecovery::new(&args.path, enable_folder_cleanup);
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

    // Determine cleanup behavior based on flags
    let should_cleanup_files = args.auto_cleanup || args.delete_small;
    let should_prompt = args.interactive && !args.yes && !args.dry_run;

    // Handle failed downloads and small files
    let mut todo_list = todo::TodoList::new(&args.todo_file, &args.path)?;
    let mut cleanup_plan = CleanupPlan::new();
    let mut todo_items = Vec::new();
    
    for file_info in &normalized {
        // Categorize problem files
        if file_info.is_failed_download {
            if should_cleanup_files {
                cleanup_plan.failed_downloads.push(file_info.original_path.clone());
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else {
                todo_list.add_failed_download(file_info)?;
                let message = format!("重新下载: {} (未完成下载)", file_info.original_name);
                todo_items.push(("failed_download".to_string(), file_info.original_name.clone(), message));
            }
        } else if file_info.is_too_small {
            if should_cleanup_files {
                cleanup_plan.small_files.push(file_info.original_path.clone());
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else {
                todo_list.add_failed_download(file_info)?;
                let message = format!("检查并重新下载: {} (文件过小，仅 {} 字节)", file_info.original_name, file_info.size);
                todo_items.push(("too_small".to_string(), file_info.original_name.clone(), message));
            }
        } else {
            // Analyze file integrity for all other files
            if let Err(_) = validate_pdf_integrity(file_info) {
                if should_cleanup_files {
                    cleanup_plan.corrupted_files.push(file_info.original_path.clone());
                    todo_list.remove_file_from_todo(&file_info.original_name);
                } else {
                    todo_list.analyze_file_integrity(file_info)?;
                }
            } else {
                todo_list.analyze_file_integrity(file_info)?;
            }
        }
    }

    // Note: download folders are already cleaned by download_recovery module
    // We don't need to add them to cleanup_plan again

    // Detect duplicates (skip if cloud storage mode)
    let (duplicate_groups, clean_files) = duplicates::detect_duplicates(normalized, args.skip_cloud_hash)?;
    if args.skip_cloud_hash {
        info!("Skipped duplicate detection (cloud storage mode)");
    } else {
        info!("Detected {} duplicate groups", duplicate_groups.len());
    }

    // Show or execute renames
    if args.dry_run {
        if args.json {
            // Output JSON format
            let files_to_delete: Vec<_> = cleanup_plan.small_files.iter()
                .chain(cleanup_plan.corrupted_files.iter())
                .chain(cleanup_plan.failed_downloads.iter())
                .cloned()
                .collect();
            
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

            // Display cleanup plan in dry-run mode
            if !cleanup_plan.is_empty() {
                println!("\n{}", "═══ 清理计划（DRY RUN）═══".bold().yellow());
                cleanup_plan.display_detailed();
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

        // Execute cleanup if needed
        if !cleanup_plan.is_empty() {
            // Show cleanup plan
            if !args.json {
                cleanup_plan.display_summary();
            }

            // Prompt for confirmation if interactive mode
            let should_proceed = if should_prompt {
                cleanup::prompt_confirmation(&cleanup_plan)?
            } else {
                true
            };

            if should_proceed {
                if !args.json {
                    println!("\n{} 正在清理文件...", "🧹".bright_white());
                }
                
                let cleanup_result = cleanup::execute_cleanup(&cleanup_plan)?;
                
                if !args.json {
                    cleanup_result.display();
                }
                
                info!(
                    "Cleanup completed: {} files deleted, {} folders removed, {} errors",
                    cleanup_result.deleted_files,
                    cleanup_result.deleted_folders,
                    cleanup_result.errors.len()
                );
            } else {
                if !args.json {
                    println!("\n{} 清理操作已取消", "ℹ️".bright_blue());
                }
                info!("Cleanup cancelled by user");
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
