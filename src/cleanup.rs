use anyhow::Result;
use colored::*;
use log::{info, debug};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// 清理计划，包含所有需要清理的文件和文件夹
#[derive(Debug, Clone)]
pub struct CleanupPlan {
    pub small_files: Vec<PathBuf>,
    pub corrupted_files: Vec<PathBuf>,
    pub failed_downloads: Vec<PathBuf>,
    pub download_folders: Vec<PathBuf>,
}

impl CleanupPlan {
    pub fn new() -> Self {
        Self {
            small_files: Vec::new(),
            corrupted_files: Vec::new(),
            failed_downloads: Vec::new(),
            download_folders: Vec::new(),
        }
    }

    /// 返回需要删除的文件总数
    pub fn total_files(&self) -> usize {
        self.small_files.len() + self.corrupted_files.len() + self.failed_downloads.len()
    }

    /// 返回需要删除的文件夹总数
    pub fn total_folders(&self) -> usize {
        self.download_folders.len()
    }

    /// 检查是否有任何需要清理的项目
    pub fn is_empty(&self) -> bool {
        self.total_files() == 0 && self.total_folders() == 0
    }

    /// 显示清理计划的摘要
    pub fn display_summary(&self) {
        if self.is_empty() {
            println!("{}", "✨ 没有发现需要清理的文件或文件夹".bright_green());
            return;
        }

        println!("\n{}", "═══ 清理计划摘要 ═══".bold().bright_blue());
        
        if !self.small_files.is_empty() {
            println!("\n{} {} 个异常小文件 (< 1KB):", 
                "📁".bright_white(), 
                self.small_files.len().to_string().yellow().bold()
            );
            for (i, path) in self.small_files.iter().enumerate() {
                if i < 5 {
                    println!("  {} {}", 
                        "•".yellow(), 
                        path.file_name().unwrap().to_string_lossy().bright_black()
                    );
                } else if i == 5 {
                    println!("  {} ... 还有 {} 个文件", 
                        "•".yellow(), 
                        (self.small_files.len() - 5).to_string().yellow()
                    );
                    break;
                }
            }
        }

        if !self.corrupted_files.is_empty() {
            println!("\n{} {} 个损坏的PDF文件:", 
                "🚨".bright_white(), 
                self.corrupted_files.len().to_string().red().bold()
            );
            for (i, path) in self.corrupted_files.iter().enumerate() {
                if i < 5 {
                    println!("  {} {}", 
                        "•".red(), 
                        path.file_name().unwrap().to_string_lossy().bright_black()
                    );
                } else if i == 5 {
                    println!("  {} ... 还有 {} 个文件", 
                        "•".red(), 
                        (self.corrupted_files.len() - 5).to_string().red()
                    );
                    break;
                }
            }
        }

        if !self.failed_downloads.is_empty() {
            println!("\n{} {} 个未完成下载文件:", 
                "🔄".bright_white(), 
                self.failed_downloads.len().to_string().yellow().bold()
            );
            for (i, path) in self.failed_downloads.iter().enumerate() {
                if i < 5 {
                    println!("  {} {}", 
                        "•".yellow(), 
                        path.file_name().unwrap().to_string_lossy().bright_black()
                    );
                } else if i == 5 {
                    println!("  {} ... 还有 {} 个文件", 
                        "•".yellow(), 
                        (self.failed_downloads.len() - 5).to_string().yellow()
                    );
                    break;
                }
            }
        }

        if !self.download_folders.is_empty() {
            println!("\n{} {} 个空下载文件夹:", 
                "📂".bright_white(), 
                self.download_folders.len().to_string().cyan().bold()
            );
            for (i, path) in self.download_folders.iter().enumerate() {
                if i < 5 {
                    println!("  {} {}", 
                        "•".cyan(), 
                        path.file_name().unwrap().to_string_lossy().bright_black()
                    );
                } else if i == 5 {
                    println!("  {} ... 还有 {} 个文件夹", 
                        "•".cyan(), 
                        (self.download_folders.len() - 5).to_string().cyan()
                    );
                    break;
                }
            }
        }

        println!("\n{} 总计: {} 个文件, {} 个文件夹", 
            "📊".bright_white(),
            self.total_files().to_string().yellow().bold(),
            self.total_folders().to_string().cyan().bold()
        );
    }

    /// 显示详细的清理计划
    pub fn display_detailed(&self) {
        if self.is_empty() {
            println!("{}", "✨ 没有发现需要清理的文件或文件夹".bright_green());
            return;
        }

        println!("\n{}", "═══ 详细清理列表 ═══".bold().bright_blue());

        if !self.small_files.is_empty() {
            println!("\n{} 异常小文件 (< 1KB):", "📁".bright_white());
            for path in &self.small_files {
                println!("  {} {}", 
                    "DELETE:".red().bold(), 
                    path.display().to_string().bright_black()
                );
            }
        }

        if !self.corrupted_files.is_empty() {
            println!("\n{} 损坏的PDF文件:", "🚨".bright_white());
            for path in &self.corrupted_files {
                println!("  {} {}", 
                    "DELETE:".red().bold(), 
                    path.display().to_string().bright_black()
                );
            }
        }

        if !self.failed_downloads.is_empty() {
            println!("\n{} 未完成下载文件:", "🔄".bright_white());
            for path in &self.failed_downloads {
                println!("  {} {}", 
                    "DELETE:".red().bold(), 
                    path.display().to_string().bright_black()
                );
            }
        }

        if !self.download_folders.is_empty() {
            println!("\n{} 空下载文件夹:", "📂".bright_white());
            for path in &self.download_folders {
                println!("  {} {}", 
                    "REMOVE:".cyan().bold(), 
                    path.display().to_string().bright_black()
                );
            }
        }
    }
}

