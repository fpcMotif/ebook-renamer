package jsonoutput

import (
	"encoding/json"
	"github.com/ebook-renamer/go/internal/types"
	"github.com/stretchr/testify/assert"
	"testing"
	"time"
)

func TestOperationsOutputSerialization(t *testing.T) {
	// Create a complete OperationsOutput structure
	output := OperationsOutput{
		Renames: []RenameOperation{
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
		DuplicateDeletes: []DuplicateDelete{
			{
				Keep:   "book1.pdf",
				Delete: []string{"book2.pdf", "book3.pdf"},
			},
		},
		SmallOrCorruptedDeletes: []FileDelete{
			{
				Path:  "tiny.pdf",
				Issue: "deleted",
			},
			{
				Path:  "corrupted.txt",
				Issue: "deleted",
			},
		},
		TodoItems: []TodoItem{
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
	var decoded OperationsOutput
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
	output := OperationsOutput{
		Renames:                 []RenameOperation{},
		DuplicateDeletes:        []DuplicateDelete{},
		SmallOrCorruptedDeletes: []FileDelete{},
		TodoItems:               []TodoItem{},
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should serialize empty arrays")

	var decoded OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize empty arrays")
	assert.Empty(t, decoded.Renames, "Renames should be empty")
	assert.Empty(t, decoded.DuplicateDeletes, "DuplicateDeletes should be empty")
	assert.Empty(t, decoded.SmallOrCorruptedDeletes, "SmallOrCorruptedDeletes should be empty")
	assert.Empty(t, decoded.TodoItems, "TodoItems should be empty")
}

func TestOperationsOutputWithTime(t *testing.T) {
	now := time.Now()

	output := OperationsOutput{
		Timestamp: now.Format(time.RFC3339),
		Renames: []RenameOperation{
			{
				From:   "test.pdf",
				To:     "clean.pdf",
				Reason: "normalized",
			},
		},
		DuplicateDeletes:        []DuplicateDelete{},
		SmallOrCorruptedDeletes: []FileDelete{},
		TodoItems:               []TodoItem{},
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should serialize with timestamp")

	var decoded OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize with timestamp")
	assert.Equal(t, output.Timestamp, decoded.Timestamp, "Timestamp should be preserved")
}

func TestFromResults(t *testing.T) {
	// Test creating OperationsOutput from results
	renames := []types.FileInfo{
		{
			OriginalPath: "/test/old.pdf",
			NewPath:      "/test/Author - Book.pdf",
			NewName:      "Author - Book.pdf",
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

	output := FromResults(renames, duplicateGroups, filesToDelete, todoItems, basePath)

	// Verify structure
	assert.NotEmpty(t, output.Renames, "Should have renames")
	assert.Len(t, output.Renames, 1, "Should have exactly one rename")
	assert.Equal(t, "old.pdf", output.Renames[0].From, "First rename From should match")
	assert.Equal(t, "Author - Book.pdf", output.Renames[0].To, "First rename To should match")
	assert.Equal(t, "normalized", output.Renames[0].Reason, "First rename Reason should match")

	assert.NotEmpty(t, output.DuplicateDeletes, "Should have duplicate deletes")
	assert.Len(t, output.DuplicateDeletes, 1, "Should have exactly one duplicate group")
	assert.Equal(t, "/test/book1.pdf", output.DuplicateDeletes[0].Keep, "First duplicate Keep should match")
	assert.Equal(t, []string{"/test/book2.pdf"}, output.DuplicateDeletes[0].Delete, "First duplicate Delete should match")

	assert.NotEmpty(t, output.FilesToDelete, "Should have files to delete")
	assert.Equal(t, []string{"/test/small.pdf"}, output.FilesToDelete, "Files to delete should match")

	assert.NotEmpty(t, output.TodoItems, "Should have todo items")
	assert.Equal(t, "too_small", output.TodoItems[0].Category, "First todo Category should match")
	assert.Equal(t, "small.pdf", output.TodoItems[0].File, "First todo File should match")
	assert.Equal(t, "File too small", output.TodoItems[0].Message, "First todo Message should match")

	assert.Equal(t, basePath, output.BasePath, "Base path should match")
}

func TestOperationsOutputEdgeCases(t *testing.T) {
	// Test edge cases that could cause serialization issues
	output := OperationsOutput{
		Renames: []RenameOperation{
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
		DuplicateDeletes:        []DuplicateDelete{},
		SmallOrCorruptedDeletes: []FileDelete{},
		TodoItems:               []TodoItem{},
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should handle special characters in JSON")

	var decoded OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize special characters")
	assert.Len(t, decoded.Renames, 2, "Should preserve renames with special characters")
}

func TestOperationsOutputConsistency(t *testing.T) {
	// Test that output is consistent across multiple runs
	output1 := OperationsOutput{
		Renames: []RenameOperation{
			{From: "test1.pdf", To: "clean1.pdf", Reason: "normalized"},
		},
		DuplicateDeletes:        []DuplicateDelete{},
		SmallOrCorruptedDeletes: []FileDelete{},
		TodoItems:               []TodoItem{},
	}

	output2 := OperationsOutput{
		Renames: []RenameOperation{
			{From: "test1.pdf", To: "clean1.pdf", Reason: "normalized"},
		},
		DuplicateDeletes:        []DuplicateDelete{},
		SmallOrCorruptedDeletes: []FileDelete{},
		TodoItems:               []TodoItem{},
	}

	data1, _ := json.Marshal(output1)
	data2, _ := json.Marshal(output2)

	assert.Equal(t, data1, data2, "Same output should serialize to same JSON")
}

func TestOperationsOutputWithAllFields(t *testing.T) {
	// Test with all possible fields populated
	output := OperationsOutput{
		Timestamp: "2023-01-01T12:00:00Z",
		Renames: []RenameOperation{
			{From: "old.pdf", To: "new.pdf", Reason: "normalized"},
		},
		DuplicateDeletes: []DuplicateDelete{
			{Keep: "keep.pdf", Delete: []string{"delete1.pdf", "delete2.pdf"}},
		},
		SmallOrCorruptedDeletes: []FileDelete{
			{Path: "small.pdf", Issue: "deleted"},
			{Path: "corrupt.txt", Issue: "deleted"},
		},
		TodoItems: []TodoItem{
			{Category: "failed_download", File: "incomplete.download", Message: "Redownload required"},
		},
		BasePath: "/test/base",
	}

	data, err := json.Marshal(output)
	assert.NoError(t, err, "Should serialize complete output")

	var decoded OperationsOutput
	err = json.Unmarshal(data, &decoded)
	assert.NoError(t, err, "Should deserialize complete output")

	// Verify all fields are preserved
	assert.Equal(t, output.Timestamp, decoded.Timestamp)
	assert.Equal(t, output.BasePath, decoded.BasePath)
	assert.Equal(t, len(output.Renames), len(decoded.Renames))
	assert.Equal(t, len(output.DuplicateDeletes), len(decoded.DuplicateDeletes))
	assert.Equal(t, len(output.SmallOrCorruptedDeletes), len(decoded.SmallOrCorruptedDeletes))
	assert.Equal(t, len(output.TodoItems), len(decoded.TodoItems))
}
