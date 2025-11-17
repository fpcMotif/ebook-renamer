package types

import (
	"time"
)

// FileInfo represents information about a scanned file
type FileInfo struct {
	OriginalPath     string    `json:"original_path"`
	OriginalName     string    `json:"original_name"`
	Extension        string    `json:"extension"`
	Size             uint64    `json:"size"`
	ModifiedTime     time.Time `json:"modified_time"`
	IsFailedDownload bool      `json:"is_failed_download"`
	IsTooSmall       bool      `json:"is_too_small"`
	NewName          *string   `json:"new_name,omitempty"`
	NewPath          string    `json:"new_path"`
}

// ParsedMetadata represents parsed filename components
type ParsedMetadata struct {
	Authors *string `json:"authors,omitempty"`
	Title   string  `json:"title"`
	Year    *uint16 `json:"year,omitempty"`
}

// RenameOperation represents a file rename operation
type RenameOperation struct {
	From   string `json:"from"`
	To     string `json:"to"`
	Reason string `json:"reason"`
}

// DuplicateGroup represents a group of duplicate files
type DuplicateGroup struct {
	Keep   string   `json:"keep"`
	Delete []string `json:"delete"`
}

// DeleteOperation represents a file deletion operation
type DeleteOperation struct {
	Path  string `json:"path"`
	Issue string `json:"issue"`
}

// TodoItem represents a todo list item
type TodoItem struct {
	Category string `json:"category"`
	File     string `json:"file"`
	Message  string `json:"message"`
}

// OperationsOutput represents the complete JSON output
type OperationsOutput struct {
	Renames                   []RenameOperation  `json:"renames"`
	DuplicateDeletes          []DuplicateGroup   `json:"duplicate_deletes"`
	SmallOrCorruptedDeletes   []DeleteOperation  `json:"small_or_corrupted_deletes"`
	TodoItems                 []TodoItem         `json:"todo_items"`
}

// FileIssue represents different types of file issues
type FileIssue string

const (
	FileIssueFailedDownload FileIssue = "failed_download"
	FileIssueTooSmall       FileIssue = "too_small"
	FileIssueCorruptedPdf   FileIssue = "corrupted_pdf"
	FileIssueReadError      FileIssue = "read_error"
)

// Config holds the application configuration
type Config struct {
	Path              string
	DryRun            bool
	MaxDepth          uint
	NoRecursive       bool
	Extensions        []string
	NoDelete          bool
	TodoFile          *string
	LogFile           *string
	PreserveUnicode   bool
	FetchArxiv        bool
	Verbose           bool
	DeleteSmall       bool
	Json              bool
}