/// 清理执行结果
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub deleted_files: usize,
    pub deleted_folders: usize,
    pub errors: Vec<String>,
}

impl CleanupResult {
    pub fn new() -> Self {
        Self {
            deleted_files: 0,
            deleted_folders: 0,
            errors: Vec::new(),
        }
    }

    pub fn display(&self) {
        println!("\n{}", "═══ 清理完成 ═══".bold().bright_green());
        println!("{} 已删除 {} 个文件", 
            "✓".green().bold(), 
            self.deleted_files.to_string().bright_cyan()
        );
        println!("{} 已删除 {} 个文件夹", 
            "✓".green().bold(), 
            self.deleted_folders.to_string().bright_cyan()
        );

        if !self.errors.is_empty() {
            println!("\n{} 遇到 {} 个错误:", 
                "⚠️".yellow(), 
                self.errors.len().to_string().yellow()
            );
            for (i, error) in self.errors.iter().enumerate() {
                if i < 5 {
                    println!("  {} {}", "•".yellow(), error.yellow());
                } else if i == 5 {
                    println!("  {} ... 还有 {} 个错误", 
                        "•".yellow(), 
                        (self.errors.len() - 5).to_string().yellow()
                    );
                    break;
                }
            }
        }
    }
}

/// 提示用户确认清理操作
pub fn prompt_confirmation(plan: &CleanupPlan) -> Result<bool> {
    if plan.is_empty() {
        return Ok(false);
    }

    println!("\n{}", "════════════════════════════════════════".bright_yellow());
    println!("{} {}", 
        "⚠️  警告:".yellow().bold(), 
        "即将删除以下文件和文件夹".bright_white()
    );
    println!("{}", "════════════════════════════════════════".bright_yellow());

    plan.display_summary();

    println!("\n{}", "════════════════════════════════════════".bright_yellow());
    print!("\n{} ", "是否继续？[y/N]:".bright_cyan().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes" || answer == "是")
}

/// 执行清理操作
pub fn execute_cleanup(plan: &CleanupPlan) -> Result<CleanupResult> {
    let mut result = CleanupResult::new();

    // 删除小文件
    for path in &plan.small_files {
        match fs::remove_file(path) {
            Ok(_) => {
                result.deleted_files += 1;
                info!("Deleted small file: {}", path.display());
                debug!("  {}", path.display());
            }
            Err(e) => {
                let error_msg = format!("Failed to delete {}: {}", path.display(), e);
                result.errors.push(error_msg.clone());
                debug!("{}", error_msg);
            }
        }
    }

    // 删除损坏文件
    for path in &plan.corrupted_files {
        match fs::remove_file(path) {
            Ok(_) => {
                result.deleted_files += 1;
                info!("Deleted corrupted file: {}", path.display());
                debug!("  {}", path.display());
            }
            Err(e) => {
                let error_msg = format!("Failed to delete {}: {}", path.display(), e);
                result.errors.push(error_msg.clone());
                debug!("{}", error_msg);
            }
        }
    }

    // 删除未完成下载文件
    for path in &plan.failed_downloads {
        match fs::remove_file(path) {
            Ok(_) => {
                result.deleted_files += 1;
                info!("Deleted failed download: {}", path.display());
                debug!("  {}", path.display());
            }
            Err(e) => {
                let error_msg = format!("Failed to delete {}: {}", path.display(), e);
                result.errors.push(error_msg.clone());
                debug!("{}", error_msg);
            }
        }
    }

    // 删除空文件夹
    for path in &plan.download_folders {
        match fs::remove_dir(path) {
            Ok(_) => {
                result.deleted_folders += 1;
                info!("Removed empty folder: {}", path.display());
                debug!("  {}", path.display());
            }
            Err(e) => {
                let error_msg = format!("Failed to remove {}: {}", path.display(), e);
                result.errors.push(error_msg.clone());
                debug!("{}", error_msg);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_plan_is_empty() {
        let plan = CleanupPlan::new();
        assert!(plan.is_empty());

        let mut plan = CleanupPlan::new();
        plan.small_files.push(PathBuf::from("test.pdf"));
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_cleanup_plan_totals() {
        let mut plan = CleanupPlan::new();
        plan.small_files.push(PathBuf::from("small1.pdf"));
        plan.small_files.push(PathBuf::from("small2.pdf"));
        plan.corrupted_files.push(PathBuf::from("corrupt.pdf"));
        plan.download_folders.push(PathBuf::from("folder.download"));

        assert_eq!(plan.total_files(), 3);
        assert_eq!(plan.total_folders(), 1);
    }

    #[test]
    fn test_cleanup_result_new() {
        let result = CleanupResult::new();
        assert_eq!(result.deleted_files, 0);
        assert_eq!(result.deleted_folders, 0);
        assert!(result.errors.is_empty());
    }
}
