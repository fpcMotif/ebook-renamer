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
