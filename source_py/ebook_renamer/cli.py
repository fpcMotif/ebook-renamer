"""
Command-line interface for the ebook renamer.
"""

import argparse
import logging
import os
import sys
from pathlib import Path
from typing import Optional

from .types import Config, CleanupResult, FileIssue
from .scanner import Scanner
from .normalizer import Normalizer
from .duplicates import DuplicateDetector
from .todo import TodoList
from .todo import TodoList
from .jsonoutput import JSONOutput
from .tui import run_tui, RICH_AVAILABLE


def create_parser() -> argparse.ArgumentParser:
    """Create the command-line argument parser."""
    parser = argparse.ArgumentParser(
        prog="ebook-renamer",
        description="Batch rename and organize downloaded books and arXiv files",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
This tool scans a directory for ebook files, normalizes their filenames,
detects duplicates, and generates a todo.md file for manual review.
        """
    )

    # Positional argument (optional, defaults to current directory)
    parser.add_argument(
        "path",
        nargs="?",
        default=".",
        help="Directory to scan (default: current directory)"
    )

    # CLI flags matching Rust implementation
    parser.add_argument(
        "-d", "--dry-run",
        action="store_true",
        help="Perform dry run: show changes without applying them (Note: todo.md is always written, even in dry-run mode)"
    )
    parser.add_argument(
        "--max-depth",
        type=str,
        default="18446744073709551615",
        help="Maximum directory depth to traverse (default: unlimited)"
    )
    parser.add_argument(
        "--no-recursive",
        action="store_true",
        help="Only scan the top-level directory, no recursion"
    )
    parser.add_argument(
        "--extensions",
        type=str,
        default="",
        help="Comma-separated extensions to process (default: pdf,epub,txt)"
    )
    parser.add_argument(
        "--no-delete",
        action="store_true",
        help="Don't delete duplicates, only list them"
    )
    parser.add_argument(
        "--todo-file",
        type=str,
        default="",
        help="Path to write todo.md (default: <target-dir>/todo.md)"
    )
    parser.add_argument(
        "--log-file",
        type=str,
        default="",
        help="Optional path to write detailed operation log"
    )
    parser.add_argument(
        "--preserve-unicode",
        action="store_true",
        help="Preserve original non-Latin script (CJK, etc.) without modification"
    )
    parser.add_argument(
        "--fetch-arxiv",
        action="store_true",
        help="Fetch arXiv metadata via API (not implemented yet)"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose logging"
    )
    parser.add_argument(
        "--delete-small",
        action="store_true",
        help="Delete small/corrupted files (< 1KB) instead of adding to todo list"
    )
    parser.add_argument(
        "--auto-cleanup",
        action="store_true",
        help="Automatically clean up incomplete downloads (.download/.crdownload) and corrupted files"
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output operations in JSON format instead of human-readable text"
    )

    return parser


def parse_args() -> Config:
    """Parse command-line arguments and create Config."""
    parser = create_parser()
    args = parser.parse_args()

    # Convert to absolute path
    abs_path = os.path.abspath(args.path)
    
    # Check if path exists and is a directory
    if not os.path.exists(abs_path):
        parser.error(f"Path does not exist: {abs_path}")
    if not os.path.isdir(abs_path):
        parser.error(f"Path is not a directory: {abs_path}")

    # Parse max depth
    try:
        max_depth = int(args.max_depth)
    except ValueError:
        parser.error(f"Invalid max-depth: {args.max_depth}")

    # Handle --no-recursive by setting max_depth to 1
    effective_max_depth = max_depth
    if args.no_recursive:
        effective_max_depth = 1

    # Parse extensions
    if args.extensions:
        extensions = [ext.strip() for ext in args.extensions.split(",")]
        # Ensure extensions start with dot
        extensions = [ext if ext.startswith(".") else f".{ext}" 
                     for ext in extensions if ext]
    else:
        extensions = [".pdf", ".epub", ".txt"]

    # Handle --fetch-arxiv placeholder
    if args.fetch_arxiv:
        print("⚠️  Warning: --fetch-arxiv is not implemented yet. Files will be processed offline only.", 
              file=sys.stderr)

    return Config(
        path=abs_path,
        dry_run=args.dry_run,
        max_depth=effective_max_depth,
        no_recursive=args.no_recursive,
        extensions=extensions,
        no_delete=args.no_delete,
        todo_file=args.todo_file if args.todo_file else None,
        log_file=args.log_file if args.log_file else None,
        preserve_unicode=args.preserve_unicode,
        fetch_arxiv=args.fetch_arxiv,
        verbose=args.verbose,
        delete_small=args.delete_small,
        auto_cleanup=args.auto_cleanup,
        json=args.json,
    )


def main() -> int:
    """Main entry point."""
    # Setup logging with timestamp and milliseconds
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s.%(msecs)03d %(levelname)s: %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )
    
    try:
        config = parse_args()
        logging.info(f"Starting ebook renamer with config: {config}")
        
        if not config.json and RICH_AVAILABLE:
            return run_tui(config)

        return process_files(config)
    except KeyboardInterrupt:
        print("\nOperation cancelled by user", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


def process_files(config: Config) -> int:
    """Process files according to the configuration."""
    # Create scanner
    scanner = Scanner(config.path, config.max_depth)
    
    # Scan for files
    files = scanner.scan()
    logging.info(f"Found {len(files)} files to process")

    # Normalize filenames
    normalizer = Normalizer()
    normalized = normalizer.normalize_files(files)
    logging.info(f"Normalized {len(normalized)} files")

    # Determine todo file path
    todo_file_path = determine_todo_file(config.path, config.todo_file)

    # Create todo list
    todo_list = TodoList(todo_file_path, config.path)

    # Categorize problematic files
    incomplete_downloads = []  # .download, .crdownload files
    corrupted_files = []       # Corrupted PDFs or unreadable files
    small_files = []           # Files that are too small (< 1KB)
    
    for file_info in normalized:
        if file_info.is_failed_download:
            incomplete_downloads.append(file_info)
        elif file_info.is_too_small:
            small_files.append(file_info)
        else:
            # Check for corruption
            if file_info.extension.lower() == ".pdf":
                if not todo_list._validate_pdf_header(file_info.original_path):
                    corrupted_files.append(file_info)

    # Print summary of found issues
    if not config.json:
        print_issue_summary(incomplete_downloads, corrupted_files, small_files)

    # Determine cleanup behavior based on flags
    should_cleanup = config.auto_cleanup or config.delete_small
    
    # Track files to delete and todo items
    files_to_delete = []
    todo_items = []
    cleanup_result = CleanupResult(
        deleted_incomplete=[],
        deleted_corrupted=[],
        deleted_small=[],
        failed_deletions=[]
    )

    # Process incomplete downloads
    for file_info in incomplete_downloads:
        if should_cleanup:
            files_to_delete.append(file_info.original_path)
            cleanup_result.deleted_incomplete.append(file_info.original_path)
            todo_list.remove_file_from_todo(file_info.original_name)
        else:
            todo_list.add_failed_download(file_info)
            todo_items.append({
                "category": "failed_download",
                "file": file_info.original_name,
                "message": f"重新下载: {file_info.original_name} (未完成下载)"
            })
    
    # Process corrupted files
    for file_info in corrupted_files:
        if should_cleanup:
            files_to_delete.append(file_info.original_path)
            cleanup_result.deleted_corrupted.append(file_info.original_path)
            todo_list.remove_file_from_todo(file_info.original_name)
        else:
            todo_list.add_file_issue(file_info, FileIssue.CORRUPTED_PDF)
            todo_items.append({
                "category": "corrupted",
                "file": file_info.original_name,
                "message": f"重新下载: {file_info.original_name} (PDF文件损坏)"
            })
    
    # Process small files
    for file_info in small_files:
        if config.delete_small:
            files_to_delete.append(file_info.original_path)
            cleanup_result.deleted_small.append(file_info.original_path)
            todo_list.remove_file_from_todo(file_info.original_name)
        elif config.auto_cleanup:
            # Auto-cleanup mode: add to todo for manual review (might be valid small ebook)
            todo_list.add_failed_download(file_info)
            todo_items.append({
                "category": "too_small",
                "file": file_info.original_name,
                "message": f"检查文件: {file_info.original_name} (文件过小 {file_info.size} 字节，可能需要重新下载)"
            })
        else:
            todo_list.add_failed_download(file_info)
            todo_items.append({
                "category": "too_small",
                "file": file_info.original_name,
                "message": f"检查并重新下载: {file_info.original_name} (文件过小，仅 {file_info.size} 字节)"
            })
    
    # Analyze other files for integrity
    for file_info in normalized:
        if (file_info not in incomplete_downloads and 
            file_info not in corrupted_files and 
            file_info not in small_files):
            todo_list.analyze_file_integrity(file_info)

    # Detect duplicates
    detector = DuplicateDetector()
    duplicate_groups, clean_files = detector.detect_duplicates(normalized)
    logging.info(f"Detected {len(duplicate_groups)} duplicate groups")

    # Sort todo items by category, then file for deterministic output (matching Rust)
    todo_items.sort(key=lambda x: (x["category"], x["file"]))

    # Output results
    if config.dry_run:
        if config.json:
            # JSON output
            output = JSONOutput.from_results(
                clean_files, duplicate_groups, files_to_delete, todo_items, config.path
            )
            json_str = JSONOutput.to_json(output)
            print(json_str)
        else:
            # Human-readable output
            print_human_output(clean_files, duplicate_groups, files_to_delete, todo_list)
        
        # Write todo.md even in dry-run mode
        todo_list.write()
        
        if not config.json:
            print("\n✓ todo.md written (dry-run mode)")
    else:
        # Execute operations
        cleanup_result = execute_operations(
            clean_files, duplicate_groups, files_to_delete, 
            todo_list, config, cleanup_result
        )
        
        # Print cleanup summary
        if not config.json:
            print_cleanup_summary(cleanup_result)

    if not config.json:
        print("\n✓ Operation completed successfully!")

    return 0


def print_issue_summary(incomplete: list, corrupted: list, small: list) -> None:
    """Print a summary of found issues."""
    total_issues = len(incomplete) + len(corrupted) + len(small)
    
    if total_issues == 0:
        print("\n📋 文件扫描完成，未发现问题文件")
        return
    
    print(f"\n📋 发现 {total_issues} 个问题文件:")
    print("-" * 40)
    
    if incomplete:
        print(f"  🔄 未完成下载: {len(incomplete)} 个")
        for f in incomplete[:3]:  # Show first 3
            print(f"     • {f.original_name}")
        if len(incomplete) > 3:
            print(f"     ... 及其他 {len(incomplete) - 3} 个文件")
    
    if corrupted:
        print(f"  🚨 损坏文件: {len(corrupted)} 个")
        for f in corrupted[:3]:
            print(f"     • {f.original_name}")
        if len(corrupted) > 3:
            print(f"     ... 及其他 {len(corrupted) - 3} 个文件")
    
    if small:
        print(f"  📁 异常小文件: {len(small)} 个")
        for f in small[:3]:
            print(f"     • {f.original_name} ({f.size} 字节)")
        if len(small) > 3:
            print(f"     ... 及其他 {len(small) - 3} 个文件")
    
    print("-" * 40)


def print_cleanup_summary(result: CleanupResult) -> None:
    """Print a summary of cleanup operations."""
    total_deleted = (len(result.deleted_incomplete) + 
                    len(result.deleted_corrupted) + 
                    len(result.deleted_small))
    
    if total_deleted == 0 and not result.failed_deletions:
        return
    
    print("\n🧹 清理完成:")
    print("-" * 40)
    
    if result.deleted_incomplete:
        print(f"  ✓ 删除未完成下载: {len(result.deleted_incomplete)} 个")
    
    if result.deleted_corrupted:
        print(f"  ✓ 删除损坏文件: {len(result.deleted_corrupted)} 个")
    
    if result.deleted_small:
        print(f"  ✓ 删除异常小文件: {len(result.deleted_small)} 个")
    
    if result.failed_deletions:
        print(f"  ⚠️  删除失败: {len(result.failed_deletions)} 个")
        for path, error in result.failed_deletions[:3]:
            print(f"     • {os.path.basename(path)}: {error}")
    
    print("-" * 40)


def determine_todo_file(target_dir: str, custom_path: Optional[str]) -> str:
    """Determine the todo file path."""
    if custom_path:
        return custom_path
    return os.path.join(target_dir, "todo.md")


def print_human_output(clean_files, duplicate_groups, files_to_delete, todo_list):
    """Print human-readable output for dry-run mode."""
    print("\n=== DRY RUN MODE ===")
    
    # Print renames
    for file_info in clean_files:
        if file_info.new_name:
            print(f"RENAME: {file_info.original_name} -> {file_info.new_name}")
    
    # Print duplicate deletions
    for group in duplicate_groups:
        if len(group) > 1:
            print("\nDELETE DUPLICATES:")
            for i, path in enumerate(group):
                if i == 0:
                    print(f"  KEEP: {path}")
                else:
                    print(f"  DELETE: {path}")
    
    # Print small/corrupted deletions
    if files_to_delete:
        print("\nDELETE SMALL/CORRUPTED FILES:")
        for path in files_to_delete:
            print(f"  DELETE: {path}")
    
    # Print todo items
    items = todo_list.get_items()
    if items:
        print("\nTODO LIST:")
        for item in items:
            print(f"  - [ ] {item}")


def execute_operations(clean_files, duplicate_groups, files_to_delete, 
                       todo_list, config: Config, cleanup_result: CleanupResult) -> CleanupResult:
    """Execute the file operations."""
    # Execute renames
    for file_info in clean_files:
        if file_info.new_name:
            os.rename(file_info.original_path, file_info.new_path)
            logging.info(f"Renamed: {file_info.original_name} -> {file_info.new_name}")
    
    # Delete duplicates
    if not config.no_delete:
        for group in duplicate_groups:
            if len(group) > 1:
                for i, path in enumerate(group):
                    if i > 0:
                        try:
                            os.remove(path)
                            logging.info(f"Deleted duplicate: {path}")
                        except OSError as e:
                            logging.error(f"Failed to delete duplicate: {path}: {e}")
    
    # Delete problematic files (incomplete downloads, corrupted, small)
    if files_to_delete:
        for path in files_to_delete:
            try:
                os.remove(path)
                logging.info(f"Deleted problematic file: {path}")
            except OSError as e:
                logging.error(f"Failed to delete file: {path}: {e}")
                cleanup_result.failed_deletions.append((path, str(e)))
                # Remove from the deleted lists if deletion failed
                if path in cleanup_result.deleted_incomplete:
                    cleanup_result.deleted_incomplete.remove(path)
                if path in cleanup_result.deleted_corrupted:
                    cleanup_result.deleted_corrupted.remove(path)
                if path in cleanup_result.deleted_small:
                    cleanup_result.deleted_small.remove(path)
    
    # Write todo.md
    todo_list.write()
    logging.info("Wrote todo.md")
    
    return cleanup_result


if __name__ == "__main__":
    sys.exit(main())
