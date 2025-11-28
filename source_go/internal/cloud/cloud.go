package cloud

import (
	"log"
	"strings"
)

// Provider represents a cloud storage provider
type Provider int

const (
	Dropbox Provider = iota
	GoogleDrive
	OneDrive
)

func (p Provider) String() string {
	switch p {
	case Dropbox:
		return "Dropbox"
	case GoogleDrive:
		return "Google Drive"
	case OneDrive:
		return "OneDrive"
	default:
		return "Unknown"
	}
}

// IsCloudStoragePath detects if a path is within a cloud storage directory
func IsCloudStoragePath(path string) *Provider {
	// Check for common cloud storage paths
	if strings.Contains(path, "Dropbox") {
		log.Printf("Detected Dropbox path: %s", path)
		p := Dropbox
		return &p
	}

	if strings.Contains(path, "Google Drive") || strings.Contains(path, "GoogleDrive") {
		log.Printf("Detected Google Drive path: %s", path)
		p := GoogleDrive
		return &p
	}

	if strings.Contains(path, "OneDrive") {
		log.Printf("Detected OneDrive path: %s", path)
		p := OneDrive
		return &p
	}

	// macOS CloudStorage paths
	if strings.Contains(path, "Library/CloudStorage/Dropbox") {
		log.Printf("Detected macOS CloudStorage Dropbox path: %s", path)
		p := Dropbox
		return &p
	}

	if strings.Contains(path, "Library/CloudStorage/GoogleDrive") {
		log.Printf("Detected macOS CloudStorage Google Drive path: %s", path)
		p := GoogleDrive
		return &p
	}

	if strings.Contains(path, "Library/CloudStorage/OneDrive") {
		log.Printf("Detected macOS CloudStorage OneDrive path: %s", path)
		p := OneDrive
		return &p
	}

	return nil
}

// CloudModeWarning returns a warning message for cloud mode
func CloudModeWarning(provider Provider) string {
	return "⚠️  Detected " + provider.String() + " storage. Using metadata-only mode to avoid downloading files.\n" +
		"Duplicate detection based on filename similarity (≥85%) + exact size match.\n" +
		"This is less accurate than content-based hashing. Review carefully!"
}
