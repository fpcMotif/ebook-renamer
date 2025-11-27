package todo

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/ebook-renamer/go/internal/types"
	"github.com/stretchr/testify/assert"
)

func TestTodoListNewAndWrite(t *testing.T) {
	// Create temp dir
	tmpDir := t.TempDir()
	todoFile := filepath.Join(tmpDir, "todo.md")

	// Create new TodoList
	tl, err := New(todoFile, tmpDir)
	assert.NoError(t, err)
	assert.NotNil(t, tl)

	// Add an item
	fileInfo := &types.FileInfo{
		OriginalName: "test_file.download",
		OriginalPath: filepath.Join(tmpDir, "test_file.download"),
		Size:         100,
		IsFailedDownload: true,
	}

	err = tl.AddFailedDownload(fileInfo)
	assert.NoError(t, err)

	// Write to file
	err = tl.Write()
	assert.NoError(t, err)

	// Verify file content
	content, err := os.ReadFile(todoFile)
	assert.NoError(t, err)
	assert.Contains(t, string(content), "test_file.download")
	assert.Contains(t, string(content), "未完成下载")
}

func TestExtractItemsFromMD(t *testing.T) {
	content := `# Todo List
- [ ] Item 1
- [x] Item 2
- [ ] 检查所有未完成下载文件 (should be skipped)
`
	items := extractItemsFromMD(content)

	assert.Contains(t, items, "Item 1")
	assert.Contains(t, items, "Item 2")
	assert.NotContains(t, items, "检查所有未完成下载文件")
}

func TestAnalyzeFileIntegrityCorruptedPDF(t *testing.T) {
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "bad.pdf")

	// Create a bad PDF file (wrong header)
	err := os.WriteFile(filePath, []byte("NOT PDF CONTENT"), 0644)
	assert.NoError(t, err)

	tl, _ := New("", tmpDir)
	fileInfo := &types.FileInfo{
		OriginalName: "bad.pdf",
		OriginalPath: filePath,
		Extension:    ".pdf",
		Size:         100,
	}

	err = tl.AnalyzeFileIntegrity(fileInfo)
	assert.NoError(t, err)

	// Should be in corrupted files
	assert.Contains(t, tl.corruptedFiles[0], "bad.pdf")
	assert.Contains(t, tl.corruptedFiles[0], "损坏")
}

func TestAnalyzeFileIntegrityValidPDF(t *testing.T) {
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "good.pdf")

	// Create a valid PDF header
	err := os.WriteFile(filePath, []byte("%PDF-1.4\n..."), 0644)
	assert.NoError(t, err)

	tl, _ := New("", tmpDir)
	fileInfo := &types.FileInfo{
		OriginalName: "good.pdf",
		OriginalPath: filePath,
		Extension:    ".pdf",
		Size:         100,
	}

	err = tl.AnalyzeFileIntegrity(fileInfo)
	assert.NoError(t, err)

	// Should NOT be in corrupted files
	assert.Empty(t, tl.corruptedFiles)
}

func TestRemoveFileFromTodo(t *testing.T) {
	tl, _ := New("", ".")

	fileInfo := &types.FileInfo{OriginalName: "remove_me.pdf"}
	tl.AddFileIssue(fileInfo, types.FileIssueCorruptedPdf)

	assert.NotEmpty(t, tl.corruptedFiles)
	assert.NotEmpty(t, tl.items)

	tl.RemoveFileFromTodo("remove_me.pdf")

	assert.Empty(t, tl.corruptedFiles)
	assert.Empty(t, tl.items)
}
