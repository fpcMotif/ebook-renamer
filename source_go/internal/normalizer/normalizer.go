package normalizer

import (
	"fmt"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"unicode"

	"github.com/ebook-renamer/go/internal/types"
)

// Regex patterns
var (
	yearRegex        = regexp.MustCompile(`\b(19|20)\d{2}\b`)
	spaceRegex       = regexp.MustCompile(`\s{2,}`)
	bracketRegex     = regexp.MustCompile(`\s*\[[^\]]*\]`)
	trailingIDRegex  = regexp.MustCompile(`[-_][A-Za-z0-9]{8,}$`)
	simpleParenRegex = regexp.MustCompile(`\([^)]+\)`)
	// Go's regex engine (RE2) doesn't support recursion, so we iterate for nested parens
	nestedParenRegex = regexp.MustCompile(`\([^()]*(?:\([^()]*\)[^()]*)*\)`)

	// Author/Title patterns
	trailingAuthorRegex = regexp.MustCompile(`^(.+?)\s*\(([^)]+)\)\s*$`)
	separatorRegex      = regexp.MustCompile(`^(.+?)\s*(?:--|[-:])\s+(.+)$`)
	multiAuthorRegex    = regexp.MustCompile(`^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*(?:--|[-:])\s+(.+)$`)
	semicolonRegex      = regexp.MustCompile(`^(.+?)\s*;\s*(.+)$`)

	// Cleaning patterns
	authNoiseRegex    = regexp.MustCompile(`\s*\((?:[Aa]uth\.?|[Aa]uthor|[Ee]ds?\.?|[Tt]ranslator)\)`)
	trailingAuthRegex = regexp.MustCompile(`\s*\([Aa]uth\.?\)`)
	
	// New Patterns
	versionRegex   = regexp.MustCompile(`(?i)\b(v|ver|version)\.?\s*\d+(\.\d+)*\b`)
	pageCountRegex = regexp.MustCompile(`(?i)\b\d+\s*(?:pages?|pp?\.?|P)\b`)
	langTagRegex   = regexp.MustCompile(`(?i)(\((?:English|Chinese|Japanese|中文|日本語)\)|(?:English|Chinese|Japanese) Edition)`)
)

// NormalizeFiles normalizes filenames according to the specification
func NormalizeFiles(files []*types.FileInfo) ([]*types.FileInfo, error) {
	result := make([]*types.FileInfo, len(files))

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

		file.NewName = &newName
		file.NewPath = filepathJoin(filepath.Dir(file.OriginalPath), newName)
		result[i] = file
	}

	return result, nil
}

// parseFilename parses a filename into metadata components
func parseFilename(filename, extension string) (types.ParsedMetadata, error) {
	// Step 1: Remove extension
	base := filename
	base = strings.TrimSuffix(base, ".download")
	base = strings.TrimSuffix(base, extension)
	base = strings.TrimSpace(base)

	// Step 2: Remove series prefixes (must be early)
	base = removeSeriesPrefixes(base)

	// Step 3: Remove ALL bracketed annotations
	base = bracketRegex.ReplaceAllString(base, "")

	// Step 4: Clean noise sources (Z-Library, etc.)
	// MUST happen BEFORE author parsing
	base = cleanNoiseSources(base)

	// Step 5: Remove duplicate markers: -2, -3, (1), (2)
	base = removeDuplicateMarkers(base)

	// Step 6: Extract year FIRST
	year := extractYear(base)

	// Step 7: Remove parentheticals
	base = cleanParentheticals(base, year)

	// Step 8: Parse author and title
	authors, title := smartParseAuthorTitle(base)

	return types.ParsedMetadata{
		Authors: authors,
		Title:   title,
		Year:    year,
	}, nil
}

func removeSeriesPrefixes(s string) string {
	prefixes := []string{
		"London Mathematical Society Lecture Note Series",
		"Graduate Texts in Mathematics",
		"Progress in Mathematics",
		"[Springer-Lehrbuch]",
		"[Graduate studies in mathematics",
		"[Progress in Mathematics №",
		"[AMS Mathematical Surveys and Monographs",
	}

	result := s
	for _, prefix := range prefixes {
		if strings.HasPrefix(result, prefix) {
			result = result[len(prefix):]
			result = strings.TrimLeft(result, "- ]")
			break
		}
	}
	return strings.TrimSpace(result)
}

