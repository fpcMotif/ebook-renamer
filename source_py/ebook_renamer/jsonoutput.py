"""
JSON output functionality for the ebook renamer.
"""

import json
import os
from typing import List, Dict, Any

from .types import (
    OperationsOutput, RenameOperation, DuplicateGroup, 
    DeleteOperation, TodoItem, FileInfo
)


class JSONOutput:
    """Handles JSON output generation with deterministic sorting."""
    
    @staticmethod
    def from_results(clean_files: List[FileInfo], 
                    duplicate_groups: List[List[str]], 
                    files_to_delete: List[str], 
                    todo_items: List[Dict[str, str]], 
                    target_dir: str) -> OperationsOutput:
        """Create an OperationsOutput from processing results."""
        output = OperationsOutput(
            renames=[],
            duplicate_deletes=[],
            small_or_corrupted_deletes=[],
            todo_items=[]
        )
        
        # Add renames
        renames = []
        for file in clean_files:
            if file.new_name is not None:
                from_path = JSONOutput._make_relative_path(file.original_path, target_dir)
                to_path = JSONOutput._make_relative_path(file.new_path, target_dir)
                
                renames.append(RenameOperation(
                    from_path=from_path,
                    to_path=to_path,
                    reason="normalized"
                ))
        
        # Sort renames by 'from' path for deterministic output
        renames.sort(key=lambda x: x.from_path)
        output.renames = renames
        
        # Add duplicate deletions
        duplicate_deletes = []
        for group in duplicate_groups:
            if len(group) > 1:
                keep_path = JSONOutput._make_relative_path(group[0], target_dir)
                delete_paths = [JSONOutput._make_relative_path(path, target_dir) 
                              for path in group[1:]]
                # Sort delete paths for deterministic output
                delete_paths.sort()
                
                duplicate_deletes.append(DuplicateGroup(
                    keep=keep_path,
                    delete=delete_paths
                ))
        
        # Sort duplicate groups by 'keep' path for deterministic output
        duplicate_deletes.sort(key=lambda x: x.keep)
        output.duplicate_deletes = duplicate_deletes
        
        # Add small/corrupted deletions
        small_deletes = []
        for path in files_to_delete:
            small_deletes.append(DeleteOperation(
                path=JSONOutput._make_relative_path(path, target_dir),
                issue="deleted"
            ))
        
        # Sort by path for deterministic output
        small_deletes.sort(key=lambda x: x.path)
        output.small_or_corrupted_deletes = small_deletes
        
        # Add todo items (already sorted by category and file in CLI)
        output.todo_items = [TodoItem(**item) for item in todo_items]
        
        return output
    
    @staticmethod
    def to_json(output: OperationsOutput) -> str:
        """Convert the OperationsOutput to a JSON string."""
        # Convert to dict for JSON serialization
        data = {
            "renames": [
                {
                    "from": op.from_path,
                    "to": op.to_path,
                    "reason": op.reason
                }
                for op in output.renames
            ],
            "duplicate_deletes": [
                {
                    "keep": group.keep,
                    "delete": group.delete
                }
                for group in output.duplicate_deletes
            ],
            "small_or_corrupted_deletes": [
                {
                    "path": op.path,
                    "issue": op.issue
                }
                for op in output.small_or_corrupted_deletes
            ],
            "todo_items": [
                {
                    "category": item.category,
                    "file": item.file,
                    "message": item.message
                }
                for item in output.todo_items
            ]
        }
        
        return json.dumps(data, indent=2, ensure_ascii=False)
    
    @staticmethod
    def _make_relative_path(path: str, target_dir: str) -> str:
        """Convert an absolute path to a relative path using forward slashes."""
        # Convert to relative path
        try:
            rel_path = os.path.relpath(path, target_dir)
        except ValueError:
            # Fallback to absolute path if relative conversion fails
            rel_path = path
        
        # Convert to forward slashes for JSON output (POSIX-style)
        rel_path = rel_path.replace('\\', '/')
        
        # Handle case where path is the same as target directory
        if rel_path == '.':
            rel_path = ''
        
        return rel_path
