package todo

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/ebook-renamer/go/internal/types"
)

// TodoList manages todo items and file issues
type TodoList struct {
	items              []string
	todoFilePath       string
	failedDownloads    []string
	smallFiles         []string
	corruptedFiles     []string
	otherIssues        []string
}

// New creates a new TodoList instance
func New(todoFilePath, targetDir string) (*TodoList, error) {
	// Determine todo file path
	if todoFilePath == "" {
		todoFilePath = filepath.Join(targetDir, "todo.md")
	}

	// Try to read existing todo.md to avoid duplicates
	var existingItems []string
	if _, err := os.Stat(todoFilePath); err == nil {
		content, err := os.ReadFile(todoFilePath)
		if err == nil {
			existingItems = extractItemsFromMD(string(content))
		}
	}

	return &TodoList{
		items:           existingItems,
		todoFilePath:    todoFilePath,
		failedDownloads: []string{},
		smallFiles:      []string{},
		corruptedFiles:  []string{},
		otherIssues:     []string{},
	}, nil
}

// AddFileIssue adds a file issue to the todo list
func (tl *TodoList) AddFileIssue(fileInfo *types.FileInfo, issue types.FileIssue) error {
	var item string
	
	switch issue {
	case types.FileIssueFailedDownload:
		item = fmt.Sprintf("重新下载: %s (未完成下载)", fileInfo.OriginalName)
	case types.FileIssueTooSmall:
		item = fmt.Sprintf("检查并重新下载: %s (文件过小，仅 %d 字节)", fileInfo.OriginalName, fileInfo.Size)
	case types.FileIssueCorruptedPdf:
		item = fmt.Sprintf("重新下载: %s (PDF文件损坏或格式无效)", fileInfo.OriginalName)
	case types.FileIssueReadError:
		item = fmt.Sprintf("检查文件权限: %s (无法读取文件)", fileInfo.OriginalName)
	default:
		item = fmt.Sprintf("检查文件: %s (未知问题)", fileInfo.OriginalName)
	}

	// Check if item already exists
	for _, existing := range tl.items {
		if existing == item {
			return nil // Already exists
		}
	}

	// Add to appropriate category list
	switch issue {
	case types.FileIssueFailedDownload:
		tl.failedDownloads = append(tl.failedDownloads, item)
	case types.FileIssueTooSmall:
		tl.smallFiles = append(tl.smallFiles, item)
	case types.FileIssueCorruptedPdf:
		tl.corruptedFiles = append(tl.corruptedFiles, item)
	default:
		tl.otherIssues = append(tl.otherIssues, item)
	}

	tl.items = append(tl.items, item)
	return nil
}

// AddFailedDownload adds a failed download file to the todo list
func (tl *TodoList) AddFailedDownload(fileInfo *types.FileInfo) error {
	if fileInfo.IsFailedDownload {
		return tl.AddFileIssue(fileInfo, types.FileIssueFailedDownload)
	} else if fileInfo.IsTooSmall {
		return tl.AddFileIssue(fileInfo, types.FileIssueTooSmall)
	}
	return nil
}

// AnalyzeFileIntegrity analyzes file integrity and adds issues if found
func (tl *TodoList) AnalyzeFileIntegrity(fileInfo *types.FileInfo) error {
	// Skip if already marked as failed or too small
	if fileInfo.IsFailedDownload || fileInfo.IsTooSmall {
		return nil
	}

	// Check PDF integrity for PDF files
	if strings.ToLower(fileInfo.Extension) == ".pdf" {
		if err := validatePDFHeader(fileInfo.OriginalPath); err != nil {
			return tl.AddFileIssue(fileInfo, types.FileIssueCorruptedPdf)
		}
	}

	// Check file readability
	if _, err := os.Stat(fileInfo.OriginalPath); err != nil {
		return tl.AddFileIssue(fileInfo, types.FileIssueReadError)
	}

	return nil
}

// RemoveFileFromTodo removes items containing the filename from all lists
func (tl *TodoList) RemoveFileFromTodo(filename string) {
	filenameLower := strings.ToLower(filename)
	
	// Remove from main items list
	var newItems []string
	for _, item := range tl.items {
		if !strings.Contains(strings.ToLower(item), filenameLower) {
			newItems = append(newItems, item)
		}
	}
	tl.items = newItems

	// Remove from category lists
	tl.failedDownloads = filterList(tl.failedDownloads, filenameLower)
	tl.smallFiles = filterList(tl.smallFiles, filenameLower)
	tl.corruptedFiles = filterList(tl.corruptedFiles, filenameLower)
	tl.otherIssues = filterList(tl.otherIssues, filenameLower)
}

