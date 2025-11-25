"""
Type definitions and data structures for the ebook renamer.
"""

from dataclasses import dataclass
from typing import List, Optional
from datetime import datetime
from enum import Enum


class FileIssue(Enum):
    """Types of file issues that can be detected."""
    FAILED_DOWNLOAD = "failed_download"
    TOO_SMALL = "too_small"
    CORRUPTED_PDF = "corrupted_pdf"
    READ_ERROR = "read_error"


@dataclass
class FileInfo:
    """Information about a scanned file."""
    original_path: str
    original_name: str
    extension: str
    size: int
    modified_time: datetime
    is_failed_download: bool
    is_too_small: bool
    new_name: Optional[str] = None
    new_path: str = ""

    def __post_init__(self):
        if self.new_path == "":
            self.new_path = self.original_path


@dataclass
class ParsedMetadata:
    """Parsed filename components."""
    authors: Optional[str]
    title: str
    year: Optional[int]


@dataclass
class RenameOperation:
    """Represents a file rename operation."""
    from_path: str
    to_path: str
    reason: str


@dataclass
class DuplicateGroup:
    """Represents a group of duplicate files."""
    keep: str
    delete: List[str]


@dataclass
class DeleteOperation:
    """Represents a file deletion operation."""
    path: str
    issue: str


@dataclass
class TodoItem:
    """Represents a todo list item."""
    category: str
    file: str
    message: str


@dataclass
class OperationsOutput:
    """Complete JSON output structure."""
    renames: List[RenameOperation]
    duplicate_deletes: List[DuplicateGroup]
    small_or_corrupted_deletes: List[DeleteOperation]
    todo_items: List[TodoItem]


@dataclass
class Config:
    """Application configuration."""
    path: str
    dry_run: bool
    max_depth: int
    no_recursive: bool
    extensions: List[str]
    no_delete: bool
    todo_file: Optional[str]
    log_file: Optional[str]
    preserve_unicode: bool
    fetch_arxiv: bool
    verbose: bool
    delete_small: bool
    auto_cleanup: bool
    json: bool


@dataclass
class CleanupResult:
    """Result of cleanup operation."""
    deleted_incomplete: List[str]
    deleted_corrupted: List[str]
    deleted_small: List[str]
    failed_deletions: List[tuple]  # (path, error message)
