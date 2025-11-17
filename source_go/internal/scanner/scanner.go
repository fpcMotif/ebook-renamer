package scanner

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/ebook-renamer/go/internal/types"
)

// Scanner handles file scanning operations
type Scanner struct {
	rootPath  string
	maxDepth  uint
}

// New creates a new Scanner instance
func New(rootPath string, maxDepth uint) *Scanner {
	return &Scanner{
		rootPath: rootPath,
		maxDepth: maxDepth,
	}
}

// Scan scans the directory for files matching criteria
func (s *Scanner) Scan() ([]types.FileInfo, error) {
	var files []types.FileInfo

	err := filepath.Walk(s.rootPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip files that can't be accessed
		}

		// Skip directories
		if info.IsDir() {
			return nil
		}

		// Check depth
		depth, err := s.calculateDepth(path)
		if err != nil {
			return nil
		}
		if depth > s.maxDepth {
			return nil
		}

		// Skip hidden files and system directories
		if s.shouldSkip(path) {
			return nil
		}

		// Create file info
		fileInfo, err := s.createFileInfo(path)
		if err != nil {
			return nil // Skip files that can't be processed
		}

		files = append(files, *fileInfo)
		return nil
	})

	if err != nil {
		return nil, fmt.Errorf("scan failed: %w", err)
	}

	return files, nil
}

// calculateDepth calculates the depth of a path relative to the root
func (s *Scanner) calculateDepth(path string) (uint, error) {
	relPath, err := filepath.Rel(s.rootPath, path)
	if err != nil {
		return 0, err
	}
	
	if relPath == "." {
		return 0, nil
	}
	
	return uint(strings.Count(relPath, string(filepath.Separator))), nil
}

// shouldSkip determines if a path should be skipped
func (s *Scanner) shouldSkip(path string) bool {
	filename := filepath.Base(path)
	
	// Skip hidden files/folders
	if strings.HasPrefix(filename, ".") {
		return true
	}

	// Skip known system directories
	skipDirs := []string{"Xcode", "node_modules", ".git", "__pycache__"}
	for _, dir := range skipDirs {
		if filename == dir {
			return true
		}
	}

	return false
}

// createFileInfo creates a FileInfo struct for the given path
func (s *Scanner) createFileInfo(path string) (*types.FileInfo, error) {
	stat, err := os.Stat(path)
	if err != nil {
		return nil, err
	}

	originalName := filepath.Base(path)
	
	// Detect extension (including .tar.gz)
	var extension string
	if strings.HasSuffix(originalName, ".tar.gz") {
		extension = ".tar.gz"
	} else if strings.HasSuffix(originalName, ".download") {
		extension = ".download"
	} else if strings.HasSuffix(originalName, ".crdownload") {
		extension = ".crdownload"
	} else {
		ext := filepath.Ext(originalName)
		if ext != "" {
			extension = ext
		} else {
			extension = ""
		}
	}

	// Detect failed downloads
	isFailedDownload := strings.HasSuffix(originalName, ".download") || strings.HasSuffix(originalName, ".crdownload")
	
	// Check if file is too small (only for PDF and EPUB files)
	isEbook := extension == ".pdf" || extension == ".epub"
	isTooSmall := !isFailedDownload && isEbook && stat.Size() < 1024 // Less than 1KB

	return &types.FileInfo{
		OriginalPath:     path,
		OriginalName:     originalName,
		Extension:        extension,
		Size:             uint64(stat.Size()),
		ModifiedTime:     stat.ModTime(),
		IsFailedDownload: isFailedDownload,
		IsTooSmall:       isTooSmall,
		NewName:          nil,
		NewPath:          path,
	}, nil
}
