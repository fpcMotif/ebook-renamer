package normalizer

import (
	"fmt"
	"path/filepath"
	"regexp"
	"sort"
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

	// Generic pattern: (Series Name) Author - Title
	reGeneric := regexp.MustCompile(`^\s*\(([^)]+)\)\s+(.+)$`)
	if matches := reGeneric.FindStringSubmatch(result); matches != nil {
		restPart := matches[2]
		// Check if 'restPart' starts with an author
		reSep := regexp.MustCompile(`(?:--|[-:])`)
		loc := reSep.FindStringIndex(restPart)
		var potentialAuthor string
		if loc != nil {
			potentialAuthor = restPart[:loc[0]]
		} else {
			potentialAuthor = restPart
		}

		if isLikelyAuthor(potentialAuthor) {
			result = restPart
		}
	}

	return strings.TrimSpace(result)
}

func cleanNoiseSources(s string) string {
	patterns := []string{
		// Z-Library variants
		`\s*[-\(]?\s*[zZ]-?Library(?:\.pdf)?\s*[)\.]?`,
		`\s*\([zZ]-?Library(?:\.pdf)?\)`,
		`\s*-\s*[zZ]-?Library(?:\.pdf)?`,
		// libgen variants
		`\s*[-\(]?\s*libgen(?:\.li)?(?:\.pdf)?\s*[)\.]?`,
		`\s*\(libgen(?:\.li)?(?:\.pdf)?\)`,
		`\s*-\s*libgen(?:\.li)?(?:\.pdf)?`,
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
		// "Uploaded by"
		`\s*[-\(]?\s*[Uu]ploaded by\s+[^)\-]+[)\.]?`,
		`\s*-\s*[Uu]ploaded by\s+[^)\-]+`,
		// "Via ..."
		`\s*[-\(]?\s*[Vv]ia\s+[^)\-]+[)\.]?`,
		// Website URLs
		`\s*[-\(]?\s*w{3}\.[a-zA-Z0-9-]+\.[a-z]{2,}\s*[)\.]?`,
		`\s*[-\(]?\s*[a-zA-Z0-9-]+\.(?:com|org|net|edu|io)\s*[)\.]?`,
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

	// Clean noise sources first
	s = cleanNoiseSources(s)

	// Remove (auth.)
	s = trailingAuthRegex.ReplaceAllString(s, "")

	// Strip trailing ID-like noise
	s = trailingIDRegex.ReplaceAllString(s, "")

	// Remove trailing publisher info separated by dash
	if idx := strings.LastIndex(s, " - "); idx != -1 {
		suffix := s[idx+3:]
		if isPublisherOrSeriesInfo(suffix) {
			s = s[:idx]
		}
	}

	// Handle just "-" without spaces if it looks like publisher
	if idx := strings.LastIndex(s, "-"); idx > 0 && idx < len(s)-1 {
		suffix := strings.TrimSpace(s[idx+1:])
		if isStrictPublisherInfo(suffix) {
			s = s[:idx]
		}
	}

	// Clean orphaned brackets
	s = cleanOrphanedBrackets(s)

	s = spaceRegex.ReplaceAllString(s, " ")
	s = strings.TrimRight(s, "-:;,.")
	s = strings.TrimLeft(s, "-:;,.")

	return strings.TrimSpace(s)
}

func isPublisherOrSeriesInfo(s string) bool {
	publisherKeywords := []string{
		"Press", "Publishing", "Academic Press", "Springer", "Cambridge", "Oxford", "MIT Press",
		"Series", "Textbook Series", "Graduate Texts", "Graduate Studies", "Lecture Notes",
		"Pure and Applied", "Mathematics", "Foundations of", "Monographs", "Studies", "Collection",
		"Textbook", "Edition", "Vol.", "Volume", "No.", "Part", "理工", "出版社", "の",
		"Z-Library", "libgen", "Anna's Archive",
	}

	for _, k := range publisherKeywords {
		if strings.Contains(s, k) {
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

func isStrictPublisherInfo(s string) bool {
	strictKeywords := []string{
		"Press", "Publishing", "Springer", "Cambridge", "Oxford", "MIT", "Wiley", "Elsevier",
		"Routledge", "Pearson", "McGraw", "Addison", "Prentice", "O'Reilly", "Princeton",
		"Harvard", "Yale", "Stanford", "Chicago", "California", "Columbia", "University",
		"Verlag", "Birkhäuser", "CUP",
	}
	for _, k := range strictKeywords {
		if strings.Contains(s, k) {
			return true
		}
	}
	return false
}

func cleanOrphanedBrackets(s string) string {
	var result []rune
	// We need to remove unclosed OPEN brackets.
	// Since we build the string iteratively, we can track the indices of open brackets *in the result*.

	openParensIndices := []int{}
	openBracketsIndices := []int{}

	for _, r := range s {
		switch r {
		case '(':
			openParensIndices = append(openParensIndices, len(result))
			result = append(result, r)
		case ')':
			if len(openParensIndices) > 0 {
				openParensIndices = openParensIndices[:len(openParensIndices)-1]
				result = append(result, r)
			} else {
				result = append(result, ' ')
			}
		case '[':
			openBracketsIndices = append(openBracketsIndices, len(result))
			result = append(result, r)
		case ']':
			if len(openBracketsIndices) > 0 {
				openBracketsIndices = openBracketsIndices[:len(openBracketsIndices)-1]
				result = append(result, r)
			} else {
				result = append(result, ' ')
			}
		case '_':
			result = append(result, ' ')
		default:
			result = append(result, r)
		}
	}

	// Remove unclosed opening brackets
	// Indices are in ascending order. We should remove from the end to keep indices valid.
	indicesToRemove := append(openParensIndices, openBracketsIndices...)
	sort.Sort(sort.Reverse(sort.IntSlice(indicesToRemove)))

	for _, idx := range indicesToRemove {
		if idx < len(result) {
			result = append(result[:idx], result[idx+1:]...)
		}
	}

	return strings.TrimSpace(spaceRegex.ReplaceAllString(string(result), " "))
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
