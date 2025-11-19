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
	"github.com/ebook-renamer/go/internal/types"
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
		Json:            jsonFlag,
	}

	log.Printf("Starting ebook renamer with config: %+v", config)

	// Process files
	return processFiles(config)
}

func nilString(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func processFiles(config *types.Config) error {
	// Create scanner
	s, err := scanner.New(config.Path, int(config.MaxDepth))
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

	// Handle failed downloads and small files
	var filesToDelete []string
	var todoItems []types.TodoItem

	for _, fileInfo := range normalized {
		if fileInfo.IsFailedDownload || fileInfo.IsTooSmall {
			if config.DeleteSmall {
				filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
				todoList.RemoveFileFromTodo(fileInfo.OriginalName)
			} else {
				if fileInfo.IsFailedDownload {
					todoList.AddFailedDownload(fileInfo)
					todoItems = append(todoItems, types.TodoItem{
						Category: "failed_download",
						File:     fileInfo.OriginalName,
						Message:  fmt.Sprintf("重新下载: %s (未完成下载)", fileInfo.OriginalName),
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
		} else {
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
		if err := executeOperations(cleanFiles, duplicateGroups, filesToDelete, todoList, config); err != nil {
			return fmt.Errorf("execution failed: %w", err)
		}
	}

	if !config.Json {
		fmt.Println("\n✓ Operation completed successfully!")
	}

	return nil
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

func executeOperations(cleanFiles []*types.FileInfo, duplicateGroups [][]string, filesToDelete []string, todoList *todo.TodoList, config *types.Config) error {
	// Execute renames
	for _, fileInfo := range cleanFiles {
		if fileInfo.NewName != nil {
			if err := os.Rename(fileInfo.OriginalPath, fileInfo.NewPath); err != nil {
				return fmt.Errorf("rename failed: %w", err)
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
							return fmt.Errorf("delete duplicate failed: %w", err)
						}
						log.Printf("Deleted duplicate: %s", path)
					}
				}
			}
		}
	}

	// Delete small/corrupted files
	if config.DeleteSmall && len(filesToDelete) > 0 {
		fmt.Printf("\nDeleting %d small/corrupted files...\n", len(filesToDelete))
		for _, path := range filesToDelete {
			if err := os.Remove(path); err != nil {
				return fmt.Errorf("delete small file failed: %w", err)
			}
			log.Printf("Deleted small/corrupted file: %s", path)
			fmt.Printf("  Deleted: %s\n", path)
		}
	}

	// Write todo.md
	if err := todoList.Write(); err != nil {
		return err
	}
	log.Printf("Wrote todo.md")

	return nil
}
