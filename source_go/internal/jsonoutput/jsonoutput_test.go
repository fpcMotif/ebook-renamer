package jsonoutput

import (
	"encoding/json"
	"testing"

	"github.com/ebook-renamer/go/internal/types"
	"github.com/stretchr/testify/assert"
)

func TestOperationsOutputSerialization(t *testing.T) {
	// Create a complete OperationsOutput structure
	output := types.OperationsOutput{
		Renames: []types.RenameOperation{
			{
				From:   "old_book.pdf",
				To:     "Author - Book Title (2020).pdf",
				Reason: "normalized",
			},
			{
				From:   "another_epub.epub",
				To:     "Another Author - Another Title.epub",
				Reason: "normalized",
			},
		},
		DuplicateDeletes: []types.DuplicateGroup{
			{
				Keep:   "book1.pdf",
				Delete: []string{"book2.pdf", "book3.pdf"},
			},
		},
		SmallOrCorruptedDeletes: []types.DeleteOperation{
			{
				Path:  "tiny.pdf",
				Issue: "deleted",
			},
			{
				Path:  "corrupted.txt",
				Issue: "deleted",
			},
		},
		TodoItems: []types.TodoItem{
			{
				Category: "failed_download",
				File:     "incomplete.download",
				Message:  "Redownload: incomplete.download (Unfinished download)",
			},
			{
				Category: "too_small",
				File:     "tiny.pdf",
				Message:  "Check and redownload: tiny.pdf (File too small, only 1024 bytes)",
			},
		},
	}

	// Test JSON serialization
	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should serialize without error")
	assert.NotEmpty(t, data, "Serialized data should not be empty")

	// Test JSON deserialization
	var decoded types.OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize without error")

	// Verify key fields are preserved
	assert.Equal(t, len(output.Renames), len(decoded.Renames), "Renames count should match")
	assert.Equal(t, output.Renames[0].From, decoded.Renames[0].From, "First rename From should match")
	assert.Equal(t, output.Renames[0].To, decoded.Renames[0].To, "First rename To should match")

	assert.Equal(t, len(output.DuplicateDeletes), len(decoded.DuplicateDeletes), "Duplicate groups count should match")
	assert.Equal(t, output.DuplicateDeletes[0].Keep, decoded.DuplicateDeletes[0].Keep, "First duplicate group Keep should match")
	assert.Equal(t, len(output.DuplicateDeletes[0].Delete), len(decoded.DuplicateDeletes[0].Delete), "First duplicate group Delete count should match")

	assert.Equal(t, len(output.SmallOrCorruptedDeletes), len(decoded.SmallOrCorruptedDeletes), "Small deletes count should match")
	assert.Equal(t, output.SmallOrCorruptedDeletes[0].Path, decoded.SmallOrCorruptedDeletes[0].Path, "First small delete Path should match")

	assert.Equal(t, len(output.TodoItems), len(decoded.TodoItems), "Todo items count should match")
	assert.Equal(t, output.TodoItems[0].Category, decoded.TodoItems[0].Category, "First todo Category should match")
	assert.Equal(t, output.TodoItems[0].Message, decoded.TodoItems[0].Message, "First todo Message should match")
}

