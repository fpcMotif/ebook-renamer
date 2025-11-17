#!/usr/bin/env python3
"""
Build golden JSON files from Rust implementation.
This script runs the Rust ebook-renamer in JSON mode and captures the output
to create golden reference files for cross-language testing.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path


def run_rust_ebook_renamer(target_dir: Path, output_format: str = "json") -> dict:
    """Run the Rust ebook-renamer and return parsed JSON output."""
    # Build the Rust project first
    print("Building Rust ebook-renamer...")
    build_result = subprocess.run(
        ["cargo", "build"],
        cwd=Path(__file__).parent.parent.parent,
        capture_output=True,
        text=True
    )
    
    if build_result.returncode != 0:
        print(f"Failed to build Rust project: {build_result.stderr}")
        sys.exit(1)
    
    # Run the tool in JSON dry-run mode
    rust_binary = Path(__file__).parent.parent.parent / "target" / "debug" / "ebook_renamer"
    cmd = [
        str(rust_binary),
        "--dry-run",
        f"--{output_format}",
        str(target_dir)
    ]
    
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True
    )
    
    if result.returncode != 0:
        print(f"Failed to run Rust ebook-renamer: {result.stderr}")
        sys.exit(1)
    
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as e:
        print(f"Failed to parse JSON output: {e}")
        print(f"Raw output: {result.stdout}")
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="Build golden JSON files from Rust implementation")
    parser.add_argument(
        "--target-dir",
        required=True,
        help="Directory containing test files to process"
    )
    parser.add_argument(
        "--output-dir",
        default=Path(__file__).parent.parent / "fixtures",
        help="Directory to write golden JSON files"
    )
    parser.add_argument(
        "--mapping-file",
        default="golden-mapping.json",
        help="Name of the mapping JSON file"
    )
    parser.add_argument(
        "--todo-file",
        default="golden-todo.json",
        help="Name of the todo JSON file"
    )
    
    args = parser.parse_args()
    
    target_dir = Path(args.target_dir)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    if not target_dir.exists():
        print(f"Target directory does not exist: {target_dir}")
        sys.exit(1)
    
    # Run Rust implementation
    rust_output = run_rust_ebook_renamer(target_dir)
    
    # Write golden mapping file (contains all operations)
    mapping_file = output_dir / args.mapping_file
    with open(mapping_file, 'w', encoding='utf-8') as f:
        json.dump(rust_output, f, indent=2, ensure_ascii=False)
    
    print(f"✓ Golden mapping written to: {mapping_file}")
    
    # Write golden todo file (contains only todo items)
    todo_file = output_dir / args.todo_file
    with open(todo_file, 'w', encoding='utf-8') as f:
        json.dump(rust_output.get('todo_items', []), f, indent=2, ensure_ascii=False)
    
    print(f"✓ Golden todo written to: {todo_file}")
    
    # Print summary
    print(f"\nSummary:")
    print(f"  Renames: {len(rust_output.get('renames', []))}")
    print(f"  Duplicate groups: {len(rust_output.get('duplicate_deletes', []))}")
    print(f"  Small/corrupted deletes: {len(rust_output.get('small_or_corrupted_deletes', []))}")
    print(f"  Todo items: {len(rust_output.get('todo_items', []))}")


if __name__ == "__main__":
    main()
