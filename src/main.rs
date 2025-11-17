mod scanner;
mod normalizer;
mod duplicates;
mod todo;
mod cli;
mod json_output;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use log::info;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let args = Args::parse();
    info!("Starting ebook renamer with args: {:?}", args);

    // Handle --fetch-arxiv placeholder
    if args.fetch_arxiv {
        println!("⚠️  Warning: --fetch-arxiv is not implemented yet. Files will be processed offline only.");
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
            } else {
                todo_list.add_failed_download(file_info)?;
                // Collect todo item for JSON output
                let category = if file_info.is_failed_download { "failed_download" } else { "too_small" };
                let message = if file_info.is_failed_download {
                    format!("重新下载: {} (未完成下载)", file_info.original_name)
                } else {
                    format!("检查并重新下载: {} (文件过小，仅 {} 字节)", file_info.original_name, file_info.size)
                };
                todo_items.push((category.to_string(), file_info.original_name.clone(), message));
            }
        } else {
            // Analyze file integrity for all other files
            todo_list.analyze_file_integrity(file_info)?;
        }
    }

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
            let operations = json_output::OperationsOutput::from_results(
                clean_files,
                duplicate_groups,
                files_to_delete,
                todo_items,
                &args.path,
            )?;
            println!("{}", operations.to_json()?);
        } else {
            // Human-readable output
            println!("\n=== DRY RUN MODE ===");
            for file_info in &clean_files {
                if let Some(ref new_name) = file_info.new_name {
                    println!("RENAME: {} -> {}", file_info.original_name, new_name);
                }
            }
            
            for group in &duplicate_groups {
                if group.len() > 1 {
                    println!("\nDELETE DUPLICATES:");
                    for (idx, path) in group.iter().enumerate() {
                        if idx == 0 {
                            println!("  KEEP: {}", path.display());
                        } else {
                            println!("  DELETE: {}", path.display());
                        }
                    }
                }
            }

            if !files_to_delete.is_empty() {
                println!("\nDELETE SMALL/CORRUPTED FILES:");
                for path in &files_to_delete {
                    println!("  DELETE: {}", path.display());
                }
            }
            
            if !todo_list.items.is_empty() {
                println!("\nTODO LIST:");
                for item in &todo_list.items {
                    println!("  - [ ] {}", item);
                }
            }
        }
        
        // Write todo.md even in dry-run mode (as requested)
        todo_list.write()?;
        if !args.json {
            println!("\n✓ todo.md written (dry-run mode)");
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

        // Delete small/corrupted files if requested
        if args.delete_small && !files_to_delete.is_empty() {
            println!("\nDeleting {} small/corrupted files...", files_to_delete.len());
            for path in &files_to_delete {
                std::fs::remove_file(path)?;
                info!("Deleted small/corrupted file: {}", path.display());
                println!("  Deleted: {}", path.display());
            }
        }

        // Write todo.md
        todo_list.write()?;
        info!("Wrote todo.md");
    }

    if !args.json {
        println!("\n✓ Operation completed successfully!");
    }
    Ok(())
}
