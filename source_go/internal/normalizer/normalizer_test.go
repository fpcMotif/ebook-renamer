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
	// "Title ) with ( orphaned ) brackets ["
	// 1. ')' at index 6: openParens=0. Skipped.
	// 2. '(' at index 13: openParens=1. Kept.
	// 3. ')' at index 24: openParens=1. openParens becomes 0. Kept.
	// 4. '[' at end: openBrackets=1. Kept.
	// Result string before suffix trim: "Title  with ( orphaned ) brackets ["
	// Suffix loop removes '['.
	// TrimSpace.
	// Expected: "Title  with ( orphaned ) brackets"
	assert.Equal(t, "Title  with ( orphaned ) brackets", result)
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

// ========== EDGE CASE TESTS ==========

func TestEmptyFilename(t *testing.T) {
	metadata, err := parseFilename(".pdf", ".pdf")
	assert.NoError(t, err)
	assert.True(t, metadata.Title == "" || len(metadata.Title) == 0)
}

func TestTitleOnlyNoAuthor(t *testing.T) {
	metadata, err := parseFilename("Introduction to Mathematics.pdf", ".pdf")
	assert.NoError(t, err)
	assert.Nil(t, metadata.Authors)
	assert.Equal(t, "Introduction to Mathematics", metadata.Title)
}

func TestMultipleYearsRightmost(t *testing.T) {
	// Should extract the rightmost year (2020), not the first (2018)
	metadata, err := parseFilename("Title (2018 Edition) (2020, Publisher).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Year)
	assert.Equal(t, uint16(2020), *metadata.Year)
}

func TestVeryLongFilename(t *testing.T) {
	longTitle := strings.Repeat("A", 200)
	filename := longTitle + ".pdf"
	metadata, err := parseFilename(filename, ".pdf")
	assert.NoError(t, err)
	assert.NotEmpty(t, metadata.Title)
}

func TestSpecialCharactersInTitle(t *testing.T) {
	metadata, err := parseFilename("Author - Title with & symbols, (special) chars!.pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Author", *metadata.Authors)
}

func TestCyrillicAuthor(t *testing.T) {
	metadata, err := parseFilename("Теория категорий (Сергей Иванов).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Сергей Иванов", *metadata.Authors)
}

func TestDoubleDashSeparator(t *testing.T) {
	metadata, err := parseFilename("Author Name -- Book Title.pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Author Name", *metadata.Authors)
	assert.Equal(t, "Book Title", metadata.Title)
}

func TestMultipleDashesInTitle(t *testing.T) {
	metadata, err := parseFilename("Self-Taught Programmer - A Step-by-Step Guide.pdf", ".pdf")
	assert.NoError(t, err)
	assert.Contains(t, metadata.Title, "Step")
}

func TestEpubExtension(t *testing.T) {
	metadata, err := parseFilename("Author - Title.epub", ".epub")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Author", *metadata.Authors)
	assert.Equal(t, "Title", metadata.Title)
}

func TestTxtExtension(t *testing.T) {
	metadata, err := parseFilename("Author - Title.txt", ".txt")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Author", *metadata.Authors)
	assert.Equal(t, "Title", metadata.Title)
}

func TestNoExtension(t *testing.T) {
	metadata, err := parseFilename("Author - Title", "")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "Author", *metadata.Authors)
	assert.Equal(t, "Title", metadata.Title)
}

func TestJapaneseTitle(t *testing.T) {
	metadata, err := parseFilename("数学入門 (山田太郎).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "山田太郎", *metadata.Authors)
	assert.Contains(t, metadata.Title, "数学入門")
}

func TestKoreanTitle(t *testing.T) {
	metadata, err := parseFilename("수학 입문 (김철수).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotNil(t, metadata.Authors)
	assert.Equal(t, "김철수", *metadata.Authors)
}

func TestMixedLanguageTitle(t *testing.T) {
	metadata, err := parseFilename("Introduction to 数学 Mathematics.pdf", ".pdf")
	assert.NoError(t, err)
	assert.Contains(t, metadata.Title, "数学")
	assert.Contains(t, metadata.Title, "Mathematics")
}

func TestCleanTitleTrailingDash(t *testing.T) {
	result := cleanTitle("Title -")
	assert.Equal(t, "Title", result)
}

func TestCleanTitleTrailingColon(t *testing.T) {
	result := cleanTitle("Title :")
	assert.Equal(t, "Title", result)
}

func TestCleanTitleTrailingSemicolon(t *testing.T) {
	result := cleanTitle("Title ;")
	assert.Equal(t, "Title", result)
}

func TestWhitespaceOnlyTitle(t *testing.T) {
	result := cleanTitle("   ")
	assert.Empty(t, result)
}

func TestNestedBracketsAndParens(t *testing.T) {
	metadata, err := parseFilename("Title ((nested (very nested)) text).pdf", ".pdf")
	assert.NoError(t, err)
	assert.NotEmpty(t, metadata.Title)
}

func TestHashPatternDetection(t *testing.T) {
	metadata, err := parseFilename(
		"Masaki Kashiwara - Systems of microdifferential equations -- 9780817631383 -- b3ab25f14db594eb0188171e0dd81250 -- Anna's Archive.pdf",
		".pdf",
	)
	assert.NoError(t, err)
	assert.NotContains(t, metadata.Title, "b3ab25f14db594eb0188171e0dd81250")
}
