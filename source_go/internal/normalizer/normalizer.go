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

		// Update file info
		// We need to create a copy or modify the pointer if it's mutable.
		// Since we are returning a new slice of pointers, we can just modify the struct if we own it,
		// or create a new one. FileInfo is a struct pointer in the input slice?
		// The input is []*types.FileInfo.
		// Let's modify it in place or create a copy.
		// Rust implementation modifies in place.

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

	// Step 5: Clean extended noise (editions, versions, etc.)
	base = cleanExtendedNoise(base)

	// Step 6: Remove duplicate markers: -2, -3, (1), (2)
	base = removeDuplicateMarkers(base)

	// Step 7: Extract year FIRST
	year := extractYear(base)

	// Step 8: Remove parentheticals
	base = cleanParentheticals(base, year)

	// Step 9: First bracket validation
	base = validateAndFixBrackets(base)

	// Step 10: Parse author and title
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
		// Special cases: source markers with .pdf suffix
		`(?i)\s+libgen\.li\.pdf\b`,
		`(?i)\s+libgen\.pdf\b`,
		`(?i)\s+[zZ]-?Library\.pdf\b`,
		`(?i)\s+Anna'?s?\s+Archive\.pdf\b`,
		// Z-Library variants (case insensitive)
		`(?i)\s*[-\(]?\s*[zZ]-?Library\s*[)\.]?`,
		`(?i)\s*\([zZ]-?Library\)`,
		`(?i)\s*-\s*[zZ]-?Library`,
		// libgen variants
		`(?i)\s*[-\(]?\s*libgen(?:\.li)?\s*[)\.]?`,
		`(?i)\s*\(libgen(?:\.li)?\)`,
		`(?i)\s*-\s*libgen(?:\.li)?`,
		// Anna's Archive variants
		`(?i)Anna'?s?\s*Archive`,
		`(?i)\s*[-\(]?\s*Anna'?s?\s+Archive\s*[)\.]?`,
		`(?i)\s*\(Anna'?s?\s+Archive\)`,
		`(?i)\s*-\s*Anna'?s?\s+Archive`,
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

func cleanExtendedNoise(s string) string {
	// Remove edition, version, language, and quality markers
	patterns := []string{
		// Edition patterns
		`(?i)\s*-?\s*\d+(?:st|nd|rd|th)\s+[Ee]dition\b`,
		`(?i)\s*-?\s*\([Rr]evised\s+[Ee]dition\)`,
		`(?i)\s*-?\s*\([Ee]dition\s+\d+\)`,
		// Reprint patterns
		`(?i)\s*-?\s*\([Rr]eprint\s+\d{4}\)`,
		// Version patterns
		`(?i)\s*-?\s*[vV](?:er(?:sion)?)?\s*\d+(?:\.\d+)*`,
		// Language annotations
		`(?i)\s*\([Ee]nglish(?:\s+[Vv]ersion)?\)`,
		`(?i)\s*\([Cc]hinese(?:\s+[Vv]ersion)?\)`,
		`\s*\(中文版\)`,
		`\s*\(英文版\)`,
		// Quality markers
		`(?i)\s*-?\s*\b(?:OCR|[Ss]canned|[Ww]atermarked|[Bb]ookmarked)\b`,
		// ArXiv IDs
		`(?i)\s*-?\s*arXiv:\d{4}\.\d{4,5}(?:v\d+)?`,
		// DOI
		`(?i)\s*-?\s*doi:\s*[\w\./]+`,
		// ISBN in text
		`(?i)\s*-?\s*ISBN[-:\s]*\d[\d\-]{8,}`,
		// Duplicate markers
		`(?i)\s*[-_]\s*[Cc]opy\s+\d+`,
		`\s*\(\d{1,2}\)\s*$`,
	}

	result := s
	for _, p := range patterns {
		re := regexp.MustCompile(p)
		result = re.ReplaceAllString(result, "")
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

	// Remove noise sources (Z-Library, etc.)
	s = cleanNoiseSources(s)

	// Remove (auth.)
	s = trailingAuthRegex.ReplaceAllString(s, "")

	// Strip trailing ID-like noise
	s = trailingIDRegex.ReplaceAllString(s, "")

	// Clean orphaned brackets
	s = cleanOrphanedBrackets(s)

	s = spaceRegex.ReplaceAllString(s, " ")
	s = strings.TrimRight(s, "-:;,.")
	s = strings.TrimLeft(s, "-:;,.")

	return strings.TrimSpace(s)
}

func isPublisherOrSeriesInfo(s string) bool {
	// Case-insensitive matching
	sLower := strings.ToLower(s)
	
	publisherKeywords := []string{
		"press", "publishing", "publisher",
		"springer", "cambridge", "oxford", "mit press", "elsevier",
		"wiley", "pearson", "academic press",
		"series", "textbook series", "lecture notes",
		"graduate texts", "graduate studies",
		"pure and applied", "foundations of",
		"monographs", "studies", "collection",
		"textbook", "edition", "revised", "reprint",
		"vol.", "volume", "no.", "part",
		// Chinese keywords
		"出版社", "出版", "教材", "系列",
		"丛书", "讲义", "版", "修订版",
		// Japanese
		"の",
		"z-library", "libgen", "anna's archive",
	}

	for _, k := range publisherKeywords {
		if strings.Contains(sLower, k) {
			return true
		}
	}
	
	// Check for edition patterns
	editionRe := regexp.MustCompile(`(?i)\d+(?:st|nd|rd|th)\s+ed(?:ition)?`)
	if editionRe.MatchString(sLower) {
		return true
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
			// Skip orphaned closing paren
		case '[':
			openBrackets++
			result.WriteRune(r)
		case ']':
			if openBrackets > 0 {
				openBrackets--
				result.WriteRune(r)
			}
			// Skip orphaned closing bracket
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

func validateAndFixBrackets(s string) string {
	// Count unmatched opening brackets
	openCount := strings.Count(s, "(") - strings.Count(s, ")")
	
	if openCount > 0 {
		result := s
		remaining := openCount
		
		// Remove orphaned opening brackets from the end
		for remaining > 0 {
			lastOpen := strings.LastIndex(result, "(")
			if lastOpen == -1 {
				break
			}
			
			// Check if this '(' has a matching ')' after it
			after := result[lastOpen+1:]
			if !strings.Contains(after, ")") {
				// This is an orphaned opening bracket, remove it
				result = result[:lastOpen] + result[lastOpen+1:]
				remaining--
			} else {
				break
			}
		}
		return strings.TrimSpace(result)
	}
	
	return s
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