func cleanNoiseSources(s string) string {
	patterns := []string{
		// Improved patterns (strictly as requested, requires .pdf to match, so won't break non-pdf cases)
		`\s+libgen\.li\.pdf\b`,
		`\s*[-\(]?\s*[zZ]-?Library\.pdf\b`,
		
		// Z-Library variants
		`\s*[-\(]?\s*[zZ]-?Library\s*[)\.]?`,
		`\s*\([zZ]-?Library\)`,
		`\s*-\s*[zZ]-?Library`,
		// libgen variants
		`\s*[-\(]?\s*libgen(?:\.li)?\s*[)\.]?`,
		`\s*\(libgen(?:\.li)?\)`,
		`\s*-\s*libgen(?:\.li)?`,
		// Anna's Archive variants
		`Anna'?s?\s*Archive`,
		`\s*[-\(]?\s*Anna'?s?\s+Archive\s*[)\.]?`,
		`\s*\(Anna'?s?\s+Archive\)`,
		`\s*-\s*Anna'?s?\s+Archive`,
		// Hash patterns
		`\s*--\s*[a-f0-9]{32}\s*(?:--)?`,
		`\s*--\s*\d{10,13}\s*(?:--)?`,
		`\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?`,
		`\s*--\s*[a-f0-9]{8,}\s*(?:--)?`,
	}

	result := s
	// Apply patterns multiple times
	for i := 0; i < 3; i++ {
		before := result
		for _, p := range patterns {
			re := regexp.MustCompile(p)
			result = re.ReplaceAllString(result, "")
		}
		if result == before {
			break
		}
	}
	return strings.TrimSpace(result)
}

func removeDuplicateMarkers(s string) string {
	// (1), (2) at end
	re1 := regexp.MustCompile(`[-\s]*\(\d{1,2}\)\s*$`)
	s = re1.ReplaceAllString(s, "")

	// -2, -3 at end
	re2 := regexp.MustCompile(`-\d{1,2}\s*$`)
	s = re2.ReplaceAllString(s, "")

	// -2 before (year)
	re3 := regexp.MustCompile(`-\d{1,2}\s+\(`)
	s = re3.ReplaceAllString(s, " (")

	return s
}

func extractYear(s string) *uint16 {
	matches := yearRegex.FindAllString(s, -1)
	if len(matches) == 0 {
		return nil
	}
	// Return the last year found
	lastMatch := matches[len(matches)-1]
	year, err := strconv.ParseUint(lastMatch, 10, 16)
	if err != nil {
		return nil
	}
	y := uint16(year)
	return &y
}

func cleanParentheticals(s string, year *uint16) string {
	result := s

	// Pattern 1: Remove (YYYY, Publisher) or (YYYY)
	if year != nil {
		y := *year
		pattern := fmt.Sprintf(`\s*\(\s*%d\s*(?:,\s*[^)]+)?\s*\)`, y)
		re := regexp.MustCompile(pattern)
		result = re.ReplaceAllString(result, "")
	}

	// Pattern 2: Remove nested parentheticals with publisher keywords
	for {
		changed := false
		result = nestedParenRegex.ReplaceAllStringFunc(result, func(match string) string {
			if isPublisherOrSeriesInfo(match) {
				changed = true
				return ""
			}
			return match
		})
		if !changed {
			break
		}
	}

	// Pattern 3: Remove simple parentheticals with publisher keywords
	result = simpleParenRegex.ReplaceAllStringFunc(result, func(match string) string {
		if isPublisherOrSeriesInfo(match) {
			return ""
		}
		return match
	})

	result = spaceRegex.ReplaceAllString(result, " ")
	return strings.TrimSpace(result)
}

