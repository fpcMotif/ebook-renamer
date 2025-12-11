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

// Series abbreviation mappings
var seriesMappings = []struct {
	FullName string
	Abbr     string
}{
	{"Graduate Texts in Mathematics", "GTM"},
	{"Cambridge Studies in Advanced Mathematics", "CSAM"},
	{"London Mathematical Society Lecture Note Series", "LMSLN"},
	{"Progress in Mathematics", "PM"},
	{"Springer Undergraduate Mathematics Series", "SUMS"},
	{"Graduate Studies in Mathematics", "GSM"},
	{"AMS Mathematical Surveys and Monographs", "AMS-MSM"},
	{"Oxford Graduate Texts in Mathematics", "OGTM"},
	{"Springer Monographs in Mathematics", "SMM"},
}

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

	// Duplicate marker patterns
	dupMarkerEndRegex      = regexp.MustCompile(`[-\s]*\(\d{1,2}\)\s*$`)
	dupMarkerDashEndRegex  = regexp.MustCompile(`-\d{1,2}\s*$`)
	dupMarkerBeforeYearRegex = regexp.MustCompile(`-\d{1,2}\s+\(`)

	// Edition patterns
	editionPatterns = []*regexp.Regexp{
		regexp.MustCompile(`(\d+)(?:st|nd|rd|th)\s+[Ee]dition`),
		regexp.MustCompile(`(\d+)(?:st|nd|rd|th)\s+[Ee]d\.?`),
		regexp.MustCompile(`[Ee]dition\s+(\d+)`),
	}

	// Volume patterns
	volumePatterns = []struct {
		Pattern           *regexp.Regexp
		AlreadyNormalized bool
	}{
		{regexp.MustCompile(`\bVol\.?\s+(\d+)\b`), true},
		{regexp.MustCompile(`\bVolume\s+(\d+)\b`), false},
		{regexp.MustCompile(`\bPart\s+(\d+)\b`), false},
	}
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

	// Step 2: Extract series information (before removal)
	seriesInfo, baseAfterSeries := extractSeriesInfo(base)
	base = baseAfterSeries

	// Step 3: Remove ALL bracketed annotations
	base = bracketRegex.ReplaceAllString(base, "")

	// Step 4: Clean noise sources (Z-Library, etc.)
	// MUST happen BEFORE author parsing
	base = cleanNoiseSources(base)

	// Step 5: Remove duplicate markers: -2, -3, (1), (2)
	base = removeDuplicateMarkers(base)

	// Step 6: Extract edition information
	editionInfo, baseAfterEdition := extractEdition(base)
	base = baseAfterEdition

	// Step 7: Extract year FIRST
	year := extractYear(base)

	// Step 8: Remove parentheticals
	base = cleanParentheticals(base, year)

	// Step 9: Extract volume information from title
	volumeInfo, baseAfterVolume := extractVolume(base)
	base = baseAfterVolume

	// Step 10: Parse author and title
	authors, title := smartParseAuthorTitle(base)

	return types.ParsedMetadata{
		Authors: authors,
		Title:   title,
		Year:    year,
		Series:  seriesInfo,
		Edition: editionInfo,
		Volume:  volumeInfo,
	}, nil
}

// extractSeriesInfo extracts series information and returns (series_info, remaining_string)
func extractSeriesInfo(s string) (*string, string) {
	result := s

	// Pattern 1: "Series Name Volume - Author - Title"
	for _, mapping := range seriesMappings {
		pattern := fmt.Sprintf(`^%s\s*(\d+)\s*[-\s]`, regexp.QuoteMeta(mapping.FullName))
		re := regexp.MustCompile("(?i)" + pattern)
		if matches := re.FindStringSubmatch(result); matches != nil {
			seriesInfo := fmt.Sprintf("%s %s", mapping.Abbr, matches[1])
			result = re.ReplaceAllString(result, "")
			return &seriesInfo, strings.TrimSpace(result)
		}
	}

	// Pattern 2: "Series Name - Author - Title" (no volume number)
	for _, mapping := range seriesMappings {
		pattern := fmt.Sprintf(`^%s\s*-\s*`, regexp.QuoteMeta(mapping.FullName))
		re := regexp.MustCompile("(?i)" + pattern)
		if re.MatchString(result) {
			result = re.ReplaceAllString(result, "")
			return nil, strings.TrimSpace(result)
		}
	}

	// Pattern 3: "(Series Name Volume) Author - Title"
	reParenSeries := regexp.MustCompile(`^\s*\(([^)]+?)\s+(\d+)\)\s*`)
	if matches := reParenSeries.FindStringSubmatch(result); matches != nil {
		seriesPart := matches[1]
		volumePart := matches[2]

		for _, mapping := range seriesMappings {
			if strings.Contains(strings.ToLower(seriesPart), strings.ToLower(mapping.FullName)) {
				seriesInfo := fmt.Sprintf("%s %s", mapping.Abbr, volumePart)
				result = reParenSeries.ReplaceAllString(result, "")
				return &seriesInfo, strings.TrimSpace(result)
			}
		}
	}

	// Pattern 4: "[Series Name Volume]" in brackets
	reBracketSeries := regexp.MustCompile(`\s*\[([^\]]+?)\s+(\d+)\]`)
	if matches := reBracketSeries.FindStringSubmatch(result); matches != nil {
		seriesPart := matches[1]
		volumePart := matches[2]

		for _, mapping := range seriesMappings {
			if strings.Contains(strings.ToLower(seriesPart), strings.ToLower(mapping.FullName)) {
				seriesInfo := fmt.Sprintf("%s %s", mapping.Abbr, volumePart)
				result = reBracketSeries.ReplaceAllString(result, "")
				return &seriesInfo, strings.TrimSpace(result)
			}
		}
	}

	// Fallback: Generic pattern (Series Name) Author - Title without volume
	reGeneric := regexp.MustCompile(`^\s*\(([^)]+)\)\s+(.+)$`)
	if matches := reGeneric.FindStringSubmatch(result); matches != nil {
		restPart := matches[2]

		// Check if 'restPart' starts with an author
		reSep := regexp.MustCompile(`(?:--|[-:])`)
		potentialAuthor := restPart
		if mat := reSep.FindStringIndex(restPart); mat != nil {
			potentialAuthor = restPart[:mat[0]]
		}

		if isLikelyAuthor(potentialAuthor) {
			return nil, strings.TrimSpace(restPart)
		}
	}

	return nil, strings.TrimSpace(result)
}

