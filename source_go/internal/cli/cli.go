package cli

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/ebook-renamer/go/internal/duplicates"
	"github.com/ebook-renamer/go/internal/jsonoutput"
	"github.com/ebook-renamer/go/internal/normalizer"
	"github.com/ebook-renamer/go/internal/scanner"
	"github.com/ebook-renamer/go/internal/todo"
	"github.com/ebook-renamer/go/internal/tui"
	"github.com/ebook-renamer/go/internal/types"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/spf13/cobra"
)

var (
	pathArg             string
	dryRunFlag          bool
	maxDepthFlag        string
	noRecursiveFlag     bool
	extensionsFlag      string
	noDeleteFlag        bool
	todoFileFlag        string
	logFileFlag         string
	preserveUnicodeFlag bool
	fetchArxivFlag      bool
	verboseFlag         bool
	deleteSmallFlag     bool
	autoCleanupFlag     bool
	jsonFlag            bool
)

var rootCmd = &cobra.Command{
	Use:   "ebook-renamer [PATH]",
	Short: "Batch rename and organize downloaded books and arXiv files",
	Long: `Batch rename and organize downloaded books and arXiv files.

This tool scans a directory for ebook files, normalizes their filenames,
detects duplicates, and generates a todo.md file for manual review.`,
	Args: cobra.MaximumNArgs(1),
	RunE: runEbookRenamer,
}

func init() {
	// Path argument (optional, defaults to current directory)
	rootCmd.Args = cobra.MaximumNArgs(1)

	// CLI flags matching Rust implementation
	rootCmd.Flags().BoolVarP(&dryRunFlag, "dry-run", "d", false, "Perform dry run: show changes without applying them (Note: todo.md is always written, even in dry-run mode)")
	rootCmd.Flags().StringVar(&maxDepthFlag, "max-depth", "18446744073709551615", "Maximum directory depth to traverse (default: unlimited)")
	rootCmd.Flags().BoolVar(&noRecursiveFlag, "no-recursive", false, "Only scan the top-level directory, no recursion")
	rootCmd.Flags().StringVar(&extensionsFlag, "extensions", "", "Comma-separated extensions to process (default: pdf,epub,txt)")
	rootCmd.Flags().BoolVar(&noDeleteFlag, "no-delete", false, "Don't delete duplicates, only list them")
	rootCmd.Flags().StringVar(&todoFileFlag, "todo-file", "", "Path to write todo.md (default: <target-dir>/todo.md)")
	rootCmd.Flags().StringVar(&logFileFlag, "log-file", "", "Optional path to write detailed operation log")
	rootCmd.Flags().BoolVar(&preserveUnicodeFlag, "preserve-unicode", false, "Preserve original non-Latin script (CJK, etc.) without modification")
	rootCmd.Flags().BoolVar(&fetchArxivFlag, "fetch-arxiv", false, "Fetch arXiv metadata via API (not implemented yet)")
	rootCmd.Flags().BoolVarP(&verboseFlag, "verbose", "v", false, "Enable verbose logging")
	rootCmd.Flags().BoolVar(&deleteSmallFlag, "delete-small", false, "Delete small/corrupted files (< 1KB) instead of adding to todo list")
	rootCmd.Flags().BoolVar(&autoCleanupFlag, "auto-cleanup", false, "Automatically clean up incomplete downloads (.download/.crdownload) and corrupted files")
	rootCmd.Flags().BoolVar(&jsonFlag, "json", false, "Output operations in JSON format instead of human-readable text")
}

func Execute() error {
	// Setup logging with timestamp
	log.SetFlags(log.Ldate | log.Ltime | log.Lmicroseconds)
	return rootCmd.Execute()
}