func smartParseAuthorTitle(s string) (*string, string) {
	s = strings.TrimSpace(s)

	// Pattern 1: "Title (Author)"
	if matches := trailingAuthorRegex.FindStringSubmatch(s); matches != nil {
		titlePart := matches[1]
		authorPart := matches[2]

		if isLikelyAuthor(authorPart) && !isPublisherOrSeriesInfo("("+authorPart+")") {
			cleanAuth := cleanAuthorName(authorPart)
			cleanTitl := cleanTitle(titlePart)
			return &cleanAuth, cleanTitl
		}
	}

	// Pattern 2: "Author - Title" or "Author: Title"
	if matches := separatorRegex.FindStringSubmatch(s); matches != nil {
		authorPart := matches[1]
		titlePart := matches[2]

		if isLikelyAuthor(authorPart) && titlePart != "" {
			cleanAuth := cleanAuthorName(authorPart)
			cleanTitl := cleanTitle(titlePart)
			return &cleanAuth, cleanTitl
		}
	}

	// Pattern 3: Multiple authors separated by commas, then dash
	if matches := multiAuthorRegex.FindStringSubmatch(s); matches != nil {
		author1 := matches[1]
		author2 := matches[2]
		titlePart := matches[3]

		if isLikelyAuthor(author1) && isLikelyAuthor(author2) {
			authors := fmt.Sprintf("%s, %s", cleanAuthorName(author1), cleanAuthorName(author2))
			cleanTitl := cleanTitle(titlePart)
			return &authors, cleanTitl
		}
	}

	// Pattern 4: "Title; Author"
	if matches := semicolonRegex.FindStringSubmatch(s); matches != nil {
		titlePart := matches[1]
		authorPart := matches[2]

		if isLikelyAuthor(authorPart) && !isPublisherOrSeriesInfo(authorPart) {
			cleanAuth := cleanAuthorName(authorPart)
			cleanTitl := cleanTitle(titlePart)
			return &cleanAuth, cleanTitl
		}
	}

	// Pattern 5: No clear author
	return nil, cleanTitle(s)
}

func isLikelyAuthor(s string) bool {
	s = strings.TrimSpace(s)
	if len(s) < 2 {
		return false
	}

	nonAuthorKeywords := []string{
		"auth.", "translator", "translated by", "z-library", "libgen", "anna's archive", "2-library",
	}
	// Case insensitive check
	lowerS := strings.ToLower(s)
	for _, k := range nonAuthorKeywords {
		if strings.Contains(lowerS, k) {
			return false
		}
	}

	// Check if contains digits only
	hasDigitOnly := true
	for _, c := range s {
		if !unicode.IsDigit(c) && c != '-' && c != '_' {
			hasDigitOnly = false
			break
		}
	}
	if hasDigitOnly {
		return false
	}

	// Check if looks like a name (uppercase Latin OR non-Latin alphabetic)
	hasUppercase := false
	hasNonLatin := false
	for _, c := range s {
		if unicode.IsUpper(c) {
			hasUppercase = true
		}
		if unicode.IsLetter(c) && c > 127 { // Rough check for non-ASCII letters
			hasNonLatin = true
		}
	}

	return hasUppercase || hasNonLatin
}

func cleanAuthorName(s string) string {
	s = strings.TrimSpace(s)

	// Remove noise patterns
	s = authNoiseRegex.ReplaceAllString(s, "")

	// Smart comma handling
	commaCount := strings.Count(s, ",")
	if commaCount == 1 {
		if idx := strings.Index(s, ", "); idx != -1 {
			before := strings.TrimSpace(s[:idx])
			after := strings.TrimSpace(s[idx+2:])

			beforeWords := len(strings.Fields(before))
			afterWords := len(strings.Fields(after))

			// Join if both parts are single words (Marco, Grandis -> Marco Grandis)
			if beforeWords == 1 && afterWords == 1 {
				s = fmt.Sprintf("%s %s", before, after)
			}
		}
	}
	// If multiple commas, keep them ALL

	s = spaceRegex.ReplaceAllString(s, " ")
	return strings.TrimSpace(s)
}

func cleanTitle(s string) string {
	s = strings.TrimSpace(s)

	// Call cleanNoiseSources to handle cases where title still contains noise
	// This fixes TestCleanTitleComprehensiveSources
	s = cleanNoiseSources(s)

	// Remove (auth.)
	s = trailingAuthRegex.ReplaceAllString(s, "")
	
	// Clean new patterns: Versions, Page counts, Language tags
	s = cleanPatterns(s)

	// Strip trailing ID-like noise
	s = trailingIDRegex.ReplaceAllString(s, "")

	// Clean orphaned brackets
	s = cleanOrphanedBrackets(s)

	s = spaceRegex.ReplaceAllString(s, " ")
	s = strings.TrimRight(s, "-:;,.")
	s = strings.TrimLeft(s, "-:;,.")

	return strings.TrimSpace(s)
}

