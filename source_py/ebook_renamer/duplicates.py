"""
Duplicate detection functionality for the ebook renamer.
"""

import hashlib
import os
from typing import List, Tuple

from .types import FileInfo


class DuplicateDetector:
    """Handles duplicate detection based on MD5 hash."""
    
    # Allowed formats to keep
    ALLOWED_EXTENSIONS = {".pdf", ".epub", ".txt"}
    
    def detect_duplicates(self, files: List[FileInfo]) -> Tuple[List[List[str]], List[FileInfo]]:
        """Find duplicate files based on MD5 hash."""
        # Filter to only allowed formats first
        filtered_files = [file for file in files 
                         if file.extension in self.ALLOWED_EXTENSIONS]
        
        # Build hash map: file_hash -> list of file infos
        hash_map = {}
        
        for file_info in filtered_files:
            if not file_info.is_failed_download and not file_info.is_too_small:
                try:
                    file_hash = self._compute_md5(file_info.original_path)
                    if file_hash not in hash_map:
                        hash_map[file_hash] = []
                    hash_map[file_hash].append(file_info)
                except (OSError, IOError):
                    # Skip files that can't be read
                    continue
        
        # Group duplicates by hash and apply retention strategy
        duplicate_groups = []
        duplicate_paths = set()
        
        for file_infos in hash_map.values():
            if len(file_infos) > 1:
                # Multiple files with same hash - apply retention strategy
                kept_file = self._select_file_to_keep(file_infos)
                
                group_paths = [kept_file.original_path]
                
                for file_info in file_infos:
                    if file_info.original_path != kept_file.original_path:
                        duplicate_paths.add(file_info.original_path)
                        group_paths.append(file_info.original_path)
                
                duplicate_groups.append(group_paths)
        
        # Return only non-duplicate files (including filtered out formats)
        clean_files = [file for file in filtered_files 
                      if file.original_path not in duplicate_paths]
        
        return duplicate_groups, clean_files
    
    def _select_file_to_keep(self, files: List[FileInfo]) -> FileInfo:
        """Select the file to keep based on priority: normalized > shortest path > newest."""
        # Priority 1: Already normalized files (have new_name set)
        normalized_files = [file for file in files if file.new_name is not None]
        candidates = normalized_files if normalized_files else files
        
        # Priority 2: Shortest path (fewest directory components) among candidates
        candidates_with_depth = []
        min_depth = float('inf')
        
        for file in candidates:
            depth = file.original_path.count('/')
            candidates_with_depth.append((depth, file))
            if depth < min_depth:
                min_depth = depth
        
        # Filter to shallowest candidates
        shallowest_candidates = [file for depth, file in candidates_with_depth 
                                if depth == min_depth]
        
        # Priority 3: Newest modification time among the shallowest candidates
        if not shallowest_candidates:
            # Fallback: return first file
            return files[0]
        
        newest_file = shallowest_candidates[0]
        for file in shallowest_candidates[1:]:
            if file.modified_time > newest_file.modified_time:
                newest_file = file
        
        return newest_file
    
    def _compute_md5(self, file_path: str) -> str:
        """Calculate MD5 hash of a file."""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
    
    def detect_name_variants(self, files: List[FileInfo]) -> List[List[int]]:
        """Group files by normalized name (treating (1), (2), etc. as variants)."""
        # Group files by normalized name (treating (1), (2), etc. as variants)
        name_groups = {}
        
        for idx, file_info in enumerate(files):
            if file_info.new_name is not None:
                # Strip off (1), (2), etc. to find base name
                base_name = self._strip_variant_suffix(file_info.new_name)
                if base_name not in name_groups:
                    name_groups[base_name] = []
                name_groups[base_name].append(idx)
        
        # Keep only groups with duplicates
        variants = [group for group in name_groups.values() if len(group) > 1]
        return variants
    
    def _strip_variant_suffix(self, filename: str) -> str:
        """Strip patterns like " (1)", " (2)", etc. from the end before extension."""
        # Match patterns like " (1)", " (2)", etc. at the end before extension
        if '.' in filename:
            name_part = filename.rsplit('.', 1)[0]
            ext_part = filename.rsplit('.', 1)[1]
            
            # Remove variant suffix from name part
            if name_part.endswith(')'):
                # Check if it matches pattern " (n)"
                open_paren = name_part.rfind(' (')
                if open_paren != -1:
                    suffix = name_part[open_paren:]
                    if len(suffix) >= 4 and suffix.startswith(' (') and suffix.endswith(')'):
                        # Check if content between parens is numeric
                        content = suffix[2:-1]
                        if content.isdigit():
                            name_part = name_part[:open_paren]
            
            return f"{name_part}.{ext_part}"
        else:
            # No extension, just check for variant suffix
            if filename.endswith(')'):
                open_paren = filename.rfind(' (')
                if open_paren != -1:
                    suffix = filename[open_paren:]
                    if len(suffix) >= 4 and suffix.startswith(' (') and suffix.endswith(')'):
                        content = suffix[2:-1]
                        if content.isdigit():
                            return filename[:open_paren]
        
        return filename