func runEbookRenamer(cmd *cobra.Command, args []string) error {
	// Handle path argument
	if len(args) == 0 {
		pathArg = "."
	} else {
		pathArg = args[0]
	}

	// Convert to absolute path
	absPath, err := filepath.Abs(pathArg)
	if err != nil {
		return fmt.Errorf("invalid path: %w", err)
	}

	// Check if path is a directory
	if stat, err := os.Stat(absPath); err != nil {
		return fmt.Errorf("path does not exist: %w", err)
	} else if !stat.IsDir() {
		return fmt.Errorf("path is not a directory: %s", absPath)
	}

	// Parse max depth
	maxDepth, err := strconv.ParseUint(maxDepthFlag, 10, 64)
	if err != nil {
		return fmt.Errorf("invalid max-depth: %w", err)
	}

	// Handle --no-recursive by setting max_depth to 1
	effectiveMaxDepth := maxDepth
	if noRecursiveFlag {
		effectiveMaxDepth = 1
	}

	// Parse extensions
	var extensions []string
	if extensionsFlag != "" {
		// Parse comma-separated extensions
		for _, ext := range strings.Split(extensionsFlag, ",") {
			ext = strings.TrimSpace(ext)
			if !strings.HasPrefix(ext, ".") {
				ext = "." + ext
			}
			if ext != "" {
				extensions = append(extensions, ext)
			}
		}
	} else {
		extensions = []string{".pdf", ".epub", ".txt"}
	}

	// Handle --fetch-arxiv placeholder
	if fetchArxivFlag {
		fmt.Fprintf(os.Stderr, "⚠️  Warning: --fetch-arxiv is not implemented yet. Files will be processed offline only.\n")
	}

	// Create config
	config := &types.Config{
		Path:            absPath,
		DryRun:          dryRunFlag,
		MaxDepth:        uint(effectiveMaxDepth),
		NoRecursive:     noRecursiveFlag,
		Extensions:      extensions,
		NoDelete:        noDeleteFlag,
		TodoFile:        nilString(todoFileFlag),
		LogFile:         nilString(logFileFlag),
		PreserveUnicode: preserveUnicodeFlag,
		FetchArxiv:      fetchArxivFlag,
		Verbose:         verboseFlag,
		DeleteSmall:     deleteSmallFlag,
		AutoCleanup:     autoCleanupFlag,
		Json:            jsonFlag,
	}

	log.Printf("Starting ebook renamer with config: %+v", config)

	if config.Json {
		return processFiles(config)
	}

	// Run TUI
	p := tea.NewProgram(tui.NewModel(config))
	if _, err := p.Run(); err != nil {
		return fmt.Errorf("error running program: %w", err)
	}
	return nil
}

