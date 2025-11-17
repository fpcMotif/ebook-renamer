#!/usr/bin/env python3
"""
Import test files from /Users/f/Downloads to create realistic test fixtures.
This script copies ebook files from the Downloads folder to the test fixtures directory.
"""

import argparse
import shutil
from pathlib import Path


def get_ebook_files(source_dir: Path) -> list[Path]:
    """Find all ebook files in the source directory."""
    extensions = {'.pdf', '.epub', '.txt', '.mobi', '.download', '.crdownload'}
    ebook_files = []
    
    for file_path in source_dir.rglob('*'):
        if file_path.is_file() and file_path.suffix.lower() in extensions:
            ebook_files.append(file_path)
    
    return ebook_files


def copy_files_to_fixture(files: list[Path], output_dir: Path, preserve_structure: bool = False) -> None:
    """Copy files to the fixtures directory."""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    copied_count = 0
    skipped_count = 0
    
    for file_path in files:
        if preserve_structure:
            # Preserve relative directory structure
            relative_path = file_path.relative_to(file_path.parents[0])  # Just the filename
            dest_path = output_dir / relative_path
        else:
            # Flatten structure, just use filename
            dest_path = output_dir / file_path.name
        
        # Skip if file already exists
        if dest_path.exists():
            print(f"Skipping existing file: {dest_path.name}")
            skipped_count += 1
            continue
        
        try:
            # Create parent directories if needed
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Copy the file
            shutil.copy2(file_path, dest_path)
            print(f"Copied: {file_path.name} -> {dest_path}")
            copied_count += 1
        except Exception as e:
            print(f"Failed to copy {file_path.name}: {e}")
    
    print(f"\nSummary: Copied {copied_count} files, skipped {skipped_count} existing files")


def main():
    parser = argparse.ArgumentParser(description="Import ebook files from Downloads to test fixtures")
    parser.add_argument(
        "--downloads-dir",
        default="/Users/f/Downloads",
        help="Source directory containing ebook files (default: /Users/f/Downloads)"
    )
    parser.add_argument(
        "--output-dir",
        default=Path(__file__).parent.parent / "fixtures" / "noisy",
        help="Output directory for test fixtures"
    )
    parser.add_argument(
        "--preserve-structure",
        action="store_true",
        help="Preserve directory structure from Downloads"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be copied without actually copying"
    )
    
    args = parser.parse_args()
    
    source_dir = Path(args.downloads_dir)
    output_dir = Path(args.output_dir)
    
    if not source_dir.exists():
        print(f"Source directory does not exist: {source_dir}")
        return 1
    
    # Find all ebook files
    print(f"Scanning {source_dir} for ebook files...")
    ebook_files = get_ebook_files(source_dir)
    
    if not ebook_files:
        print("No ebook files found in source directory")
        return 0
    
    print(f"Found {len(ebook_files)} ebook files:")
    for file_path in ebook_files[:10]:  # Show first 10
        print(f"  - {file_path.relative_to(source_dir)}")
    if len(ebook_files) > 10:
        print(f"  ... and {len(ebook_files) - 10} more files")
    
    if args.dry_run:
        print(f"\nDry run: Would copy {len(ebook_files)} files to {output_dir}")
        return 0
    
    # Copy files
    copy_files_to_fixture(ebook_files, output_dir, args.preserve_structure)
    
    print(f"\n✓ Import completed. Test fixtures ready in: {output_dir}")
    return 0


if __name__ == "__main__":
    exit(main())
