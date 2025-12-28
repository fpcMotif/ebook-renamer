// Package security provides security utilities for the ebook renamer.
package security

import (
	"errors"
	"path/filepath"
	"regexp"
	"strings"
)

// Common security errors
var (
	ErrPathTraversal    = errors.New("path traversal attempt detected")
	ErrInvalidFilename  = errors.New("invalid filename")
	ErrFilenameTooLong  = errors.New("filename too long")
	ErrNullByte         = errors.New("null byte in input")
	ErrInvalidInput     = errors.New("invalid input")
)

const (
	// MaxFilenameLength is the maximum allowed filename length
	MaxFilenameLength = 255

	// MaxDepth is the maximum allowed directory traversal depth
	MaxDepth = 1000

	// MaxHashFileSize is the maximum file size for hash computation (100MB)
	MaxHashFileSize = 100 * 1024 * 1024
)

var (
	// Dangerous characters regex for filename sanitization
	dangerousCharsRegex = regexp.MustCompile(`[\x00-\x1F\x7F<>:"/\\|?*\n\r\t]`)
)

// ShellEscape escapes a string for safe use in POSIX shells.
// Uses single-quote escaping which prevents all shell interpretation.
func ShellEscape(arg string) string {
	if arg == "" {
		return "''"
	}

	var escaped strings.Builder
	escaped.WriteRune('\'')

	for _, ch := range arg {
		if ch == '\'' {
			escaped.WriteString("'\\''")
		} else {
			escaped.WriteRune(ch)
		}
	}

	escaped.WriteRune('\'')
	return escaped.String()
}

// SanitizeFilename validates and sanitizes a filename to prevent security issues.
func SanitizeFilename(input string) (string, error) {
	// Check for null bytes
	if strings.Contains(input, "\x00") {
		return "", ErrNullByte
	}

	// Remove dangerous characters
	sanitized := dangerousCharsRegex.ReplaceAllString(input, "_")

	// Prevent path traversal in filename
	if strings.Contains(sanitized, "..") {
		return "", ErrPathTraversal
	}

	// Remove path separators
	sanitized = strings.ReplaceAll(sanitized, "/", "_")
	sanitized = strings.ReplaceAll(sanitized, "\\", "_")

	// Check length
	if len(sanitized) > MaxFilenameLength {
		return "", ErrFilenameTooLong
	}

	// Prevent empty or whitespace-only names
	if strings.TrimSpace(sanitized) == "" {
		return "", ErrInvalidFilename
	}

	return sanitized, nil
}

// ValidatePathTraversal checks if a path escapes its base directory.
func ValidatePathTraversal(path, base string) error {
	// Clean both paths
	cleanPath := filepath.Clean(path)
	cleanBase := filepath.Clean(base)

	// Get absolute paths
	absPath, err := filepath.Abs(cleanPath)
	if err != nil {
		return ErrInvalidInput
	}

	absBase, err := filepath.Abs(cleanBase)
	if err != nil {
		return ErrInvalidInput
	}

	// Check if path starts with base (with trailing separator to prevent partial matches)
	if !strings.HasPrefix(absPath, absBase+string(filepath.Separator)) && absPath != absBase {
		return ErrPathTraversal
	}

	return nil
}

// ValidateMaxDepth checks if depth is within allowed limits.
func ValidateMaxDepth(depth int) error {
	if depth > MaxDepth {
		return errors.New("depth exceeds maximum allowed")
	}
	return nil
}

// ValidateFileSize checks if file size is within limits for hash computation.
func ValidateFileSize(size int64) error {
	if size > MaxHashFileSize {
		return errors.New("file too large for hash computation")
	}
	return nil
}

// SanitizePathForLogging removes sensitive path components for logging.
func SanitizePathForLogging(path string) string {
	return filepath.Base(path)
}
