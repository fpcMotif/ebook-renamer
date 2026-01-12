package cloud

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestIsCloudStoragePath(t *testing.T) {
	tests := []struct {
		name       string
		path       string
		expectNil  bool
		expectType Provider
	}{
		{"Dropbox path", "/Users/user/Dropbox", false, Dropbox},
		{"Google Drive path", "/Users/user/Google Drive", false, GoogleDrive},
		{"OneDrive path", "/Users/user/OneDrive", false, OneDrive},
		{"Local path", "/tmp", true, 0},
		{"Home directory", "/Users/user", true, 0},
		{"Documents folder", "/Users/user/Documents", true, 0},
		{"macOS CloudStorage Dropbox", "/Users/user/Library/CloudStorage/Dropbox", false, Dropbox},
		{"macOS CloudStorage GoogleDrive", "/Users/user/Library/CloudStorage/GoogleDrive", false, GoogleDrive},
		{"macOS CloudStorage OneDrive", "/Users/user/Library/CloudStorage/OneDrive", false, OneDrive},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsCloudStoragePath(tt.path)
			if tt.expectNil {
				assert.Nil(t, result, "Cloud detection for %s should be nil", tt.path)
			} else {
				assert.NotNil(t, result, "Cloud detection for %s should not be nil", tt.path)
				assert.Equal(t, tt.expectType, *result, "Cloud provider for %s should match", tt.path)
			}
		})
	}
}

func TestCloudModeWarning(t *testing.T) {
	tests := []struct {
		name     string
		provider Provider
	}{
		{"Dropbox", Dropbox},
		{"Google Drive", GoogleDrive},
		{"OneDrive", OneDrive},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			warning := CloudModeWarning(tt.provider)
			assert.NotEmpty(t, warning, "Warning for %s should not be empty", tt.name)
			assert.Contains(t, warning, tt.provider.String(), "Warning should contain provider name")
			assert.Contains(t, warning, "metadata-only", "Warning should mention metadata-only mode")
		})
	}
}

func TestProviderString(t *testing.T) {
	tests := []struct {
		provider Provider
		expected string
	}{
		{Dropbox, "Dropbox"},
		{GoogleDrive, "Google Drive"},
		{OneDrive, "OneDrive"},
	}

	for _, tt := range tests {
		t.Run(tt.expected, func(t *testing.T) {
			assert.Equal(t, tt.expected, tt.provider.String())
		})
	}
}