func TestOperationsOutputEmptyArrays(t *testing.T) {
	// Test with empty arrays
	output := types.OperationsOutput{
		Renames:                 []types.RenameOperation{},
		DuplicateDeletes:        []types.DuplicateGroup{},
		SmallOrCorruptedDeletes: []types.DeleteOperation{},
		TodoItems:               []types.TodoItem{},
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should serialize empty arrays")

	var decoded types.OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize empty arrays")
	assert.Empty(t, decoded.Renames, "Renames should be empty")
	assert.Empty(t, decoded.DuplicateDeletes, "DuplicateDeletes should be empty")
	assert.Empty(t, decoded.SmallOrCorruptedDeletes, "SmallOrCorruptedDeletes should be empty")
	assert.Empty(t, decoded.TodoItems, "TodoItems should be empty")
}

func TestFromResults(t *testing.T) {
	// Test creating OperationsOutput from results
	newName := "Author - Book.pdf"
	renames := []*types.FileInfo{
		{
			OriginalPath: "/test/old.pdf",
			NewPath:      "/test/Author - Book.pdf",
			NewName:      &newName,
		},
	}

	duplicateGroups := [][]string{
		{"/test/book1.pdf", "/test/book2.pdf"},
	}

	filesToDelete := []string{
		"/test/small.pdf",
	}

	todoItems := []types.TodoItem{
		{
			Category: "too_small",
			File:     "small.pdf",
			Message:  "File too small",
		},
	}

	basePath := "/test"

	output, err := FromResults(renames, duplicateGroups, filesToDelete, todoItems, basePath)
	assert.NoError(t, err, "FromResults should not return error")

	// Verify structure
	assert.NotEmpty(t, output.Renames, "Should have renames")
	assert.Len(t, output.Renames, 1, "Should have exactly one rename")
	assert.Equal(t, "old.pdf", output.Renames[0].From, "First rename From should match")
	assert.Equal(t, "Author - Book.pdf", output.Renames[0].To, "First rename To should match")
	assert.Equal(t, "normalized", output.Renames[0].Reason, "First rename Reason should match")

	assert.NotEmpty(t, output.DuplicateDeletes, "Should have duplicate deletes")
	assert.Len(t, output.DuplicateDeletes, 1, "Should have exactly one duplicate group")

	assert.NotEmpty(t, output.TodoItems, "Should have todo items")
	assert.Equal(t, "too_small", output.TodoItems[0].Category, "First todo Category should match")
	assert.Equal(t, "small.pdf", output.TodoItems[0].File, "First todo File should match")
	assert.Equal(t, "File too small", output.TodoItems[0].Message, "First todo Message should match")
}

func TestOperationsOutputEdgeCases(t *testing.T) {
	// Test edge cases that could cause serialization issues
	output := types.OperationsOutput{
		Renames: []types.RenameOperation{
			{
				From:   "file with\nnewlines.pdf",
				To:     "sanitized\nfile.pdf",
				Reason: "normalized",
			},
			{
				From:   "file with \"quotes\".pdf",
				To:     "file with \"quotes\".pdf",
				Reason: "normalized",
			},
		},
		DuplicateDeletes:        []types.DuplicateGroup{},
		SmallOrCorruptedDeletes: []types.DeleteOperation{},
		TodoItems:               []types.TodoItem{},
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should handle special characters in JSON")

	var decoded types.OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize special characters")
	assert.Len(t, decoded.Renames, 2, "Should preserve renames with special characters")
}

func TestOperationsOutputConsistency(t *testing.T) {
	// Test that output is consistent across multiple runs
	output1 := types.OperationsOutput{
		Renames: []types.RenameOperation{
			{From: "test1.pdf", To: "clean1.pdf", Reason: "normalized"},
		},
		DuplicateDeletes:        []types.DuplicateGroup{},
		SmallOrCorruptedDeletes: []types.DeleteOperation{},
		TodoItems:               []types.TodoItem{},
	}

	output2 := types.OperationsOutput{
		Renames: []types.RenameOperation{
			{From: "test1.pdf", To: "clean1.pdf", Reason: "normalized"},
		},
		DuplicateDeletes:        []types.DuplicateGroup{},
		SmallOrCorruptedDeletes: []types.DeleteOperation{},
		TodoItems:               []types.TodoItem{},
	}

	data1, _ := json.Marshal(output1)
	data2, _ := json.Marshal(output2)

	assert.Equal(t, data1, data2, "Same output should serialize to same JSON")
}

func TestToJSON(t *testing.T) {
	output := &types.OperationsOutput{
		Renames: []types.RenameOperation{
			{From: "old.pdf", To: "new.pdf", Reason: "normalized"},
		},
		DuplicateDeletes: []types.DuplicateGroup{
			{Keep: "keep.pdf", Delete: []string{"delete1.pdf", "delete2.pdf"}},
		},
		SmallOrCorruptedDeletes: []types.DeleteOperation{
			{Path: "small.pdf", Issue: "deleted"},
		},
		TodoItems: []types.TodoItem{
			{Category: "failed_download", File: "incomplete.download", Message: "Redownload required"},
		},
	}

	jsonStr, err := ToJSON(output)
	assert.NoError(t, err, "Should serialize complete output")
	assert.NotEmpty(t, jsonStr, "JSON string should not be empty")
	assert.Contains(t, jsonStr, "old.pdf", "JSON should contain rename from")
	assert.Contains(t, jsonStr, "new.pdf", "JSON should contain rename to")
}

func TestMakeRelativePath(t *testing.T) {
	tests := []struct {
		name      string
		path      string
		targetDir string
		expected  string
	}{
		{"simple relative", "/test/file.pdf", "/test", "file.pdf"},
		{"nested relative", "/test/subdir/file.pdf", "/test", "subdir/file.pdf"},
		{"same directory", "/test", "/test", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := makeRelativePath(tt.path, tt.targetDir)
			assert.Equal(t, tt.expected, result)
		})
	}
}
