package scanner

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestScannerCreatesCorrectFileInfo(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "test_book.pdf")
	// Create content larger than 1KB
	largeContent := ""
	for i := 0; i < 50; i++ {
		largeContent += "This is a test file that is definitely larger than 1KB. "
	}
	err = os.WriteFile(testFile, []byte(largeContent), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)

	fileInfo := files[0]
	assert.Equal(t, "test_book.pdf", fileInfo.OriginalName)
	assert.Equal(t, ".pdf", fileInfo.Extension)
	assert.False(t, fileInfo.IsFailedDownload)
	assert.False(t, fileInfo.IsTooSmall)
	assert.True(t, fileInfo.ModifiedTime.Before(time.Now().Add(time.Second)))
}

func TestScannerDetectsTarGz(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "arXiv-2012.08669v1.tar.gz")
	err = os.WriteFile(testFile, []byte("test content"), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Equal(t, ".tar.gz", files[0].Extension)
}

func TestScannerDetectsDownloadFiles(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "test_book.pdf.download")
	err = os.WriteFile(testFile, []byte(""), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.True(t, files[0].IsFailedDownload)
}

func TestScannerDetectsSmallFiles(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "tiny.pdf")
	err = os.WriteFile(testFile, []byte("x"), 0644) // 1 byte
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.True(t, files[0].IsTooSmall)
}

func TestScannerSkipsHiddenFiles(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, ".hidden.pdf")
	err = os.WriteFile(testFile, []byte("content"), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 0)
}

// ========== EDGE CASE TESTS ==========

func TestScannerDetectsCrdownloadFiles(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "test_book.pdf.crdownload")
	err = os.WriteFile(testFile, []byte("partial content"), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.True(t, files[0].IsFailedDownload)
	assert.Equal(t, ".crdownload", files[0].Extension)
}

func TestScannerEmptyDirectory(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 0)
}

func TestScannerMultipleFileTypes(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	largeContent := make([]byte, 2000)

	os.WriteFile(filepath.Join(tmpDir, "book.pdf"), largeContent, 0644)
	os.WriteFile(filepath.Join(tmpDir, "book.epub"), largeContent, 0644)
	os.WriteFile(filepath.Join(tmpDir, "notes.txt"), largeContent, 0644)
	os.WriteFile(filepath.Join(tmpDir, "archive.tar.gz"), largeContent, 0644)
	os.WriteFile(filepath.Join(tmpDir, "document.mobi"), largeContent, 0644)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 5)

	extensions := make([]string, len(files))
	for i, f := range files {
		extensions[i] = f.Extension
	}
	assert.Contains(t, extensions, ".pdf")
	assert.Contains(t, extensions, ".epub")
	assert.Contains(t, extensions, ".txt")
	assert.Contains(t, extensions, ".tar.gz")
	assert.Contains(t, extensions, ".mobi")
}

func TestScannerMaxDepthLimit(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	largeContent := make([]byte, 2000)

	// Create file at depth 1
	os.WriteFile(filepath.Join(tmpDir, "book1.pdf"), largeContent, 0644)

	// Create file at depth 2
	subdir := filepath.Join(tmpDir, "subdir")
	os.Mkdir(subdir, 0755)
	os.WriteFile(filepath.Join(subdir, "book2.pdf"), largeContent, 0644)

	// Create file at depth 3
	subsubdir := filepath.Join(subdir, "subsubdir")
	os.Mkdir(subsubdir, 0755)
	os.WriteFile(filepath.Join(subsubdir, "book3.pdf"), largeContent, 0644)

	// With max_depth=1, should only find book1.pdf
	scanner, _ := New(tmpDir, 1)
	files, _ := scanner.Scan()
	assert.Len(t, files, 1)

	// With max_depth=2, should find book1.pdf and book2.pdf
	scanner, _ = New(tmpDir, 2)
	files, _ = scanner.Scan()
	assert.Len(t, files, 2)

	// With max_depth=3, should find all three
	scanner, _ = New(tmpDir, 3)
	files, _ = scanner.Scan()
	assert.Len(t, files, 3)
}

func TestScannerEpubSmallFileDetection(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "tiny.epub")
	err = os.WriteFile(testFile, []byte("x"), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.True(t, files[0].IsTooSmall)
}

func TestScannerSizeThresholdExactly1KB(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	// Exactly 1024 bytes should NOT be too small
	testFile := filepath.Join(tmpDir, "exact_1kb.pdf")
	content := make([]byte, 1024)
	err = os.WriteFile(testFile, content, 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.False(t, files[0].IsTooSmall)
}

func TestScannerSizeThreshold1023Bytes(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	// 1023 bytes should be too small
	testFile := filepath.Join(tmpDir, "under_1kb.pdf")
	content := make([]byte, 1023)
	err = os.WriteFile(testFile, content, 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.True(t, files[0].IsTooSmall)
}

func TestScannerFileWithoutExtension(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	testFile := filepath.Join(tmpDir, "README")
	err = os.WriteFile(testFile, []byte("content"), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Equal(t, "", files[0].Extension)
	assert.Equal(t, "README", files[0].OriginalName)
}

func TestScannerUnicodeFilename(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	largeContent := make([]byte, 2000)
	testFile := filepath.Join(tmpDir, "数学入門.pdf")
	err = os.WriteFile(testFile, largeContent, 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Equal(t, "数学入門.pdf", files[0].OriginalName)
	assert.Equal(t, ".pdf", files[0].Extension)
}

func TestScannerFileSizeRecorded(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	content := "Hello, World!" // 13 bytes
	testFile := filepath.Join(tmpDir, "small.txt")
	err = os.WriteFile(testFile, []byte(content), 0644)
	assert.NoError(t, err)

	scanner, err := New(tmpDir, 1)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Equal(t, uint64(13), files[0].Size)
}

func TestScannerPreservesRelativePathStructure(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "scanner_test")
	assert.NoError(t, err)
	defer os.RemoveAll(tmpDir)

	largeContent := make([]byte, 2000)
	subdir := filepath.Join(tmpDir, "books", "2024")
	os.MkdirAll(subdir, 0755)
	os.WriteFile(filepath.Join(subdir, "book.pdf"), largeContent, 0644)

	scanner, err := New(tmpDir, 10)
	assert.NoError(t, err)

	files, err := scanner.Scan()
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Contains(t, files[0].OriginalPath, "books")
	assert.Contains(t, files[0].OriginalPath, "2024")
}
