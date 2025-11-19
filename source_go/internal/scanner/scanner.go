package scanner

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/ebook-renamer/go/internal/types"
	"github.com/rs/zerolog/log"
)

// Scanner handles file scanning operations
type Scanner struct {
	RootPath string
	MaxDepth int
}

// New creates a new Scanner instance
func New(path string, maxDepth int) (*Scanner, error) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}

	info, err := os.Stat(absPath)
	if err != nil {
		return nil, err
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("path is not a directory: %s", path)
	}

	return &Scanner{
		RootPath: absPath,
		MaxDepth: maxDepth,
	}, nil
}

// Scan walks the directory tree and returns a list of interesting files
func (s *Scanner) Scan() ([]*types.FileInfo, error) {
	var files []*types.FileInfo

	err := filepath.Walk(s.RootPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			log.Warn().Err(err).Str("path", path).Msg("Error accessing path")
			return nil // Continue walking
		}

		// Calculate depth
		relPath, err := filepath.Rel(s.RootPath, path)
		if err != nil {
			return nil
		}
		depth := len(strings.Split(relPath, string(os.PathSeparator)))
		if relPath == "." {
			depth = 0
		}

		if depth > s.MaxDepth {
			if info.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}

		// Skip directories and hidden files/system dirs
		if info.IsDir() {
			if s.shouldSkip(path) {
				return filepath.SkipDir
			}
			return nil
		}

		if s.shouldSkip(path) {
			return nil
		}

		// Create FileInfo
		fileInfo, err := s.createFileInfo(path, info)
		if err != nil {
			log.Warn().Err(err).Str("path", path).Msg("Error creating file info")
			return nil
		}

		if fileInfo != nil {
			files = append(files, fileInfo)
		}

		return nil
	})

	if err != nil {
		return nil, err
	}

	log.Debug().Int("count", len(files)).Msg("Scanner found files")
	return files, nil
}

func (s *Scanner) createFileInfo(path string, info os.FileInfo) (*types.FileInfo, error) {
	originalName := info.Name()
	size := uint64(info.Size())
	modifiedTime := info.ModTime()

	// Detect extension
	extension := ""
	if strings.HasSuffix(originalName, ".tar.gz") {
		extension = ".tar.gz"
	} else if strings.HasSuffix(originalName, ".download") {
		extension = ".download"
	} else if strings.HasSuffix(originalName, ".crdownload") {
		extension = ".crdownload"
	} else {
		extension = filepath.Ext(originalName)
	}

	isFailedDownload := strings.HasSuffix(originalName, ".download") || strings.HasSuffix(originalName, ".crdownload")

	// Only check size for PDF and EPUB files
	isEbook := extension == ".pdf" || extension == ".epub"
	isTooSmall := !isFailedDownload && isEbook && size < 1024 // Less than 1KB

	return &types.FileInfo{
		OriginalPath:     path,
		OriginalName:     originalName,
		Extension:        extension,
		Size:             size,
		ModifiedTime:     modifiedTime,
		IsFailedDownload: isFailedDownload,
		IsTooSmall:       isTooSmall,
		NewName:          nil,
		NewPath:          "", // Will be set during normalization
	}, nil
}

func (s *Scanner) shouldSkip(path string) bool {
	filename := filepath.Base(path)

	// Skip hidden files/folders
	if strings.HasPrefix(filename, ".") {
		return true
	}

	info, err := os.Stat(path)
	if err == nil && info.IsDir() {
		// Skip download folders
		if strings.HasSuffix(filename, ".download") || strings.HasSuffix(filename, ".crdownload") {
			return true
		}
	}

	// Skip known system directories
	skipDirs := []string{"Xcode", "node_modules", ".git", "__pycache__"}
	for _, d := range skipDirs {
		if filename == d {
			return true
		}
	}

	return false
}
