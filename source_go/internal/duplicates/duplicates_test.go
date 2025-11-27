package duplicates

import (
    "os"
	"path/filepath"
	"testing"
	"time"

	"github.com/ebook-renamer/go/internal/types"
	"github.com/stretchr/testify/assert"
)

func TestStripVariantSuffix(t *testing.T) {
	testCases := []struct {
		input    string
		expected string
	}{
		{"file (1).pdf", "file.pdf"},
		{"file (2).txt", "file.txt"},
		{"file (10).epub", "file.epub"},
		{"file.pdf", "file.pdf"},
		{"file (text).pdf", "file (text).pdf"}, // Non-numeric content kept
		{"file (1) (2).pdf", "file (1).pdf"},   // Only last one removed
		{"file", "file"},
		{"file (1)", "file"},
	}

	for _, tc := range testCases {
		assert.Equal(t, tc.expected, stripVariantSuffix(tc.input), "Input: %s", tc.input)
	}
}

func TestSelectFileToKeep(t *testing.T) {
	// Setup mock files
	now := time.Now()

	// Case 1: Normalized vs Unnormalized
	normFile := &types.FileInfo{
		OriginalPath: "/path/normalized.pdf",
		OriginalName: "normalized.pdf",
		ModifiedTime: now,
		Extension: ".pdf",
	}
	name := "normalized.pdf"
	normFile.NewName = &name

	unNormFile := &types.FileInfo{
		OriginalPath: "/path/raw.pdf",
		OriginalName: "raw.pdf",
		ModifiedTime: now,
		Extension: ".pdf",
	}

	t.Run("Prioritize normalized", func(t *testing.T) {
		kept := selectFileToKeep([]*types.FileInfo{normFile, unNormFile})
		assert.Equal(t, normFile, kept)
	})

	// Case 2: Shortest Path
	shortPath := &types.FileInfo{
		OriginalPath: "/a/file.pdf",
		OriginalName: "file.pdf",
		ModifiedTime: now,
		Extension: ".pdf",
	}
	longPath := &types.FileInfo{
		OriginalPath: "/a/b/c/file.pdf",
		OriginalName: "file.pdf",
		ModifiedTime: now,
		Extension: ".pdf",
	}

	t.Run("Prioritize shortest path", func(t *testing.T) {
		kept := selectFileToKeep([]*types.FileInfo{shortPath, longPath})
		assert.Equal(t, shortPath, kept)
	})

	// Case 3: Newest time
	oldFile := &types.FileInfo{
		OriginalPath: "/path/file1.pdf",
		OriginalName: "file1.pdf",
		ModifiedTime: now.Add(-1 * time.Hour),
		Extension: ".pdf",
	}
	newFile := &types.FileInfo{
		OriginalPath: "/path/file2.pdf",
		OriginalName: "file2.pdf",
		ModifiedTime: now,
		Extension: ".pdf",
	}

	t.Run("Prioritize newest file", func(t *testing.T) {
		kept := selectFileToKeep([]*types.FileInfo{oldFile, newFile})
		assert.Equal(t, newFile, kept)
	})
}

func TestDetectNameVariants(t *testing.T) {
	name1 := "Book.pdf"
	name2 := "Book (1).pdf"
	name3 := "Other.pdf"

	file1 := &types.FileInfo{NewName: &name1}
	file2 := &types.FileInfo{NewName: &name2}
	file3 := &types.FileInfo{NewName: &name3}

	variants := DetectNameVariants([]*types.FileInfo{file1, file2, file3})

	assert.Len(t, variants, 1) // One group
	assert.Len(t, variants[0], 2) // Two files in that group
	// Indices 0 and 1 should be in the group
	assert.Contains(t, variants[0], 0)
	assert.Contains(t, variants[0], 1)
}

func TestComputeMD5(t *testing.T) {
	// Create temp file
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "test.txt")
	err := os.WriteFile(filePath, []byte("test content"), 0644)
	assert.NoError(t, err)

	hash, err := computeMD5(filePath)
	assert.NoError(t, err)
	assert.NotEmpty(t, hash)

	// "test content" md5 -> 9473fdd0d880a43c21b7778d34872157
	assert.Equal(t, "9473fdd0d880a43c21b7778d34872157", hash)
}