// Write writes the todo list to the markdown file
func (tl *TodoList) Write() error {
	content := tl.generateTodoMD()
	return os.WriteFile(tl.todoFilePath, []byte(content), 0644)
}

// GetItems returns all todo items
func (tl *TodoList) GetItems() []string {
	return tl.items
}

// extractItemsFromMD extracts todo items from markdown content
func extractItemsFromMD(content string) []string {
	// Skip generic checklist items
	skipPatterns := []string{
		"检查所有未完成下载文件",
		"重新下载过小文件",
		"验证损坏的PDF文件",
		"处理其他文件问题",
		"MD5校验重复文件",
	}
	
	var items []string
	lines := strings.Split(content, "\n")
	
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "- [") {
			// Extract item text
			item := strings.TrimPrefix(line, "- [ ] ")
			item = strings.TrimPrefix(item, "- [x] ")
			item = strings.TrimSpace(item)
			
			// Skip if matches any skip pattern
			shouldSkip := false
			for _, pattern := range skipPatterns {
				if strings.Contains(item, pattern) {
					shouldSkip = true
					break
				}
			}
			
			if !shouldSkip && item != "" {
				items = append(items, item)
			}
		}
	}
	
	return items
}

// validatePDFHeader validates that a PDF file has the correct header
func validatePDFHeader(filePath string) error {
	file, err := os.Open(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	header := make([]byte, 5)
	_, err = file.Read(header)
	if err != nil {
		return err
	}

	// PDF files should start with "%PDF-"
	if string(header) != "%PDF-" {
		return fmt.Errorf("invalid PDF header")
	}

	return nil
}

// generateTodoMD generates the markdown content for the todo list
func (tl *TodoList) generateTodoMD() string {
	var md strings.Builder

	md.WriteString("# 需要检查的任务\n\n")
	md.WriteString(fmt.Sprintf("更新时间: %s\n\n", time.Now().Format("2006-01-02 15:04:05")))

	if len(tl.failedDownloads) > 0 {
		md.WriteString("## 🔄 未完成下载文件（.download）\n\n")
		for _, item := range tl.failedDownloads {
			md.WriteString(fmt.Sprintf("- [ ] %s\n", item))
		}
		md.WriteString("\n")
	}

	if len(tl.smallFiles) > 0 {
		md.WriteString("## 📁 异常小文件（< 1KB）\n\n")
		for _, item := range tl.smallFiles {
			md.WriteString(fmt.Sprintf("- [ ] %s\n", item))
		}
		md.WriteString("\n")
	}

	if len(tl.corruptedFiles) > 0 {
		md.WriteString("## 🚨 损坏的PDF文件\n\n")
		for _, item := range tl.corruptedFiles {
			md.WriteString(fmt.Sprintf("- [ ] %s\n", item))
		}
		md.WriteString("\n")
	}

	if len(tl.otherIssues) > 0 {
		md.WriteString("## ⚠️ 其他文件问题\n\n")
		for _, item := range tl.otherIssues {
			md.WriteString(fmt.Sprintf("- [ ] %s\n", item))
		}
		md.WriteString("\n")
	}

	// Add other items that don't fit in categories
	var otherItems []string
	for _, item := range tl.items {
		isInCategory := false
		for _, catItem := range tl.failedDownloads {
			if item == catItem {
				isInCategory = true
				break
			}
		}
		for _, catItem := range tl.smallFiles {
			if item == catItem {
				isInCategory = true
				break
			}
		}
		for _, catItem := range tl.corruptedFiles {
			if item == catItem {
				isInCategory = true
				break
			}
		}
		for _, catItem := range tl.otherIssues {
			if item == catItem {
				isInCategory = true
				break
			}
		}
		
		if !isInCategory {
			otherItems = append(otherItems, item)
		}
	}

	if len(otherItems) > 0 {
		md.WriteString("## 📋 其他需要处理的文件\n\n")
		for _, item := range otherItems {
			md.WriteString(fmt.Sprintf("- [ ] %s\n", item))
		}
		md.WriteString("\n")
	}

	if len(tl.failedDownloads) == 0 && len(tl.smallFiles) == 0 && len(tl.corruptedFiles) == 0 && len(tl.otherIssues) == 0 && len(otherItems) == 0 {
		md.WriteString("✅ 所有文件已检查完毕，无需处理的问题。\n\n")
	}

	md.WriteString("---\n")
	md.WriteString("*此文件由 ebook renamer 自动生成*\n")

	return md.String()
}

// filterList removes items containing the filename from a list
func filterList(list []string, filename string) []string {
	var result []string
	for _, item := range list {
		if !strings.Contains(strings.ToLower(item), filename) {
			result = append(result, item)
		}
	}
	return result
}
