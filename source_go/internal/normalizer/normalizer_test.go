package normalizer

import (
	"testing"

	"github.com/ebook-renamer/go/internal/types"
)

func TestParseSimpleFilename(t *testing.T) {
	metadata, err := parseFilename("John Smith - Sample Book Title.pdf", ".pdf")
	if err != nil {
		t.Fatalf("Expected no error, got %v", err)
	}

	if metadata.Authors == nil || *metadata.Authors != "John Smith" {
		t.Errorf("Expected authors to be 'John Smith', got %v", metadata.Authors)
	}

	if metadata.Title != "Sample Book Title" {
		t.Errorf("Expected title to be 'Sample Book Title', got %s", metadata.Title)
	}

	if metadata.Year != nil {
		t.Errorf("Expected year to be nil, got %v", metadata.Year)
	}
}

func TestParseWithYear(t *testing.T) {
	metadata, err := parseFilename("Jane Doe - Another Title (2020, Publisher).pdf", ".pdf")
	if err != nil {
		t.Fatalf("Expected no error, got %v", err)
	}

	if metadata.Authors == nil || *metadata.Authors != "Jane Doe" {
		t.Errorf("Expected authors to be 'Jane Doe', got %v", metadata.Authors)
	}

	if metadata.Title != "Another Title" {
		t.Errorf("Expected title to be 'Another Title', got %s", metadata.Title)
	}

	if metadata.Year == nil || *metadata.Year != 2020 {
		t.Errorf("Expected year to be 2020, got %v", metadata.Year)
	}
}

func TestParseWithSeriesPrefix(t *testing.T) {
	filename := "London Mathematical Society Lecture Note Series B. R. Tennison - Sheaf Theory (1976).pdf"
	metadata, err := parseFilename(filename, ".pdf")
	if err != nil {
		t.Fatalf("Expected no error, got %v", err)
	}

	if metadata.Authors == nil || *metadata.Authors != "B. R. Tennison" {
		t.Errorf("Expected authors to be 'B. R. Tennison', got %v", metadata.Authors)
	}

	if metadata.Title != "Sheaf Theory" {
		t.Errorf("Expected title to be 'Sheaf Theory', got %s", metadata.Title)
	}

	if metadata.Year == nil || *metadata.Year != 1976 {
		t.Errorf("Expected year to be 1976, got %v", metadata.Year)
	}
}

func TestGenerateNewFilenameWithAllFields(t *testing.T) {
	authors := "John Smith"
	metadata := types.ParsedMetadata{
		Authors: &authors,
		Title:   "Great Book",
		Year:    uint16Ptr(2015),
	}

	newName := generateNewFilename(metadata, ".pdf")
	expected := "John Smith - Great Book (2015).pdf"

	if newName != expected {
		t.Errorf("Expected filename to be '%s', got '%s'", expected, newName)
	}
}

func TestGenerateNewFilenameWithoutYear(t *testing.T) {
	authors := "Jane Doe"
	metadata := types.ParsedMetadata{
		Authors: &authors,
		Title:   "Another Book",
		Year:    nil,
	}

	newName := generateNewFilename(metadata, ".pdf")
	expected := "Jane Doe - Another Book.pdf"

	if newName != expected {
		t.Errorf("Expected filename to be '%s', got '%s'", expected, newName)
	}
}

func TestCleanUnderscores(t *testing.T) {
	result := cleanOrphanedBrackets("Sample_Title_With_Underscores")
	expected := "Sample Title With Underscores"

	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}

func TestCleanOrphanedBrackets(t *testing.T) {
	result := cleanOrphanedBrackets("Title ) with ( orphaned ) brackets [")
	// Should remove orphaned closing but keep matched pairs
	openCount := 0
	closeCount := 0
	for _, r := range result {
		if r == '(' {
			openCount++
		} else if r == ')' {
			closeCount++
		}
	}

	// Should not have more closing than opening brackets
	if closeCount > openCount {
		t.Errorf("Expected no more closing than opening brackets, got %d open, %d close", openCount, closeCount)
	}
}

func TestParseAuthorBeforeTitleWithPublisher(t *testing.T) {
	filename := "Ernst Kunz, Richard G. Belshoff - Introduction to Plane Algebraic Curves (2005, Birkhäuser) - libgen.li.pdf"
	metadata, err := parseFilename(filename, ".pdf")
	if err != nil {
		t.Fatalf("Expected no error, got %v", err)
	}

	if metadata.Authors == nil || *metadata.Authors != "Ernst Kunz, Richard G. Belshoff" {
		t.Errorf("Expected authors to be 'Ernst Kunz, Richard G. Belshoff', got %v", metadata.Authors)
	}

	if metadata.Title != "Introduction to Plane Algebraic Curves" {
		t.Errorf("Expected title to be 'Introduction to Plane Algebraic Curves', got %s", metadata.Title)
	}

	if metadata.Year == nil || *metadata.Year != 2005 {
		t.Errorf("Expected year to be 2005, got %v", metadata.Year)
	}
}

func TestParseZLibraryVariant(t *testing.T) {
	filename := "Daniel Huybrechts - Fourier-Mukai transforms in algebraic geometry (z-Library).pdf"
	metadata, err := parseFilename(filename, ".pdf")
	if err != nil {
		t.Fatalf("Expected no error, got %v", err)
	}

	if metadata.Authors == nil || *metadata.Authors != "Daniel Huybrechts" {
		t.Errorf("Expected authors to be 'Daniel Huybrechts', got %v", metadata.Authors)
	}

	if metadata.Title != "Fourier-Mukai transforms in algebraic geometry" {
		t.Errorf("Expected title to be 'Fourier-Mukai transforms in algebraic geometry', got %s", metadata.Title)
	}

	if metadata.Year != nil {
		t.Errorf("Expected year to be nil, got %v", metadata.Year)
	}
}

func TestRemoveYearFromStringWithPublisher(t *testing.T) {
	result := removeYearFromString("Title (2005, Birkhäuser) - libgen.li")
	expected := "Title - libgen.li"

	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}

func TestRemoveYearFromStringStandalone(t *testing.T) {
	result := removeYearFromString("Title 2020, Publisher Name")
	expected := "Title"

	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}

func TestCleanTitleComprehensiveSources(t *testing.T) {
	testCases := []struct {
		input    string
		expected string
	}{
		{"Title - libgen.li", "Title"},
		{"Title - Z-Library", "Title"},
		{"Title - z-Library", "Title"},
		{"Title (libgen.li)", "Title"},
		{"Title libgen.li.pdf", "Title"},
		{"Title Z-Library.pdf", "Title"},
	}

	for _, tc := range testCases {
		result := cleanTitle(tc.input)
		if result != tc.expected {
			t.Errorf("cleanTitle(%s): expected '%s', got '%s'", tc.input, tc.expected, result)
		}
	}
}

// Helper function to create uint16 pointer
func uint16Ptr(n uint16) *uint16 {
	return &n
}
