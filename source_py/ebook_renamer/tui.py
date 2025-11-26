import os
import sys
import logging
from .types import Config, CleanupResult, FileIssue
from .scanner import Scanner
from .normalizer import Normalizer
from .duplicates import DuplicateDetector
from .todo import TodoList

try:
    from rich.console import Console
    from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn
    from rich.table import Table
    RICH_AVAILABLE = True
except ImportError:
    RICH_AVAILABLE = False

def run_tui(config: Config) -> int:
    if not RICH_AVAILABLE:
        print("Rich library not found. Please install it with `pip install rich` to see the TUI.")
        return 1

    console = Console()
    console.print("[bold green]Ebook Renamer[/bold green]")

    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TaskProgressColumn(),
        console=console
    ) as progress:
        
        # 1. Scan
        task_scan = progress.add_task("[cyan]Scanning...", total=None)
        scanner = Scanner(config.path, config.max_depth)
        files = scanner.scan()
        progress.update(task_scan, completed=100, total=100)
        console.print(f"Found {len(files)} files")

        # 2. Normalize
        task_norm = progress.add_task("[magenta]Normalizing...", total=len(files))
        normalizer = Normalizer()
        normalized = normalizer.normalize_files(files)
        progress.update(task_norm, completed=len(files))
        console.print(f"Normalized {len(normalized)} files")

        # 3. Check Integrity
        task_check = progress.add_task("[yellow]Checking Integrity...", total=len(normalized))
        
        # Determine todo file path
        todo_file_path = os.path.join(config.path, "todo.md")
        if config.todo_file:
            todo_file_path = config.todo_file

        todo_list = TodoList(todo_file_path, config.path)
        
        # Categorize problematic files (Simplified logic)
        incomplete_downloads = []
        corrupted_files = []
        small_files = []
        
        for file_info in normalized:
            if file_info.is_failed_download:
                incomplete_downloads.append(file_info)
            elif file_info.is_too_small:
                small_files.append(file_info)
            else:
                if file_info.extension.lower() == ".pdf":
                    if not todo_list._validate_pdf_header(file_info.original_path):
                        corrupted_files.append(file_info)
            
            # Analyze integrity
            if not file_info.is_failed_download and not file_info.is_too_small:
                 todo_list.analyze_file_integrity(file_info)
            
            progress.advance(task_check)
        
        console.print("Integrity check complete")

        # 4. Duplicates
        task_dup = progress.add_task("[blue]Detecting Duplicates...", total=None)
        detector = DuplicateDetector()
        duplicate_groups, clean_files = detector.detect_duplicates(normalized)
        progress.update(task_dup, completed=100, total=100)
        console.print(f"Detected {len(duplicate_groups)} duplicate groups")

        # 5. Execute
        if not config.dry_run:
            task_exec = progress.add_task("[red]Executing...", total=len(clean_files))
            for file_info in clean_files:
                if file_info.new_name:
                    os.rename(file_info.original_path, file_info.new_path)
                progress.advance(task_exec)
            
            # Delete duplicates
            if not config.no_delete:
                for group in duplicate_groups:
                    if len(group) > 1:
                        for i, path in enumerate(group):
                            if i > 0:
                                try:
                                    os.remove(path)
                                except OSError:
                                    pass
            
            # Write todo
            todo_list.write()
            console.print("[bold green]Done![/bold green]")
        else:
            # Dry run output
            console.print("[bold yellow]Dry Run Complete[/bold yellow]")
            todo_list.write()
            console.print("todo.md written")

    return 0