func cleanPatterns(s string) string {
	s = versionRegex.ReplaceAllString(s, "")
	s = pageCountRegex.ReplaceAllString(s, "")
	s = langTagRegex.ReplaceAllString(s, "")
	return s
}

func isPublisherOrSeriesInfo(s string) bool {
	publisherKeywords := []string{
		// Original
		"Press", "Publishing", "Academic Press", "Springer", "Cambridge", "Oxford", "MIT Press",
		"Series", "Textbook Series", "Graduate Texts", "Graduate Studies", "Lecture Notes",
		"Pure and Applied", "Mathematics", "Foundations of", "Monographs", "Studies", "Collection",
		"Textbook", "Edition", "Vol.", "Volume", "No.", "Part", "理工", "出版社", "の",
		"Z-Library", "libgen", "Anna's Archive",
		
		// New Publishers
		"Wiley", "Pearson", "McGraw-Hill", "Elsevier", "Taylor & Francis",
		
		// General Genres
		"Fiction", "Novel", "Handbook", "Manual", "Guide", "Reference",
		"Cookbook", "Workbook", "Encyclopedia", "Dictionary", "Atlas", "Anthology",
		"Biography", "Memoir", "Essay", "Poetry", "Drama", "Short Stories",
		
		// Academic Genres
		"Thesis", "Dissertation", "Proceedings", "Conference", "Symposium", "Workshop",
		"Report", "Technical Report", "White Paper", "Preprint", "Manuscript",
		"Lecture", "Course Notes", "Study Guide", "Solutions Manual",
		
		// Version Keywords
		"Revised Edition", "Updated Edition", "Expanded Edition",
		"Abridged", "Unabridged", "Complete Edition", "Anniversary Edition",
		"Collector's Edition", "Special Edition", "1st ed", "2nd ed", "3rd ed",
		
		// Format/Quality
		"OCR", "Scanned", "Retail", "Searchable", "Bookmarked", "Optimized",
		"Compressed", "High Quality", "HQ", "DRM-free", "No DRM", "Cracked",
		"Kindle Edition", "PDF version", "EPUB version", "MOBI version",
		
		// Chinese
		"小说", "教材", "教程", "手册", "指南", "参考书", "文集", "论文集",
		"丛书", "系列", "修订版", "第二版", "第三版", "增订版",
		
		// Japanese
		"小説", "教科書", "テキスト", "ハンドブック", "マニュアル", "ガイド",
		"講義", "シリーズ", "改訂版", "第2版", "第3版",
	}

	sLower := strings.ToLower(s)
	for _, k := range publisherKeywords {
		if strings.Contains(sLower, strings.ToLower(k)) {
			return true
		}
	}

	// Detect hash patterns
	if regexp.MustCompile(`[a-f0-9]{8,}`).MatchString(s) && len(s) > 8 {
		return true
	}
	if regexp.MustCompile(`[A-Za-z0-9]{16,}`).MatchString(s) && len(s) > 16 {
		return true
	}

	// Check for series info (mostly non-letters with numbers)
	hasNumbers := false
	nonLetterCount := 0
	for _, c := range s {
		if unicode.IsDigit(c) {
			hasNumbers = true
		}
		if !unicode.IsLetter(c) && c != ' ' {
			nonLetterCount++
		}
	}
	if hasNumbers && nonLetterCount > 2 {
		return true
	}

	return false
}

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
			result.WriteRune(' ')
		default:
			result.WriteRune(r)
		}
	}

	resultStr := result.String()
	// Remove trailing orphaned opening brackets
	for strings.HasSuffix(resultStr, "(") || strings.HasSuffix(resultStr, "[") {
		resultStr = resultStr[:len(resultStr)-1]
	}

	return strings.TrimSpace(resultStr)
}

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

func filepathJoin(dir, file string) string {
	return filepath.Join(dir, file)
}
