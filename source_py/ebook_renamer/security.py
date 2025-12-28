"""
Security utilities for the ebook renamer.

This module provides functions to prevent common security vulnerabilities:
- Command injection in shell scripts
- Path traversal attacks
- Invalid filename handling
"""

import os
import re
from pathlib import Path
from typing import Optional


# Constants
MAX_FILENAME_LENGTH = 255
MAX_DEPTH = 1000
MAX_HASH_FILE_SIZE = 100 * 1024 * 1024  # 100MB


class SecurityError(Exception):
    """Base class for security-related errors."""
    pass


class PathTraversalError(SecurityError):
    """Raised when a path traversal attack is detected."""
    pass


class InvalidFilenameError(SecurityError):
    """Raised when a filename is invalid or dangerous."""
    pass


class FileTooLargeError(SecurityError):
    """Raised when a file exceeds maximum size for processing."""
    pass


# Regex for dangerous characters
DANGEROUS_CHARS_REGEX = re.compile(r'[\x00-\x1f\x7f<>:"/\\|?*\n\r\t]')


def shell_escape(arg: str) -> str:
    """
    Escape a string for safe use in POSIX shells.

    Uses single-quote escaping which prevents all shell interpretation,
    including backticks, $(), and other shell metacharacters.

    Args:
        arg: The string to escape

    Returns:
        A safely escaped string wrapped in single quotes

    Examples:
        >>> shell_escape("hello")
        "'hello'"
        >>> shell_escape("O'Reilly")
        "'O'\\''Reilly'"
        >>> shell_escape("$(rm -rf ~)")
        "'$(rm -rf ~)'"
    """
    if not arg:
        return "''"

    # Replace each single quote with '\'' (end quote, escaped quote, start quote)
    escaped = arg.replace("'", "'\\''")
    return f"'{escaped}'"


def sanitize_filename(input_str: str) -> str:
    """
    Validate and sanitize a filename to prevent security issues.

    Args:
        input_str: The filename to sanitize

    Returns:
        A sanitized filename

    Raises:
        InvalidFilenameError: If the filename is invalid or contains dangerous content
        PathTraversalError: If the filename contains path traversal sequences
    """
    # Check for null bytes
    if '\x00' in input_str:
        raise InvalidFilenameError("Null byte detected in filename")

    # Remove dangerous characters
    sanitized = DANGEROUS_CHARS_REGEX.sub('_', input_str)

    # Prevent path traversal in filename
    if '..' in sanitized:
        raise PathTraversalError("Path traversal sequence detected in filename")

    # Remove path separators
    sanitized = sanitized.replace('/', '_').replace('\\', '_')

    # Check length
    if len(sanitized) > MAX_FILENAME_LENGTH:
        raise InvalidFilenameError(f"Filename too long: {len(sanitized)} > {MAX_FILENAME_LENGTH}")

    # Prevent empty or whitespace-only names
    if not sanitized.strip():
        raise InvalidFilenameError("Empty or whitespace-only filename")

    return sanitized


def validate_path_traversal(path: str, base: str) -> None:
    """
    Check if a path escapes its base directory.

    Args:
        path: The path to validate
        base: The base directory that should contain the path

    Raises:
        PathTraversalError: If the path escapes the base directory
    """
    # Resolve both paths to absolute
    abs_path = os.path.abspath(os.path.normpath(path))
    abs_base = os.path.abspath(os.path.normpath(base))

    # Ensure path starts with base
    if not abs_path.startswith(abs_base + os.sep) and abs_path != abs_base:
        raise PathTraversalError(
            f"Path {path!r} escapes base directory {base!r}"
        )


def validate_max_depth(depth: int) -> None:
    """
    Check if depth is within allowed limits.

    Args:
        depth: The depth value to validate

    Raises:
        ValueError: If depth exceeds maximum allowed
    """
    if depth > MAX_DEPTH:
        raise ValueError(f"Depth {depth} exceeds maximum allowed {MAX_DEPTH}")


def validate_file_size(size: int) -> None:
    """
    Check if file size is within limits for hash computation.

    Args:
        size: The file size in bytes

    Raises:
        FileTooLargeError: If file exceeds maximum size
    """
    if size > MAX_HASH_FILE_SIZE:
        raise FileTooLargeError(
            f"File size {size} exceeds maximum {MAX_HASH_FILE_SIZE} bytes"
        )


def sanitize_path_for_logging(path: str) -> str:
    """
    Remove sensitive path components for logging.

    Only returns the filename portion of the path.

    Args:
        path: The full path

    Returns:
        Just the filename portion
    """
    return os.path.basename(path) or "[invalid_path]"


def is_safe_extension(ext: str, allowed: Optional[set] = None) -> bool:
    """
    Check if a file extension is in the allowed list.

    Args:
        ext: The extension to check (with or without leading dot)
        allowed: Set of allowed extensions (default: pdf, epub, txt)

    Returns:
        True if extension is allowed, False otherwise
    """
    if allowed is None:
        allowed = {'.pdf', '.epub', '.txt'}

    # Normalize extension
    ext_lower = ext.lower()
    if not ext_lower.startswith('.'):
        ext_lower = '.' + ext_lower

    return ext_lower in allowed
