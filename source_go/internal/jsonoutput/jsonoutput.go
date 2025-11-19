package jsonoutput

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"sort"
	"strings"

	"github.com/ebook-renamer/go/internal/types"
)

// FromResults creates an OperationsOutput from processing results
func FromResults(cleanFiles []*types.FileInfo, duplicateGroups [][]string, filesToDelete []string, todoItems []types.TodoItem, targetDir string) (*types.OperationsOutput, error) {
	output := &types.OperationsOutput{
		Renames:                 []types.RenameOperation{},
		DuplicateDeletes:        []types.DuplicateGroup{},
		SmallOrCorruptedDeletes: []types.DeleteOperation{},
		TodoItems:               []types.TodoItem{},
	}

	// Add renames
	renames := []types.RenameOperation{}
	for _, file := range cleanFiles {
		if file.NewName != nil {
			fromPath := makeRelativePath(file.OriginalPath, targetDir)
			toPath := makeRelativePath(file.NewPath, targetDir)

			renames = append(renames, types.RenameOperation{
				From:   fromPath,
				To:     toPath,
				Reason: "normalized",
			})
		}
	}
	// Sort renames by 'from' path for deterministic output
	sort.Slice(renames, func(i, j int) bool {
		return renames[i].From < renames[j].From
	})
	output.Renames = renames

	// Add duplicate deletions
	duplicateDeletes := []types.DuplicateGroup{}
	for _, group := range duplicateGroups {
		if len(group) > 1 {
			keepPath := makeRelativePath(group[0], targetDir)
			var deletePaths []string
			for _, path := range group[1:] {
				deletePaths = append(deletePaths, makeRelativePath(path, targetDir))
			}
			// Sort delete paths for deterministic output
			sort.Strings(deletePaths)

			duplicateDeletes = append(duplicateDeletes, types.DuplicateGroup{
				Keep:   keepPath,
				Delete: deletePaths,
			})
		}
	}
	// Sort duplicate groups by 'keep' path for deterministic output
	sort.Slice(duplicateDeletes, func(i, j int) bool {
		return duplicateDeletes[i].Keep < duplicateDeletes[j].Keep
	})
	output.DuplicateDeletes = duplicateDeletes

	// Add small/corrupted deletions
	smallDeletes := []types.DeleteOperation{}
	for _, path := range filesToDelete {
		smallDeletes = append(smallDeletes, types.DeleteOperation{
			Path:  makeRelativePath(path, targetDir),
			Issue: "deleted",
		})
	}
	// Sort by path for deterministic output
	sort.Slice(smallDeletes, func(i, j int) bool {
		return smallDeletes[i].Path < smallDeletes[j].Path
	})
	output.SmallOrCorruptedDeletes = smallDeletes

	// Add todo items (already sorted by category and file in CLI)
	output.TodoItems = todoItems

	return output, nil
}

// ToJSON converts the OperationsOutput to a JSON string
func ToJSON(output *types.OperationsOutput) (string, error) {
	jsonBytes, err := json.MarshalIndent(output, "", "  ")
	if err != nil {
		return "", fmt.Errorf("JSON serialization failed: %w", err)
	}
	return string(jsonBytes), nil
}

// makeRelativePath converts an absolute path to a relative path using forward slashes
func makeRelativePath(path, targetDir string) string {
	// Convert to relative path
	relPath, err := filepath.Rel(targetDir, path)
	if err != nil {
		// Fallback to absolute path if relative conversion fails
		relPath = path
	}

	// Convert to forward slashes for JSON output (POSIX-style)
	relPath = strings.ReplaceAll(relPath, "\\", "/")

	// Handle case where path is the same as target directory
	if relPath == "." {
		relPath = ""
	}

	return relPath
}
