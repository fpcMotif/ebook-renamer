mod scanner;
mod normalizer;
mod duplicates;
mod todo;
mod cli;
mod json_output;
mod download_recovery;

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
    // Automatically enable cleanup for better UX - users don't need to think about it
    let auto_cleanup = args.cleanup_downloads || !args.dry_run;
    let recovery = DownloadRecovery::new(&args.path, auto_cleanup);
    let recovery_result = recovery.recover_downloads()?;
    
    // Show recovery results with clear feedback
    if !args.json {
        if !recovery_result.extracted_files.is_empty() || !recovery_result.deleted_corrupted_files.is_empty() || !recovery_result.cleaned_folders.is_empty() {
            println!("\n{} {}", "📥 下载恢复与清理:".bright_cyan().bold(), "");
        }
        
        if !recovery_result.extracted_files.is_empty() {
            info!("Recovered {} PDFs from download folders", recovery_result.extracted_files.len());
            if args.dry_run {
                println!("  {} 从下载文件夹中恢复 {} 个 PDF 文件", 
                    "✓".green().bold(),
                    recovery_result.extracted_files.len().to_string().cyan()
                );
            } else {
                println!("  {} 已恢复 {} 个 PDF 文件", 
                    "✓".green().bold(),
                    recovery_result.extracted_files.len().to_string().cyan()
                );
            }
        }
        
        if !recovery_result.deleted_corrupted_files.is_empty() {
            info!("Deleted {} corrupted files during recovery", recovery_result.deleted_corrupted_files.len());
            if args.dry_run {
                println!("  {} 将删除 {} 个损坏的文件", 
                    "🗑️".yellow().bold(),
                    recovery_result.deleted_corrupted_files.len().to_string().yellow()
                );
            } else {
                println!("  {} 已删除 {} 个损坏的文件", 
                    "🗑️".red().bold(),
                    recovery_result.deleted_corrupted_files.len().to_string().red()
                );
            }
        }
        
        if !recovery_result.cleaned_folders.is_empty() {
            info!("Cleaned {} empty download folders", recovery_result.cleaned_folders.len());
            if args.dry_run {
                println!("  {} 将清理 {} 个空下载文件夹", 
                    "🧹".bright_blue().bold(),
                    recovery_result.cleaned_folders.len().to_string().bright_blue()
                );
            } else {
                println!("  {} 已清理 {} 个空下载文件夹", 
                    "🧹".bright_blue().bold(),
                    recovery_result.cleaned_folders.len().to_string().bright_blue()
                );
            }
        }
    }
    
    if !recovery_result.errors.is_empty() {
        info!("Encountered {} errors during download recovery", recovery_result.errors.len());
        if !args.json {
            for error in &recovery_result.errors {
                println!("  {}  {}", "⚠️".yellow(), error.yellow());
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

    // Handle failed downloads and small/corrupted files
    // Business logic: Automatically clean up obviously broken files for better UX
    // Users don't need to manually specify --delete-small for obvious cases
    let mut todo_list = todo::TodoList::new(&args.todo_file, &args.path)?;
    let mut files_to_delete = Vec::new();
    let mut todo_items = Vec::new();
    
    // Auto-delete policy: Delete obviously broken files automatically
    // This makes the UX more natural - users don't need to think about cleanup
    let auto_delete_broken = args.delete_small || !args.dry_run;
    
    for file_info in &normalized {
        // Handle failed downloads and small files
        if file_info.is_failed_download {
            // Failed downloads (.download/.crdownload files) should always be deleted
            // They're clearly incomplete and taking up space
            if auto_delete_broken {
                files_to_delete.push(file_info.original_path.clone());
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else {
                todo_list.add_failed_download(file_info)?;
                let message = format!("重新下载: {} (未完成下载)", file_info.original_name);
                todo_items.push(("failed_download".to_string(), file_info.original_name.clone(), message));
            }
        } else if file_info.is_too_small {
            // Very small files (< 1KB) are likely corrupted or incomplete
            // Auto-delete them unless user explicitly wants to keep them
            if auto_delete_broken {
                files_to_delete.push(file_info.original_path.clone());
                todo_list.remove_file_from_todo(&file_info.original_name);
            } else {
                todo_list.add_failed_download(file_info)?;
                let message = format!("检查并重新下载: {} (文件过小，仅 {} 字节)", file_info.original_name, file_info.size);
                todo_items.push(("too_small".to_string(), file_info.original_name.clone(), message));
            }
        } else {
            // Analyze file integrity for all other files
            // This will detect corrupted PDFs and add them to todo list
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

    // Calculate statistics before moving values
    let rename_count = clean_files.iter()
        .filter(|f| f.new_name.is_some())
        .count();
    let duplicate_count: usize = duplicate_groups.iter()
        .map(|g| if g.len() > 1 { g.len() - 1 } else { 0 })
        .sum();
    let files_to_delete_count = files_to_delete.len();

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
            
            if rename_count > 0 {
                for file_info in &clean_files {
                    if let Some(ref new_name) = file_info.new_name {
                        println!("{} {} {} {}", 
                            "RENAME:".green().bold(),
                            file_info.original_name.bright_white(),
                            "→".bright_blue().bold(),
                            new_name.bright_cyan()
                        );
                    }
                }
                println!("\n{} {} files to rename", 
                    "📝".bright_white(),
                    rename_count.to_string().bright_cyan().bold()
                );
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

            if files_to_delete_count > 0 {
                println!("\n{}", "🗑️  将删除的损坏/未完成文件:".red().bold());
                for path in &files_to_delete {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    println!("  {} {}", 
                        "DELETE:".red().bold(),
                        filename.bright_black()
                    );
                }
                println!("  {} 共 {} 个文件将被自动清理", 
                    "ℹ️".bright_blue(),
                    files_to_delete_count.to_string().bright_blue()
                );
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

        // Auto-delete broken files (natural business logic - clean up obviously broken files)
        if files_to_delete_count > 0 {
            if !args.json {
                println!("\n{} 正在清理 {} 个损坏/未完成的文件...", 
                    "🗑️".bright_white(),
                    files_to_delete_count.to_string().red().bold()
                );
            }
            for path in &files_to_delete {
                match std::fs::remove_file(path) {
                    Ok(_) => {
                        info!("Deleted broken file: {}", path.display());
                        if !args.json {
                            let filename = path.file_name()
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| path.display().to_string());
                            println!("  {} {}", 
                                "已删除:".red().bold(),
                                filename.bright_black()
                            );
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to delete {}: {}", path.display(), e);
                        info!("{}", error_msg);
                        if !args.json {
                            println!("  {} {}", 
                                "⚠️".yellow(),
                                error_msg.yellow()
                            );
                        }
                    }
                }
            }
            if !args.json {
                println!("  {} 清理完成", "✓".green().bold());
            }
        }

        // Write todo.md
        todo_list.write()?;
        info!("Wrote todo.md");
    }

    // Final summary with clear statistics
    if !args.json {
        println!("\n{}", "═══ 操作总结 ═══".bold().bright_green());
        
        let mut has_operations = false;
        
        // Recovery summary
        if !recovery_result.extracted_files.is_empty() || !recovery_result.deleted_corrupted_files.is_empty() {
            has_operations = true;
            if !recovery_result.extracted_files.is_empty() {
                println!("  {} 恢复文件: {}", "📥".bright_cyan(), recovery_result.extracted_files.len().to_string().bright_cyan());
            }
            if !recovery_result.deleted_corrupted_files.is_empty() {
                println!("  {} 清理损坏文件: {}", "🗑️".red(), recovery_result.deleted_corrupted_files.len().to_string().red());
            }
        }
        
        // Rename summary
        if rename_count > 0 {
            has_operations = true;
            println!("  {} 重命名文件: {}", "📝".bright_blue(), rename_count.to_string().bright_blue());
        }
        
        // Duplicate summary
        if duplicate_count > 0 {
            has_operations = true;
            println!("  {} 删除重复文件: {}", "🔍".yellow(), duplicate_count.to_string().yellow());
        }
        
        // Cleanup summary
        if files_to_delete_count > 0 {
            has_operations = true;
            if args.dry_run {
                println!("  {} 将清理损坏文件: {}", "🗑️".yellow(), files_to_delete_count.to_string().yellow());
            } else {
                println!("  {} 已清理损坏文件: {}", "🗑️".red(), files_to_delete_count.to_string().red());
            }
        }
        
        // Todo summary
        if !todo_list.items.is_empty() {
            has_operations = true;
            println!("  {} 待处理任务: {} (已保存到 todo.md)", 
                "📋".bright_yellow(), 
                todo_list.items.len().to_string().bright_yellow()
            );
        }
        
        if !has_operations {
            println!("  {} 没有需要处理的操作", "✓".green());
        }
        
        println!("\n{} {}", 
            "✓".green().bold(),
            "操作完成！".bright_green().bold()
        );
    }
    Ok(())
}
