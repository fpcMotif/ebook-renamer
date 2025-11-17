package normalizer

import (
	"fmt"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/ebook-renamer/go/internal/types"
)

// Series prefixes to remove
var seriesPrefixes = []string{
	"London Mathematical Society Lecture Note Series",
	"Graduate Texts in Mathematics",
	"Progress in Mathematics",
	"[Springer-Lehrbuch]",
	"[Graduate studies in mathematics",
	"[Progress in Mathematics №",
	"[AMS Mathematical Surveys and Monographs",
}

// Source indicators to remove
var sourceIndicators = []string{
	" - libgen.li",
	" - libgen",
	" - Z-Library",
	" - z-Library",
	" - Anna's Archive",
	" (Z-Library)",
	" (z-Library)",
	" (libgen.li)",
	" (libgen)",
	" (Anna's Archive)",
	" libgen.li.pdf",
	" libgen.pdf",
	" Z-Library.pdf",
	" z-Library.pdf",
	" Anna's Archive.pdf",
}

// Non-author keywords to filter out
var nonAuthorKeywords = []string{
	"auth.",
	"translator",
	"translated by",
	"Z-Library",
	"libgen",
	"Anna's Archive",
	"2-Library",
}

// Regex patterns
var (
	yearRegex        = regexp.MustCompile(`\b(19|20)\d{2}\b`)
	yearWithPubRegex = regexp.MustCompile(`\s*\(\s*(19|20)\d{2}\s*(?:,\s*[^)]+)?\s*\)`)
	yearCommaRegex   = regexp.MustCompile(`\s*(19|20)\d{2}\s*,\s*[^,]+$`)
	authRegex        = regexp.MustCompile(`\s*\([Aa]uth\.?\).*`)
	spaceRegex       = regexp.MustCompile(`\s+`)
)

// NormalizeFiles normalizes filenames according to the specification
func NormalizeFiles(files []types.FileInfo) ([]types.FileInfo, error) {
	result := make([]types.FileInfo, len(files))
	
	for i, file := range files {
		// Skip normalization for failed/damaged files
		if file.IsFailedDownload || file.IsTooSmall {
			result[i] = file
			continue
		}

		metadata, err := parseFilename(file.OriginalName, file.Extension)
		if err != nil {
			return nil, fmt.Errorf("failed to parse filename %s: %w", file.OriginalName, err)
		}

		newName := generateNewFilename(metadata, file.Extension)
		
		// Update file info
		updatedFile := file
		updatedFile.NewName = &newName
		updatedFile.NewPath = filepathJoin(filepath.Dir(file.OriginalPath), newName)
		result[i] = updatedFile
	}

	return result, nil
}

// parseFilename parses a filename into metadata components
func parseFilename(filename, extension string) (types.ParsedMetadata, error) {
	// Remove extension and any .download suffix
	base := filename
	if strings.HasSuffix(base, ".download") {
		base = base[:len(base)-len(".download")]
	}
	if strings.HasSuffix(base, extension) {
		base = base[:len(base)-len(extension)]
	}
	base = strings.TrimSpace(base)

	// Clean up obvious noise/series prefixes
	base = stripPrefixNoise(base)

	// Clean source indicators BEFORE parsing authors and titles
	base = cleanSourceIndicators(base)

	// Extract year (4 digits: 19xx or 20xx)
	year := extractYear(base)

	// Remove year and surrounding brackets/parens from base for further processing
	baseWithoutYear := removeYearFromString(base)

	// Try to split authors and title by common separators
	authors, title := splitAuthorsAndTitle(baseWithoutYear)

	return types.ParsedMetadata{
		Authors: authors,
		Title:   title,
		Year:    year,
	}, nil
}

// stripPrefixNoise removes series prefixes
func stripPrefixNoise(s string) string {
	for _, prefix := range seriesPrefixes {
		if strings.HasPrefix(s, prefix) {
			s = s[len(prefix):]
			// Remove leading spaces or dashes
			s = strings.TrimLeft(s, " -")
			break
		}
	}
	return s
}

// cleanSourceIndicators removes source indicators
func cleanSourceIndicators(s string) string {
	for _, pattern := range sourceIndicators {
		if strings.HasSuffix(s, pattern) {
			s = s[:len(s)-len(pattern)]
		}
	}
	return strings.TrimSpace(s)
}

// extractYear extracts the last year found in the string
func extractYear(s string) *uint16 {
	matches := yearRegex.FindAllString(s, -1)
	if len(matches) == 0 {
		return nil
	}

	// Return the last year found (usually most relevant)
	lastMatch := matches[len(matches)-1]
	year, err := strconv.ParseUint(lastMatch, 10, 16)
	if err != nil {
		return nil
	}
	
	y := uint16(year)
	return &y
}

// removeYearFromString removes year patterns from the string
func removeYearFromString(s string) string {
	// Remove year patterns but keep the rest of the string
	// Pattern: (YYYY, Publisher) or (YYYY) or YYYY,
	result := yearWithPubRegex.ReplaceAllString(s, "")
	result = yearCommaRegex.ReplaceAllString(result, "")
	return result
}