func nilString(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func processFiles(config *types.Config) error {
	// Create scanner
	s, err := scanner.New(config.Path, config.MaxDepth)
	if err != nil {
		return fmt.Errorf("failed to create scanner: %w", err)
	}

	// Scan for files
	files, err := s.Scan()
	if err != nil {
		return fmt.Errorf("scan failed: %w", err)
	}
	log.Printf("Found %d files to process", len(files))

	// Normalize filenames
	normalized, err := normalizer.NormalizeFiles(files)
	if err != nil {
		return fmt.Errorf("normalization failed: %w", err)
	}
	log.Printf("Normalized %d files", len(normalized))

	// Determine todo file path
	todoFilePath := determineTodoFile(config.Path, config.TodoFile)

	// Create todo list
	todoList, err := todo.New(todoFilePath, config.Path)
	if err != nil {
		return fmt.Errorf("todo list creation failed: %w", err)
	}

	// Categorize problematic files
	var incompleteDownloads []*types.FileInfo // .download, .crdownload files
	var corruptedFiles []*types.FileInfo      // Corrupted PDFs
	var smallFiles []*types.FileInfo          // Files that are too small (< 1KB)

	for _, fileInfo := range normalized {
		if fileInfo.IsFailedDownload {
			incompleteDownloads = append(incompleteDownloads, fileInfo)
		} else if fileInfo.IsTooSmall {
			smallFiles = append(smallFiles, fileInfo)
		} else {
			// Check for PDF corruption
			if strings.ToLower(fileInfo.Extension) == ".pdf" {
				if err := validatePDFHeader(fileInfo.OriginalPath); err != nil {
					corruptedFiles = append(corruptedFiles, fileInfo)
				}
			}
		}
	}

	// Print summary of found issues
	if !config.Json {
		printIssueSummary(incompleteDownloads, corruptedFiles, smallFiles)
	}

	// Determine cleanup behavior based on flags
	shouldCleanup := config.AutoCleanup || config.DeleteSmall

	// Track files to delete and todo items
	var filesToDelete []string
	var todoItems []types.TodoItem
	cleanupResult := &types.CleanupResult{
		DeletedIncomplete: []string{},
		DeletedCorrupted:  []string{},
		DeletedSmall:      []string{},
		FailedDeletions:   []types.FailedDeletion{},
	}

	// Process incomplete downloads
	for _, fileInfo := range incompleteDownloads {
		if shouldCleanup {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			cleanupResult.DeletedIncomplete = append(cleanupResult.DeletedIncomplete, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else {
			todoList.AddFailedDownload(fileInfo)
			todoItems = append(todoItems, types.TodoItem{
				Category: "failed_download",
				File:     fileInfo.OriginalName,
				Message:  fmt.Sprintf("重新下载: %s (未完成下载)", fileInfo.OriginalName),
			})
		}
	}

	// Process corrupted files
	for _, fileInfo := range corruptedFiles {
		if shouldCleanup {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			cleanupResult.DeletedCorrupted = append(cleanupResult.DeletedCorrupted, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else {
			todoList.AddFileIssue(fileInfo, types.FileIssueCorruptedPdf)
			todoItems = append(todoItems, types.TodoItem{
				Category: "corrupted",
				File:     fileInfo.OriginalName,
				Message:  fmt.Sprintf("重新下载: %s (PDF文件损坏)", fileInfo.OriginalName),
			})
		}
	}

	// Process small files
	for _, fileInfo := range smallFiles {
		if config.DeleteSmall {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			cleanupResult.DeletedSmall = append(cleanupResult.DeletedSmall, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else if config.AutoCleanup {
			// Auto-cleanup mode: add to todo for manual review (might be valid small ebook)
			todoList.AddFailedDownload(fileInfo)
			todoItems = append(todoItems, types.TodoItem{
				Category: "too_small",
				File:     fileInfo.OriginalName,
				Message:  fmt.Sprintf("检查文件: %s (文件过小 %d 字节，可能需要重新下载)", fileInfo.OriginalName, fileInfo.Size),
			})
		} else {
			todoList.AddFailedDownload(fileInfo)
			todoItems = append(todoItems, types.TodoItem{
				Category: "too_small",
				File:     fileInfo.OriginalName,
				Message:  fmt.Sprintf("检查并重新下载: %s (文件过小，仅 %d 字节)", fileInfo.OriginalName, fileInfo.Size),
			})
		}
	}

	// Analyze other files for integrity
	for _, fileInfo := range normalized {
		isProblematic := false
		for _, f := range incompleteDownloads {
			if f == fileInfo {
				isProblematic = true
				break
			}
		}
		for _, f := range corruptedFiles {
			if f == fileInfo {
				isProblematic = true
				break
			}
		}
		for _, f := range smallFiles {
			if f == fileInfo {
				isProblematic = true
				break
			}
		}
		if !isProblematic {
			todoList.AnalyzeFileIntegrity(fileInfo)
		}
	}

	// Detect duplicates
	duplicateGroups, cleanFiles, err := duplicates.DetectDuplicates(normalized)
	if err != nil {
		return fmt.Errorf("duplicate detection failed: %w", err)
	}
	log.Printf("Detected %d duplicate groups", len(duplicateGroups))

	// Sort todo items by category, then file for deterministic output (matching Rust)
	sort.Slice(todoItems, func(i, j int) bool {
		if todoItems[i].Category != todoItems[j].Category {
			return todoItems[i].Category < todoItems[j].Category
		}
		return todoItems[i].File < todoItems[j].File
	})

	// Output results
	if config.DryRun {
		if config.Json {
			// JSON output
			output, err := jsonoutput.FromResults(cleanFiles, duplicateGroups, filesToDelete, todoItems, config.Path)
			if err != nil {
				return fmt.Errorf("JSON output generation failed: %w", err)
			}
			jsonStr, err := jsonoutput.ToJSON(output)
			if err != nil {
				return fmt.Errorf("JSON serialization failed: %w", err)
			}
			fmt.Println(jsonStr)
		} else {
			// Human-readable output
			printHumanOutput(cleanFiles, duplicateGroups, filesToDelete, todoList)
		}

		// Write todo.md even in dry-run mode
		if err := todoList.Write(); err != nil {
			return fmt.Errorf("todo write failed: %w", err)
		}

		if !config.Json {
			fmt.Println("\n✓ todo.md written (dry-run mode)")
		}
	} else {
		// Execute operations
		cleanupResult, err = executeOperations(cleanFiles, duplicateGroups, filesToDelete, todoList, config, cleanupResult)
		if err != nil {
			return fmt.Errorf("execution failed: %w", err)
		}

		// Print cleanup summary
		if !config.Json {
			printCleanupSummary(cleanupResult)
		}
	}

	if !config.Json {
		fmt.Println("\n✓ Operation completed successfully!")
	}

	return nil
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

	if string(header) != "%PDF-" {
		return fmt.Errorf("invalid PDF header")
	}

	return nil
}

func printIssueSummary(incomplete, corrupted, small []*types.FileInfo) {
	totalIssues := len(incomplete) + len(corrupted) + len(small)

	if totalIssues == 0 {
		fmt.Println("\n📋 文件扫描完成，未发现问题文件")
		return
	}

	fmt.Printf("\n📋 发现 %d 个问题文件:\n", totalIssues)
	fmt.Println("----------------------------------------")

	if len(incomplete) > 0 {
		fmt.Printf("  🔄 未完成下载: %d 个\n", len(incomplete))
		for i, f := range incomplete {
			if i >= 3 {
				fmt.Printf("     ... 及其他 %d 个文件\n", len(incomplete)-3)
				break
			}
			fmt.Printf("     • %s\n", f.OriginalName)
		}
	}

	if len(corrupted) > 0 {
		fmt.Printf("  🚨 损坏文件: %d 个\n", len(corrupted))
		for i, f := range corrupted {
			if i >= 3 {
				fmt.Printf("     ... 及其他 %d 个文件\n", len(corrupted)-3)
				break
			}
			fmt.Printf("     • %s\n", f.OriginalName)
		}
	}

	if len(small) > 0 {
		fmt.Printf("  📁 异常小文件: %d 个\n", len(small))
		for i, f := range small {
			if i >= 3 {
				fmt.Printf("     ... 及其他 %d 个文件\n", len(small)-3)
				break
			}
			fmt.Printf("     • %s (%d 字节)\n", f.OriginalName, f.Size)
		}
	}

	fmt.Println("----------------------------------------")
}

func printCleanupSummary(result *types.CleanupResult) {
	totalDeleted := len(result.DeletedIncomplete) + len(result.DeletedCorrupted) + len(result.DeletedSmall)

	if totalDeleted == 0 && len(result.FailedDeletions) == 0 {
		return
	}

	fmt.Println("\n🧹 清理完成:")
	fmt.Println("----------------------------------------")

	if len(result.DeletedIncomplete) > 0 {
		fmt.Printf("  ✓ 删除未完成下载: %d 个\n", len(result.DeletedIncomplete))
	}

	if len(result.DeletedCorrupted) > 0 {
		fmt.Printf("  ✓ 删除损坏文件: %d 个\n", len(result.DeletedCorrupted))
	}

	if len(result.DeletedSmall) > 0 {
		fmt.Printf("  ✓ 删除异常小文件: %d 个\n", len(result.DeletedSmall))
	}

	if len(result.FailedDeletions) > 0 {
		fmt.Printf("  ⚠️  删除失败: %d 个\n", len(result.FailedDeletions))
		for i, fd := range result.FailedDeletions {
			if i >= 3 {
				break
			}
			fmt.Printf("     • %s: %s\n", filepath.Base(fd.Path), fd.Error)
		}
	}

	fmt.Println("----------------------------------------")
}

func determineTodoFile(targetDir string, customPath *string) string {
	if customPath != nil {
		return *customPath
	}
	return filepath.Join(targetDir, "todo.md")
}

func printHumanOutput(cleanFiles []*types.FileInfo, duplicateGroups [][]string, filesToDelete []string, todoList *todo.TodoList) {
	fmt.Println("\n=== DRY RUN MODE ===")

	// Print renames
	for _, fileInfo := range cleanFiles {
		if fileInfo.NewName != nil {
			fmt.Printf("RENAME: %s -> %s\n", fileInfo.OriginalName, *fileInfo.NewName)
		}
	}

	// Print duplicate deletions
	for _, group := range duplicateGroups {
		if len(group) > 1 {
			fmt.Println("\nDELETE DUPLICATES:")
			for i, path := range group {
				if i == 0 {
					fmt.Printf("  KEEP: %s\n", path)
				} else {
					fmt.Printf("  DELETE: %s\n", path)
				}
			}
		}
	}

	// Print small/corrupted deletions
	if len(filesToDelete) > 0 {
		fmt.Println("\nDELETE SMALL/CORRUPTED FILES:")
		for _, path := range filesToDelete {
			fmt.Printf("  DELETE: %s\n", path)
		}
	}

	// Print todo items
	items := todoList.GetItems()
	if len(items) > 0 {
		fmt.Println("\nTODO LIST:")
		for _, item := range items {
			fmt.Printf("  - [ ] %s\n", item)
		}
	}
}

func executeOperations(cleanFiles []*types.FileInfo, duplicateGroups [][]string, filesToDelete []string, todoList *todo.TodoList, config *types.Config, cleanupResult *types.CleanupResult) (*types.CleanupResult, error) {
	// Execute renames
	for _, fileInfo := range cleanFiles {
		if fileInfo.NewName != nil {
			if err := os.Rename(fileInfo.OriginalPath, fileInfo.NewPath); err != nil {
				return cleanupResult, fmt.Errorf("rename failed: %w", err)
			}
			log.Printf("Renamed: %s -> %s", fileInfo.OriginalName, *fileInfo.NewName)
		}
	}

	// Delete duplicates
	if !config.NoDelete {
		for _, group := range duplicateGroups {
			if len(group) > 1 {
				for i, path := range group {
					if i > 0 {
						if err := os.Remove(path); err != nil {
							log.Printf("Failed to delete duplicate: %s: %v", path, err)
						} else {
							log.Printf("Deleted duplicate: %s", path)
						}
					}
				}
			}
		}
	}

	// Delete problematic files (incomplete downloads, corrupted, small)
	if len(filesToDelete) > 0 {
		for _, path := range filesToDelete {
			if err := os.Remove(path); err != nil {
				log.Printf("Failed to delete file: %s: %v", path, err)
				cleanupResult.FailedDeletions = append(cleanupResult.FailedDeletions, types.FailedDeletion{
					Path:  path,
					Error: err.Error(),
				})
				// Remove from the deleted lists if deletion failed
				cleanupResult.DeletedIncomplete = removeFromSlice(cleanupResult.DeletedIncomplete, path)
				cleanupResult.DeletedCorrupted = removeFromSlice(cleanupResult.DeletedCorrupted, path)
				cleanupResult.DeletedSmall = removeFromSlice(cleanupResult.DeletedSmall, path)
			} else {
				log.Printf("Deleted problematic file: %s", path)
			}
		}
	}

	// Write todo.md
	if err := todoList.Write(); err != nil {
		return cleanupResult, err
	}
	log.Printf("Wrote todo.md")

	return cleanupResult, nil
}

func removeFromSlice(slice []string, item string) []string {
	result := make([]string, 0, len(slice))
	for _, s := range slice {
		if s != item {
			result = append(result, s)
		}
	}
	return result
}
