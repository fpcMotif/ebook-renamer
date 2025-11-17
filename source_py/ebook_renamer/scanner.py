"""
File scanning functionality for the ebook renamer.
"""

import os
from pathlib import Path
from typing import List

from .types import FileInfo


class Scanner:
    """Handles file scanning operations."""
    
    def __init__(self, root_path: str, max_depth: int):
        self.root_path = root_path
        self.max_depth = max_depth
    
    def scan(self) -> List[FileInfo]:
        """Scan the directory for files matching criteria."""
        files = []
        
        for root, dirs, filenames in os.walk(self.root_path):
            # Calculate depth
            relative_path = os.path.relpath(root, self.root_path)
            if relative_path == ".":
                depth = 0
            else:
                depth = relative_path.count(os.sep)
            
            # Skip if depth exceeds max_depth
            if depth > self.max_depth:
                # Don't recurse deeper
                dirs[:] = []
                continue
            
            # Skip hidden directories and system directories
            dirs[:] = [d for d in dirs if not self._should_skip_directory(d)]
            
            for filename in filenames:
                if not self._should_skip_file(filename):
                    file_path = os.path.join(root, filename)
                    try:
                        file_info = self._create_file_info(file_path)
                        if file_info:
                            files.append(file_info)
                    except (OSError, IOError):
                        # Skip files that can't be accessed
                        continue
        
        return files
    
    def _should_skip_directory(self, dirname: str) -> bool:
        """Determine if a directory should be skipped."""
        # Skip hidden directories
        if dirname.startswith("."):
            return True
        
        # Skip known system directories
        skip_dirs = {"Xcode", "node_modules", ".git", "__pycache__"}
        if dirname in skip_dirs:
            return True
        
        return False
    
    def _should_skip_file(self, filename: str) -> bool:
        """Determine if a file should be skipped."""
        # Skip hidden files
        if filename.startswith("."):
            return True
        
        return False
    
    def _create_file_info(self, file_path: str) -> FileInfo:
        """Create a FileInfo struct for the given path."""
        stat = os.stat(file_path)
        original_name = os.path.basename(file_path)
        
        # Detect extension (including .tar.gz)
        extension = self._detect_extension(original_name)
        
        # Detect failed downloads
        is_failed_download = (original_name.endswith(".download") or 
                             original_name.endswith(".crdownload"))
        
        # Check if file is too small (only for PDF and EPUB files)
        is_ebook = extension in {".pdf", ".epub"}
        is_too_small = (not is_failed_download and 
                       is_ebook and 
                       stat.st_size < 1024)  # Less than 1KB
        
        return FileInfo(
            original_path=file_path,
            original_name=original_name,
            extension=extension,
            size=stat.st_size,
            modified_time=stat.st_mtime,
            is_failed_download=is_failed_download,
            is_too_small=is_too_small,
            new_path=file_path,
        )
    
    def _detect_extension(self, filename: str) -> str:
        """Detect file extension, including special cases like .tar.gz."""
        if filename.endswith(".tar.gz"):
            return ".tar.gz"
        elif filename.endswith(".download"):
            return ".download"
        elif filename.endswith(".crdownload"):
            return ".crdownload"
        else:
            return Path(filename).suffix