// splitAuthorsAndTitle splits authors and title from the cleaned string
func splitAuthorsAndTitle(s string) (*string, string) {
	// Check for trailing (author) pattern
	if lastOpenParen := strings.LastIndex(s, "("); lastOpenParen != -1 && strings.HasSuffix(s, ")") {
		potentialAuthor := s[lastOpenParen+1 : len(s)-1]
		potentialAuthor = strings.TrimSpace(potentialAuthor)
		if isLikelyAuthor(potentialAuthor) {
			title := strings.TrimSpace(s[:lastOpenParen])
			return &potentialAuthor, title
		}
	}

	// Check for " - " separator (most common)
	if lastDash := strings.LastIndex(s, " - "); lastDash != -1 {
		maybeAuthor := strings.TrimSpace(s[:lastDash])
		maybeTitle := strings.TrimSpace(s[lastDash+3:])
		
		if isLikelyAuthor(maybeAuthor) && maybeTitle != "" {
			cleanAuthor := cleanAuthorName(maybeAuthor)
			cleanTitle := cleanTitle(maybeTitle)
			return &cleanAuthor, cleanTitle
		}
	}

	// Check for ":" separator
	if colonIndex := strings.Index(s, ":"); colonIndex != -1 {
		maybeAuthor := strings.TrimSpace(s[:colonIndex])
		maybeTitle := strings.TrimSpace(s[colonIndex+1:])
		
		if isLikelyAuthor(maybeAuthor) && maybeTitle != "" {
			cleanAuthor := cleanAuthorName(maybeAuthor)
			cleanTitle := cleanTitle(maybeTitle)
			return &cleanAuthor, cleanTitle
		}
	}

	// If no clear separator, treat entire string as title
	return nil, cleanTitle(s)
}

// isLikelyAuthor determines if a string is likely an author name
func isLikelyAuthor(s string) bool {
	s = strings.TrimSpace(s)
	
	// Too short to be an author
	if len(s) < 2 {
		return false
	}

	// Filter out obvious non-author phrases
	for _, keyword := range nonAuthorKeywords {
		if strings.Contains(strings.ToLower(s), keyword) {
			return false
		}
	}

	// Check if looks like a name (has at least one uppercase letter)
	for _, r := range s {
		if r >= 'A' && r <= 'Z' {
			return true
		}
	}

	return false
}

// cleanAuthorName cleans up author name
func cleanAuthorName(s string) string {
	s = strings.TrimSpace(s)
	
	// Remove trailing (auth.) etc.
	s = authRegex.ReplaceAllString(s, "")
	
	return strings.TrimSpace(s)
}

// cleanTitle cleans up title
func cleanTitle(s string) string {
	s = strings.TrimSpace(s)

	// Remove trailing source markers
	for _, pattern := range sourceIndicators {
		if strings.HasSuffix(s, pattern) {
			s = s[:len(s)-len(pattern)]
		}
	}

	// Remove trailing .download suffix
	for strings.HasSuffix(s, ".download") {
		s = s[:len(s)-len(".download")]
	}

	// Remove (auth.) and similar patterns
	s = authRegex.ReplaceAllString(s, "")

	// Clean up orphaned brackets/parens
	s = cleanOrphanedBrackets(s)

	// Remove multiple spaces
	s = spaceRegex.ReplaceAllString(s, " ")

	// Remove leading/trailing punctuation
	s = strings.TrimRight(s, "-:;,")
	s = strings.TrimLeft(s, "-:;,")

	return strings.TrimSpace(s)
}

// cleanOrphanedBrackets removes orphaned brackets and replaces underscores
func cleanOrphanedBrackets(s string) string {
	var result strings.Builder
	openParens := 0
	openBrackets := 0
	
	for _, r := range s {
		switch r {
		case '(':
			openParens++
			result.WriteRune(r)
		case ')':
			if openParens > 0 {
				openParens--
				result.WriteRune(r)
			}
		case '[':
			openBrackets++
			result.WriteRune(r)
		case ']':
			if openBrackets > 0 {
				openBrackets--
				result.WriteRune(r)
			}
		case '_':
			result.WriteRune(' ') // Replace underscores with spaces
		default:
			result.WriteRune(r)
		}
	}

	// Remove trailing orphaned opening brackets
	resultStr := result.String()
	for strings.HasSuffix(resultStr, "(") || strings.HasSuffix(resultStr, "[") {
		resultStr = resultStr[:len(resultStr)-1]
	}

	return resultStr
}

// generateNewFilename generates the new filename from metadata
func generateNewFilename(metadata types.ParsedMetadata, extension string) string {
	var result strings.Builder

	if metadata.Authors != nil {
		result.WriteString(*metadata.Authors)
		result.WriteString(" - ")
	}

	result.WriteString(metadata.Title)

	if metadata.Year != nil {
		result.WriteString(fmt.Sprintf(" (%d)", *metadata.Year))
	}

	result.WriteString(extension)
	return result.String()
}

// Helper function for path manipulation
func filepathJoin(dir, file string) string {
	return filepath.Join(dir, file)
}
