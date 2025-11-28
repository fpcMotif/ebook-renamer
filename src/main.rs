mod scanner;
mod normalizer;
mod duplicates;
mod todo;
mod cli;
mod json_output;
mod download_recovery;
mod tui;
mod cloud;

use anyhow::{Result, anyhow};
use clap::Parser;
use cli::Args;
use log::{info, warn};
use download_recovery::DownloadRecovery;
use colored::*;
use crate::cloud::{CloudFile, CloudProvider, dropbox::DropboxProvider, gdrive::GDriveProvider};

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let args = Args::parse();
    info!("Starting ebook renamer with args: {:?}", args);

    // Handle Cloud Mode
    if let Some(ref provider_name) = args.cloud_provider {
        return run_cloud_mode(&args, provider_name);
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

        // Delete small/corrupted/failed files if requested
        if (args.delete_small || args.clean_failed) && !files_to_delete.is_empty() {
            println!("\n{} {} small/corrupted/failed files...",
                "🗑️".bright_white(),
                files_to_delete.len().to_string().red().bold()
            );
            for path in &files_to_delete {
                if !args.dry_run {
                    std::fs::remove_file(path)?;
                    info!("Deleted small/corrupted/failed file: {}", path.display());
                    println!("  {} {}",
                        "Deleted:".red().bold(),
                        path.display().to_string().bright_black()
                    );
                }
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

fn run_cloud_mode(args: &Args, provider_name: &str) -> Result<()> {
    println!("{}", format!("☁️  Running in Cloud Mode: {}", provider_name).blue().bold());

    let token = args.cloud_secret.clone().or_else(|| {
        match provider_name {
            "dropbox" => std::env::var("DROPBOX_ACCESS_TOKEN").ok(),
            "gdrive" => std::env::var("GDRIVE_ACCESS_TOKEN").ok(), // Simplified for now, usually needs JSON creds
            _ => None
        }
    }).ok_or_else(|| anyhow!("No credentials found. Provide --cloud-secret or set env vars."))?;

    let provider: Box<dyn CloudProvider> = match provider_name {
        "dropbox" => Box::new(DropboxProvider::new(token)),
        "gdrive" => Box::new(GDriveProvider::new(token)),
        _ => return Err(anyhow!("Unknown cloud provider: {}", provider_name)),
    };

    println!("Scanning files in {}...", args.path.display());
    let cloud_files = provider.list_files(args.path.to_str().unwrap_or("."))?;
    info!("Found {} files in cloud", cloud_files.len());

    // Create map for hash lookup
    let mut path_to_hash = std::collections::HashMap::new();
    for cf in &cloud_files {
        if let Some(ref h) = cf.hash {
            path_to_hash.insert(cf.path.clone(), h.clone());
            // Also map ID if different (for GDrive)
            if cf.id != cf.path {
                path_to_hash.insert(cf.id.clone(), h.clone());
            }
        }
    }

    let mut file_infos: Vec<scanner::FileInfo> = cloud_files.iter().map(|cf| cf.to_file_info()).collect();

    // Filter by extensions
    let allowed_extensions = args.get_extensions();
    file_infos.retain(|f| allowed_extensions.contains(&f.extension));
    info!("Filtered to {} files based on extensions", file_infos.len());

    // Normalize
    let normalized = normalizer::normalize_files(file_infos)?;

    // Detect Duplicates (using hash if available, else relying on filename)

    let mut seen_names = std::collections::HashMap::new();
    let mut seen_target_names = std::collections::HashSet::new();

    // Populate seen_target_names with existing filenames in cloud to detect collisions
    for file in &normalized {
        seen_target_names.insert(file.original_name.to_lowercase());
    }

    let mut duplicates: Vec<(&scanner::FileInfo, String)> = Vec::new(); // Store file and new unique name
    let mut to_rename: Vec<(&scanner::FileInfo, String)> = Vec::new(); // Store file and new name

    for file in &normalized {
        let file_hash = path_to_hash.get(&file.original_path.to_string_lossy().to_string());

        let key = if !args.skip_cloud_hash && file_hash.is_some() {
             // Use Content Hash if available and not skipped
             format!("hash::{}", file_hash.unwrap())
        } else {
             // Fallback to Filename (as per user request snippet)
             file.original_name.to_lowercase()
        };

        let is_duplicate = seen_names.contains_key(&key);

        if is_duplicate {
             // Active Deduplication: Rename with suffix
             let base_name = std::path::Path::new(&file.original_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
             let ext = &file.extension;

             let mut unique_name = format!("{}_unique{}", base_name, ext);
             let mut counter = 1;
             while seen_target_names.contains(&unique_name.to_lowercase()) {
                 unique_name = format!("{}_unique_{}{}", base_name, counter, ext);
                 counter += 1;
             }

             seen_target_names.insert(unique_name.to_lowercase());
             duplicates.push((file, unique_name));

        } else {
            seen_names.insert(key, file.original_path.clone());

            if let Some(target_name) = &file.new_name {
                let mut final_target_name = target_name.clone();

                // Check if this rename targets an existing file (Collision)
                // Note: file.original_name is in seen_target_names, so we check if target != original
                if final_target_name.to_lowercase() != file.original_name.to_lowercase() && seen_target_names.contains(&final_target_name.to_lowercase()) {
                     // Collision detected, append suffix
                     let base_name = std::path::Path::new(target_name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                     let ext = &file.extension; // normalization usually preserves extension

                     let mut counter = 2;
                     while seen_target_names.contains(&final_target_name.to_lowercase()) {
                         final_target_name = format!("{} ({}){}", base_name, counter, ext);
                         counter += 1;
                     }
                }

                if final_target_name != file.original_name {
                    seen_target_names.insert(final_target_name.to_lowercase());
                    to_rename.push((file, final_target_name));
                }
            }
        }
    }

    if args.dry_run {
         println!("\n{}", "═══ DRY RUN MODE (CLOUD) ═══".bold().bright_blue());
         for (file, new_name) in &to_rename {
             println!("{} {} {} {}",
                "RENAME:".green().bold(),
                file.original_name.bright_white(),
                "→".bright_blue().bold(),
                new_name.bright_cyan()
            );
         }
         for (file, unique_name) in &duplicates {
             println!("{} {} {} {}",
                "DUPLICATE (RENAME):".yellow().bold(),
                file.original_name.bright_white(),
                "→".bright_blue().bold(),
                unique_name.bright_cyan()
             );
         }
    } else {
        // Combine all operations
        let mut all_ops = to_rename;
        all_ops.extend(duplicates);

        for (file, new_name) in all_ops {
             // We need to map FileInfo back to CloudFile id to rename?
             // FileInfo.original_path holds the path/id.
             // We can reconstruct a temporary CloudFile or adjust provider signature.
             // Provider expects CloudFile.
             let cf = CloudFile {
                 id: file.original_path.to_string_lossy().to_string(), // For GDrive, path is ID. For Dropbox, it's path.
                 path: file.original_path.to_string_lossy().to_string(),
                 name: file.original_name.clone(),
                 hash: None,
                 size: file.size,
                 modified_time: file.modified_time,
                 provider: provider_name.to_string(),
             };

             match provider.rename_file(&cf, &new_name) {
                 Ok(_) => info!("Renamed {} to {}", file.original_name, new_name),
                 Err(e) => warn!("Failed to rename {}: {}", file.original_name, e),
             }
        }
    }

    println!("\n{} {}",
            "✓".green().bold(),
            "Cloud operation completed!".bright_green().bold()
    );

    Ok(())
}
