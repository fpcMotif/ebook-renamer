"""
Command-line interface for the ebook renamer.
"""

import argparse
import os
import sys
from pathlib import Path
from typing import Optional

from .types import Config
from .scanner import Scanner
from .normalizer import Normalizer
from .duplicates import DuplicateDetector
from .todo import TodoList
from .jsonoutput import JSONOutput


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
        json=args.json,
    )


def main() -> int:
    """Main entry point."""
    try:
        config = parse_args()
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

    # Normalize filenames
    normalizer = Normalizer()
    normalized = normalizer.normalize_files(files)

    # Determine todo file path
    todo_file_path = determine_todo_file(config.path, config.todo_file)

    # Create todo list
    todo_list = TodoList(todo_file_path, config.path)

    # Handle failed downloads and small files
    files_to_delete = []
    todo_items = []

    for file_info in normalized:
        if file_info.is_failed_download or file_info.is_too_small:
            if config.delete_small:
                files_to_delete.append(file_info.original_path)
                todo_list.remove_file_from_todo(file_info.original_name)
            else:
                if file_info.is_failed_download:
                    todo_list.add_failed_download(file_info)
                    todo_items.append({
                        "category": "failed_download",
                        "file": file_info.original_name,
                        "message": f"重新下载: {file_info.original_name} (未完成下载)"
                    })
                else:
                    todo_list.add_failed_download(file_info)
                    todo_items.append({
                        "category": "too_small",
                        "file": file_info.original_name,
                        "message": f"检查并重新下载: {file_info.original_name} (文件过小，仅 {file_info.size} 字节)"
                    })
        else:
            todo_list.analyze_file_integrity(file_info)

    # Detect duplicates
    detector = DuplicateDetector()
    duplicate_groups, clean_files = detector.detect_duplicates(normalized)

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
        execute_operations(clean_files, duplicate_groups, files_to_delete, todo_list, config)

    if not config.json:
        print("\n✓ Operation completed successfully!")

    return 0


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


def execute_operations(clean_files, duplicate_groups, files_to_delete, todo_list, config: Config):
    """Execute the file operations."""
    # Execute renames
    for file_info in clean_files:
        if file_info.new_name:
            os.rename(file_info.original_path, file_info.new_path)
    
    # Delete duplicates
    if not config.no_delete:
        for group in duplicate_groups:
            if len(group) > 1:
                for i, path in enumerate(group):
                    if i > 0:
                        os.remove(path)
    
    # Delete small/corrupted files
    if config.delete_small and files_to_delete:
        print(f"\nDeleting {len(files_to_delete)} small/corrupted files...")
        for path in files_to_delete:
            os.remove(path)
            print(f"  Deleted: {path}")
    
    # Write todo.md
    todo_list.write()


if __name__ == "__main__":
    sys.exit(main())
