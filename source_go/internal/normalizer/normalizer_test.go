package normalizer

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestParseSimpleFilename(t *testing.T) {
	metadata, err := parseFilename("John Smith - Sample Book Title.pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "John Smith", *metadata.Authors)
	assert.Equal(t, "Sample Book Title", metadata.Title)
}

func TestParseWithYear(t *testing.T) {
	metadata, err := parseFilename("Jane Doe - Another Title (2020, Publisher).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Jane Doe", *metadata.Authors)
	assert.NotNil(t, metadata.Year)
	assert.Equal(t, uint16(2020), *metadata.Year)
}

func TestParseWithSeriesPrefix(t *testing.T) {
	metadata, err := parseFilename(
		"B. R. Tennison - Sheaf Theory (1976).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "B. R. Tennison", *metadata.Authors)
	assert.Equal(t, "Sheaf Theory", metadata.Title)
	assert.NotNil(t, metadata.Year)
	assert.Equal(t, uint16(1976), *metadata.Year)
}

func TestCleanUnderscores(t *testing.T) {
	result := cleanOrphanedBrackets("Sample_Title_With_Underscores")
	assert.Equal(t, "Sample Title With Underscores", result)
}

func TestCleanOrphanedBrackets(t *testing.T) {
	result := cleanOrphanedBrackets("Title ) with ( orphaned ) brackets [")
	// Orphaned closing should be removed, opening at end removed
	// "Title ) with ( orphaned ) brackets [" -> "Title  with ( orphaned  brackets"
	// Wait, logic in Go:
	// ) -> if openParens > 0 { ... } else skip
	// ( -> openParens++
	// So "Title " (skip )) "with (" (open=1) " orphaned " (skip )) " brackets " (skip [ as it is at end?)
	// The Go implementation:
	// case '[': openBrackets++; result.WriteRune(r)
	// Then at end: for strings.HasSuffix(..., "[") { remove }

	// Let's trace "Title ) with ( orphaned ) brackets ["
	// ) -> skipped
	// ( -> kept, open=1
	// ) -> kept, open=0
	// [ -> kept, open=1
	// Result so far: "Title  with ( orphaned ) brackets ["
	// Then remove trailing [: "Title  with ( orphaned ) brackets "
	// TrimSpace -> "Title  with ( orphaned ) brackets"
	// Wait, my manual trace might be slightly off on spaces, but let's see.
	// The Rust test expects: "Title  with ( orphaned ) brackets" (roughly)
	// Actually Rust test just checks counts.

	// Check that closing parens count <= opening parens count (no orphaned closing parens)
	openCount := strings.Count(result, "(")
	closeCount := strings.Count(result, ")")
	assert.LessOrEqual(t, closeCount, openCount, "Should not have more closing parens than opening parens")
	
	// Check that opening brackets count <= closing brackets count (no orphaned opening brackets)
	openBracketCount := strings.Count(result, "[")
	closeBracketCount := strings.Count(result, "]")
	assert.LessOrEqual(t, openBracketCount, closeBracketCount, "Should not have more opening brackets than closing brackets")
}

func TestParseAuthorBeforeTitleWithPublisher(t *testing.T) {
	metadata, err := parseFilename(
		"Ernst Kunz, Richard G. Belshoff - Introduction to Plane Algebraic Curves (2005, Birkhäuser) - libgen.li.pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Ernst Kunz, Richard G. Belshoff", *metadata.Authors)
	assert.Equal(t, "Introduction to Plane Algebraic Curves", metadata.Title)
	assert.NotNil(t, metadata.Year)
	assert.Equal(t, uint16(2005), *metadata.Year)
}

func TestParseZLibraryVariant(t *testing.T) {
	metadata, err := parseFilename(
		"Daniel Huybrechts - Fourier-Mukai transforms in algebraic geometry (z-Library).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Daniel Huybrechts", *metadata.Authors)
	assert.Equal(t, "Fourier-Mukai transforms in algebraic geometry", metadata.Title)
	assert.Nil(t, metadata.Year)
}

func TestCleanParentheticalsWithPublisher(t *testing.T) {
	year := uint16(2005)
	result := cleanParentheticals("Title (2005, Birkhäuser) - libgen.li", &year)
	assert.Contains(t, result, "Title")
	assert.NotContains(t, result, "2005")
	assert.NotContains(t, result, "Birkhäuser")
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
		{"Title", "Title"},
		{"Title (auth.)", "Title"},
		{"Title with  double  spaces", "Title with double spaces"},
		{"Title -", "Title"},
		{"Title :", "Title"},
		{"Title ;", "Title"},
	}

	for _, tc := range testCases {
		result := cleanTitle(tc.input)
		assert.Equal(t, tc.expected, result, "Input: %s", tc.input)
	}
}

func TestMultiAuthorWithCommas(t *testing.T) {
	metadata, err := parseFilename(
		"Lectures on harmonic analysis (Thomas H. Wolff, Izabella Aba, Carol Shubin).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Thomas H. Wolff, Izabella Aba, Carol Shubin", *metadata.Authors)
	assert.Equal(t, "Lectures on harmonic analysis", metadata.Title)
}

func TestSingleWordCommaRemoval(t *testing.T) {
	metadata, err := parseFilename(
		"Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Marco Grandis", *metadata.Authors)
}

func TestLectureNotesRemoval(t *testing.T) {
	metadata, err := parseFilename(
		"Introduction to Category Theory and Categorical Logic [Lecture notes] (Thomas Streicher).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Thomas Streicher", *metadata.Authors)
	assert.Equal(t, "Introduction to Category Theory and Categorical Logic", metadata.Title)
	assert.NotContains(t, strings.ToLower(metadata.Title), "lecture")
}

func TestTrailingIDNoiseRemoval(t *testing.T) {
	metadata, err := parseFilename(
		"Math History A Long-Form Mathematics Textbook (The Long-Form Math Textbook Series)-B0F5TFL6ZQ.pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.Equal(t, "Math History A Long-Form Mathematics Textbook", metadata.Title)
	assert.NotContains(t, metadata.Title, "B0F5TFL6ZQ")
	assert.NotContains(t, metadata.Title, "Series")
}

func TestCJKAuthorDetection(t *testing.T) {
	metadata, err := parseFilename(
		"文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution (苏阳).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "苏阳", *metadata.Authors)
	assert.Contains(t, metadata.Title, "文革时期中国农村的集体杀戮")
}

func TestNestedPublisherRemoval(t *testing.T) {
	metadata, err := parseFilename(
		"Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Barry Mitchell", *metadata.Authors)
	assert.Equal(t, "Theory of Categories", metadata.Title)
	assert.NotContains(t, metadata.Title, "Pure")
	assert.NotContains(t, metadata.Title, "Academic")
}

func TestDeadlyDecisionBeijing(t *testing.T) {
	metadata, err := parseFilename(
		"Deadly Decision in Beijing. Succession Politics, Protest Repression, and the 1989 Tiananmen Massacre (Yang Su).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Yang Su", *metadata.Authors)
	assert.Contains(t, metadata.Title, "Deadly Decision")
}

func TestToolsForPDE(t *testing.T) {
	metadata, err := parseFilename(
		"Tools for PDE Pseudodifferential Operators, Paradifferential Operators, and Layer Potentials (Michael E. Taylor).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Michael E. Taylor", *metadata.Authors)
	assert.Contains(t, metadata.Title, "Tools for PDE")
}

func TestQuantumCohomology(t *testing.T) {
	metadata, err := parseFilename(
		"From Quantum Cohomology to Integrable Systems (Martin A. Guest).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Martin A. Guest", *metadata.Authors)
	assert.Equal(t, "From Quantum Cohomology to Integrable Systems", metadata.Title)
}

func TestKashiwara(t *testing.T) {
	metadata, err := parseFilename(
		"Bases cristallines des groupes quantiques (Masaki Kashiwara).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Masaki Kashiwara", *metadata.Authors)
	assert.Contains(t, metadata.Title, "Bases cristallines")
}

func TestWaveletsWithMultipleAuthorsAndZLibrary(t *testing.T) {
	metadata, err := parseFilename(
		"Wavelets and their applications (Michel Misiti, Yves Misiti, Georges Oppenheim etc.) (Z-Library).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Michel Misiti, Yves Misiti, Georges Oppenheim etc.", *metadata.Authors)
	assert.Equal(t, "Wavelets and their applications", metadata.Title)
	assert.NotContains(t, metadata.Title, "Z-Library")
}

func TestSystemsOfMicrodifferentialWithHash(t *testing.T) {
	metadata, err := parseFilename(
		"Masaki Kashiwara - Systems of microdifferential equations -- 9780817631383 -- b3ab25f14db594eb0188171e0dd81250 -- Anna's Archive.pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Masaki Kashiwara", *metadata.Authors)
	assert.Equal(t, "Systems of microdifferential equations", metadata.Title)
	assert.NotContains(t, metadata.Title, "9780817631383")
	assert.NotContains(t, metadata.Title, "b3ab25f14db594eb0188171e0dd81250")
	assert.NotContains(t, metadata.Title, "Anna's Archive")
}

func TestManiMehraWavelets(t *testing.T) {
	metadata, err := parseFilename(
		"Wavelets Theory and Its Applications A First Course (Mani Mehra) (Z-Library).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Mani Mehra", *metadata.Authors)
	assert.Equal(t, "Wavelets Theory and Its Applications A First Course", metadata.Title)
	assert.NotContains(t, metadata.Title, "Z-Library")
}

func TestGraduateTextsSeriesRemoval(t *testing.T) {
	metadata, err := parseFilename(
		"Graduate Texts in Mathematics - Saunders Mac Lane - Categories for the Working Mathematician (1978).pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Saunders Mac Lane", *metadata.Authors)
	assert.Equal(t, "Categories for the Working Mathematician", metadata.Title)
	assert.NotNil(t, metadata.Year)
	assert.Equal(t, uint16(1978), *metadata.Year)
	assert.NotContains(t, metadata.Title, "Graduate Texts")
}
