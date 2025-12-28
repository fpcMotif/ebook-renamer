package cloud

import (
	"github.com/stretchr/testify/assert"
	"path/filepath"
	"testing"
)

func TestIsCloudStoragePath(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		expected bool
	}{
		{"Dropbox path", "/Users/user/Dropbox", true},
		{"Google Drive path", "/Users/user/Google Drive", true},
		{"OneDrive path", "/Users/user/OneDrive", true},
		{"iCloud Drive path", "/Users/user/Library/Mobile Documents/com~apple~CloudDocs", true},
		{"pCloud path", "/Users/user/pCloudDrive", true},
		{"Local path", "/tmp", false},
		{"Home directory", "/Users/user", false},
		{"Documents folder", "/Users/user/Documents", false},
		{"Nested non-cloud", "/Users/user/Local/Dropbox/Folder", false}, // Dropbox detected by parent
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsCloudStoragePath(tt.path)
			assert.Equal(t, tt.expected, result, "Cloud detection for %s should be %v", tt.path, tt.expected)
		})
	}
}

func TestGetCloudProvider(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		expected string
	}{
		{"Dropbox", "/Users/user/Dropbox", "dropbox"},
		{"Google Drive", "/Users/user/Google Drive", "googledrive"},
		{"OneDrive", "/Users/user/OneDrive", "onedrive"},
		{"iCloud Drive", "/Users/user/Library/Mobile Documents/com~apple~CloudDocs", "icloud"},
		{"Non-cloud", "/tmp", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			provider := GetCloudProvider(tt.path)
			assert.Equal(t, tt.expected, provider.Name(), "Provider name for %s should be %s", tt.path, tt.expected)
		})
	}
}

func TestCloudModeWarning(t *testing.T) {
	tests := []struct {
		name     string
		provider string
		expected string
	}{
		{"Dropbox", "dropbox", "⚠️  Dropbox detected: Using metadata-only duplicate detection to avoid triggering file downloads."},
		{"Google Drive", "googledrive", "⚠️  Google Drive detected: Using metadata-only duplicate detection to avoid triggering file downloads."},
		{"OneDrive", "onedrive", "⚠️  OneDrive detected: Using metadata-only duplicate detection to avoid triggering file downloads."},
		{"iCloud", "icloud", "⚠️  iCloud Drive detected: Using metadata-only duplicate detection to avoid triggering file downloads."},
		{"None", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.expected == "" {
				// Should return empty string for non-cloud
				warning := CloudModeWarning(tt.provider)
				assert.Empty(t, warning)
			} else {
				warning := CloudModeWarning(tt.provider)
				assert.Equal(t, tt.expected, warning, "Warning for %s should match", tt.provider)
			}
		})
	}
}

func TestIsCloudFile(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		expected bool
	}{
		{"Cloud file", "/Users/user/Dropbook.pdf", true},
		{"Cloud file deep", "/Users/user/Dropbox/Folder/book.pdf", true},
		{"Local file", "/tmp/book.pdf", false},
		{"Relative path", "book.pdf", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsCloudFile(tt.path)
			assert.Equal(t, tt.expected, result, "Cloud file detection for %s should be %v", tt.path, tt.expected)
		})
	}
}