// extractEdition extracts edition information and returns (edition_info, remaining_string)
func extractEdition(s string) (*string, string) {
	result := s

	for _, pattern := range editionPatterns {
		if matches := pattern.FindStringSubmatch(result); matches != nil {
			numStr := matches[1]
			suffix := getOrdinalSuffix(numStr)
			editionInfo := fmt.Sprintf("%s%s ed", numStr, suffix)
			result = pattern.ReplaceAllString(result, "")
			return &editionInfo, strings.TrimSpace(result)
		}
	}

	return nil, strings.TrimSpace(result)
}

// getOrdinalSuffix returns the ordinal suffix for a number
func getOrdinalSuffix(numStr string) string {
	switch numStr {
	case "1":
		return "st"
	case "2":
		return "nd"
	case "3":
		return "rd"
	default:
		return "th"
	}
}

// extractVolume extracts volume information and returns (volume_info, normalized_string)
func extractVolume(s string) (*string, string) {
	for _, vp := range volumePatterns {
		if matches := vp.Pattern.FindStringSubmatch(s); matches != nil {
			volumeInfo := fmt.Sprintf("Vol %s", matches[1])
			var normalizedText string
			if !vp.AlreadyNormalized {
				// Replace "Volume N" or "Part N" with "Vol N"
				normalizedText = vp.Pattern.ReplaceAllString(s, volumeInfo)
			} else {
				normalizedText = s
			}
			return &volumeInfo, normalizedText
		}
	}

	return nil, s
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
		// Hash patterns (MD5/SHA hashes)
		`\s*--\s*[a-f0-9]{32}\s*(?:--)?`,
		// ISBN-like patterns (10-13 digits)
		`\s*--\s*\d{10,13}\s*(?:--)?`,
		// Long alphanumeric IDs (16+ chars)
		`\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?`,
		// Shorter hash patterns (8+ hex chars)
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
	// Apply patterns multiple times to handle consecutive patterns
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
	s = dupMarkerEndRegex.ReplaceAllString(s, "")

	// -2, -3 at end
	s = dupMarkerDashEndRegex.ReplaceAllString(s, "")

	// -2 before (year)
	s = dupMarkerBeforeYearRegex.ReplaceAllString(s, " (")

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
	// e.g. "Title - Publisher"
	if idx := strings.LastIndex(s, " - "); idx != -1 {
		suffix := s[idx+3:]
		if isPublisherOrSeriesInfo(suffix) {
			s = s[:idx]
		}
	}
	// Also handle just "-" without spaces if it looks like publisher
	if idx := strings.LastIndex(s, "-"); idx != -1 {
		if idx > 0 && idx < len(s)-1 {
			suffix := strings.TrimSpace(s[idx+1:])
			// Use stricter check for non-spaced dash to avoid stripping parts of title
			if isStrictPublisherInfo(suffix) {
				s = s[:idx]
			}
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
	// Stricter version for suffix stripping (no parens)
	strictKeywords := []string{
		"Press",
		"Publishing",
		"Springer",
		"Cambridge",
		"Oxford",
		"MIT",
		"Wiley",
		"Elsevier",
		"Routledge",
		"Pearson",
		"McGraw",
		"Addison",
		"Prentice",
		"O'Reilly",
		"Princeton",
		"Harvard",
		"Yale",
		"Stanford",
		"Chicago",
		"California",
		"Columbia",
		"University",
		"Verlag",
		"Birkhäuser",
		"CUP",
	}

	for _, keyword := range strictKeywords {
		if strings.Contains(s, keyword) {
			return true
		}
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
			} else {
                // If no open paren, we skip this closing paren
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
	for strings.HasSuffix(resultStr, "(") || strings.HasSuffix(resultStr, "[") {
		resultStr = resultStr[:len(resultStr)-1]
	}

	return strings.TrimSpace(resultStr)
}

func generateNewFilename(metadata types.ParsedMetadata, extension string) string {
	var result strings.Builder

	// Author(s)
	if metadata.Authors != nil {
		result.WriteString(*metadata.Authors)
		result.WriteString(" - ")
	}

	// Title (volume is kept in title if present)
	result.WriteString(metadata.Title)

	// Series info in brackets
	if metadata.Series != nil {
		result.WriteString(fmt.Sprintf(" [%s]", *metadata.Series))
	}

	// Year and Edition in parentheses
	if metadata.Year != nil && metadata.Edition != nil {
		result.WriteString(fmt.Sprintf(" (%d, %s)", *metadata.Year, *metadata.Edition))
	} else if metadata.Year != nil {
		result.WriteString(fmt.Sprintf(" (%d)", *metadata.Year))
	} else if metadata.Edition != nil {
		result.WriteString(fmt.Sprintf(" (%s)", *metadata.Edition))
	}

	result.WriteString(extension)
	return result.String()
}

func filepathJoin(dir, file string) string {
	return filepath.Join(dir, file)
}
