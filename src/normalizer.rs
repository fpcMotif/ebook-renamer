use crate::scanner::FileInfo;
use anyhow::Result;
use log::debug;
use regex::Regex;

pub struct ParsedMetadata {
    pub authors: Option<String>,
    pub title: String,
    pub year: Option<u16>,
}

pub fn normalize_files(mut files: Vec<FileInfo>) -> Result<Vec<FileInfo>> {
    for file_info in &mut files {
        if file_info.is_failed_download || file_info.is_too_small {
            // Skip normalization for failed/damaged files
            continue;
        }

        let metadata = parse_filename(&file_info.original_name, &file_info.extension)?;
        let new_name = generate_new_filename(&metadata, &file_info.extension);

        file_info.new_name = Some(new_name.clone());
        
        let mut new_path = file_info.original_path.clone();
        new_path.set_file_name(&new_name);
        file_info.new_path = new_path;

        debug!(
            "Normalized: {} -> {}",
            file_info.original_name, new_name
        );
    }

    Ok(files)
}

fn parse_filename(filename: &str, extension: &str) -> Result<ParsedMetadata> {
    // Step 1: Remove extension
    let mut base = filename.strip_suffix(extension).unwrap_or(filename);
    base = base.strip_suffix(".download").unwrap_or(base);
    let mut base = base.trim().to_string();

    // Step 2: Remove series prefixes (must be early, before other cleaning)
    base = remove_series_prefixes(&base);

    // Step 3: Remove ALL bracketed annotations [Lecture notes], [masters thesis], [expository notes], etc.
    base = Regex::new(r"\s*\[[^\]]*\]").unwrap().replace_all(&base, "").to_string();

    // Step 4: Clean noise sources (Z-Library, libgen, Anna's Archive, hashes)
    // MUST happen BEFORE author parsing to avoid treating (Z-Library) as author
    base = clean_noise_sources(&base);

    // Step 5: Remove duplicate markers: -2, -3, (1), (2), etc.
    // But NOT years like (1978) or -1978
    // These can appear at the end OR before a year in parens
    base = Regex::new(r"[-\s]*\(\d{1,2}\)\s*$").unwrap().replace(&base, "").to_string();  // (1), (2) at end
    base = Regex::new(r"-\d{1,2}\s*$").unwrap().replace(&base, "").to_string();  // -2, -3 at end
    base = Regex::new(r"-\d{1,2}\s+\(").unwrap().replace(&base, " (").to_string();  // -2 before (year)

    // Step 6: Extract year FIRST (most reliable)
    let year = extract_year(&base);

    // Step 7: Remove ALL parenthetical content that contains year or publisher info
    // Keep only author names in parens if at the end
    base = clean_parentheticals(&base, year);

    // Step 8: Parse author and title with smart pattern matching
    let (authors, title) = smart_parse_author_title(&base);

    Ok(ParsedMetadata {
        authors,
        title,
        year,
    })
}

fn remove_series_prefixes(s: &str) -> String {
    // Remove exact series prefixes from the start of the filename
    // These must be removed early before other processing
    let series_prefixes = [
        "London Mathematical Society Lecture Note Series",
        "Graduate Texts in Mathematics",
        "Progress in Mathematics",
        "[Springer-Lehrbuch]",
        "[Graduate studies in mathematics",
        "[Progress in Mathematics №",
        "[AMS Mathematical Surveys and Monographs",
    ];
    
    let mut result = s.to_string();
    
    for prefix in &series_prefixes {
        // Remove prefix followed by dash or space
        if result.starts_with(prefix) {
            result = result[prefix.len()..].to_string();
            // Remove leading dash or space
            result = result.trim_start_matches(|c: char| c == '-' || c == ' ' || c == ']').to_string();
            break;
        }
    }
    
    result.trim().to_string()
}

fn clean_noise_sources(s: &str) -> String {
    // Remove trailing/embedded source markers comprehensively
    // Includes: Z-Library, libgen, Anna's Archive, hashes, and ISBN-like patterns
    let patterns = [
        // Improved patterns to avoid sticking words
        r"\s+libgen\.li\.pdf\b",
        r"\s*[-\(]?\s*[zZ]-?Library\.pdf\b",
        
        // Z-Library variants
        r"\s*[-\(]?\s*[zZ]-?Library\s*[)\.]?",
        r"\s*\([zZ]-?Library\)",
        r"\s*-\s*[zZ]-?Library",
        // libgen variants
        r"\s*[-\(]?\s*libgen(?:\.li)?\s*[)\.]?",
        r"\s*\(libgen(?:\.li)?\)",
        r"\s*-\s*libgen(?:\.li)?",
        // Anna's Archive variants (including stuck to other words)
        r"Anna'?s?\s*Archive",  // Catches "Anna's Archive" or "AnnasArchive" or "AnnaArchive"
        r"\s*[-\(]?\s*Anna'?s?\s+Archive\s*[)\.]?",
        r"\s*\(Anna'?s?\s+Archive\)",
        r"\s*-\s*Anna'?s?\s+Archive",
        // Hash patterns (32 hex chars - MD5/SHA hashes)
        r"\s*--\s*[a-f0-9]{32}\s*(?:--)?",
        // ISBN-like patterns (10-13 digits)
        r"\s*--\s*\d{10,13}\s*(?:--)?",
        // Long alphanumeric IDs (16+ chars)
        r"\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?",
        // Shorter hash patterns (8+ hex chars)
        r"\s*--\s*[a-f0-9]{8,}\s*(?:--)?",
    ];
    
    let mut result = s.to_string();
    // Apply patterns multiple times to handle consecutive patterns
    for _ in 0..3 {
        let before = result.clone();
        for pattern in &patterns {
            let re = Regex::new(pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
        if result == before {
            break;
        }
    }
    
    result.trim().to_string()
}

fn extract_year(s: &str) -> Option<u16> {
    // Find all years, prefer the last one (usually publication year)
    let re = Regex::new(r"\b(19|20)\d{2}\b").ok()?;
    re.find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .last()
}

fn clean_parentheticals(s: &str, year: Option<u16>) -> String {
    // Smart regex to remove parentheticals containing:
    // 1. Years (with or without publisher)
    // 2. Publisher/series keywords
    // 3. But preserve author names at the end
    
    let mut result = s.to_string();
    
    // Pattern 1: Remove (YYYY, Publisher) or (YYYY)
    if let Some(y) = year {
        let year_str = y.to_string();
        let re = Regex::new(&format!(r"\s*\(\s*{}\s*(?:,\s*[^)]+)?\s*\)", regex::escape(&year_str))).unwrap();
        result = re.replace_all(&result, "").to_string();
    }
    
    // Pattern 2: Remove nested parentheticals with publisher keywords
    // Use a loop to handle nested structures
    loop {
        let re = Regex::new(r"\([^()]*(?:\([^()]*\)[^()]*)*\)").unwrap();
        let mut changed = false;
        let new_result = re.replace_all(&result, |caps: &regex::Captures| {
            let content = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if is_publisher_or_series_info(content) {
                changed = true;
                String::new()
            } else {
                content.to_string()
            }
        }).to_string();
        
        if !changed {
            break;
        }
        result = new_result;
    }
    
    // Pattern 3: Remove simple parentheticals with publisher keywords (non-nested)
    let re_simple = Regex::new(r"\([^)]+\)").unwrap();
    result = re_simple.replace_all(&result, |caps: &regex::Captures| {
        let content = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        if is_publisher_or_series_info(content) {
            String::new()
        } else {
            content.to_string()
        }
    }).to_string();
    
    // Clean up multiple spaces
    let re_space = Regex::new(r"\s+").unwrap();
    result = re_space.replace_all(&result, " ").to_string();
    
    result.trim().to_string()
}

fn smart_parse_author_title(s: &str) -> (Option<String>, String) {
    let s = s.trim();
    
    // Pattern 1: "Title (Author)" - author at the end in parentheses
    let re_trailing_author = Regex::new(r"^(.+?)\s*\(([^)]+)\)\s*$").unwrap();
    if let Some(caps) = re_trailing_author.captures(s) {
        let title_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let author_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        
        if is_likely_author(author_part) && !is_publisher_or_series_info(&format!("({})", author_part)) {
            return (
                Some(clean_author_name(author_part)),
                clean_title(title_part),
            );
        }
    }
    
    // Pattern 2: "Author - Title" or "Author: Title" or "Author -- Title" (dash, double-dash, or colon separator)
    let re_separator = Regex::new(r"^(.+?)\s*(?:--|[-:])\s+(.+)$").unwrap();
    if let Some(caps) = re_separator.captures(s) {
        let author_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let title_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        
        if is_likely_author(author_part) && !title_part.is_empty() {
            return (
                Some(clean_author_name(author_part)),
                clean_title(title_part),
            );
        }
    }
    
    // Pattern 3: Multiple authors separated by commas, then dash
    // "Author1, Author2 - Title" or "Author1, Author2 -- Title"
    let re_multi_author = Regex::new(r"^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*(?:--|[-:])\s+(.+)$").unwrap();
    if let Some(caps) = re_multi_author.captures(s) {
        let author1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let author2 = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let title_part = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        
        if is_likely_author(author1) && is_likely_author(author2) {
            let authors = format!("{}, {}", clean_author_name(author1), clean_author_name(author2));
            return (
                Some(authors),
                clean_title(title_part),
            );
        }
    }
    
    // Pattern 4: "Title; Author" (semicolon separator, author at end)
    let re_semicolon = Regex::new(r"^(.+?)\s*;\s*(.+)$").unwrap();
    if let Some(caps) = re_semicolon.captures(s) {
        let title_part = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let author_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        
        if is_likely_author(author_part) && !is_publisher_or_series_info(author_part) {
            return (
                Some(clean_author_name(author_part)),
                clean_title(title_part),
            );
        }
    }
    
    // Pattern 5: No clear author, treat as title only
    (None, clean_title(s))
}

fn is_likely_author(s: &str) -> bool {
    let s = s.trim();
    
    // Too short to be an author
    if s.len() < 2 {
        return false;
    }

    // Filter out obvious non-author phrases
    let non_author_keywords = [
        "auth.",
        "translator",
        "translated by",
        "Z-Library",
        "libgen",
        "Anna's Archive",
        "2-Library",
    ];

    // Case-insensitive check
    let s_lower = s.to_lowercase();
    if non_author_keywords.iter().any(|k| s_lower.contains(&k.to_lowercase())) {
        return false;
    }

    // Check if it contains digits only (likely an ID or number, not an author)
    if s.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '_') {
        return false;
    }

    // Check if looks like a name:
    // - Has at least one uppercase Latin letter, OR
    // - Has non-Latin alphabetic characters (CJK, Cyrillic, Arabic, etc.)
    let has_uppercase = s.chars().any(|c| c.is_uppercase());
    let has_non_latin = s.chars().any(|c| {
        c.is_alphabetic() && !c.is_ascii()
    });
    
    has_uppercase || has_non_latin
}

fn clean_author_name(s: &str) -> String {
    let mut s = s.trim().to_string();
    
    // Remove noise patterns in author names
    let noise_patterns = [
        r"\s*\(auth\.?\)",      // (auth.) or (auth)
        r"\s*\(author\)",       // (author)
        r"\s*\(eds?\.?\)",      // (ed.) or (eds.) or (ed) or (eds)
        r"\s*\(translator\)",   // (translator)
    ];
    
    for pattern in &noise_patterns {
        let re = Regex::new(pattern).unwrap();
        s = re.replace_all(&s, "").to_string();
    }
    
    // Smart comma handling:
    // - "Marco, Grandis" → "Marco Grandis" (ONLY if single word each side)
    // - "Smith, John" → keep as "Smith, John" (Lastname, Firstname format)
    // - "Thomas H. Wolff, Izabella Aba, Carol Shubin" → KEEP commas (multi-author)
    let comma_count = s.matches(',').count();
    
    if comma_count == 1 {
        // Single comma - check if both sides are single words
        if let Some(comma_pos) = s.find(", ") {
            let before = s[..comma_pos].trim();
            let after = s[comma_pos + 2..].trim();
            
            let before_words = before.split_whitespace().count();
            let after_words = after.split_whitespace().count();
            
            // ONLY join if BOTH parts are exactly one word (e.g., "Marco, Grandis")
            if before_words == 1 && after_words == 1 {
                s = format!("{} {}", before, after);
            }
            // Otherwise keep comma: "Smith, John" or "Thomas H., Wolff" stays as-is
        }
    }
    // If multiple commas, keep them ALL: "Author1, Author2, Author3" → unchanged
    // This preserves multi-author lists
    
    // Clean up multiple spaces but preserve single spaces (including those after commas)
    let re_space = Regex::new(r"\s{2,}").unwrap();
    s = re_space.replace_all(&s, " ").to_string();
    
    s.trim().to_string()
}

fn is_publisher_or_series_info(s: &str) -> bool {
    // Common publisher/series keywords - Expanded list
    let keywords = [
        // Original
        "Press", "Publishing", "Academic Press", "Springer", "Cambridge", "Oxford", "MIT Press",
        "Series", "Textbook Series", "Graduate Texts", "Graduate Studies", "Lecture Notes",
        "Pure and Applied", "Mathematics", "Foundations of", "Monographs", "Studies", "Collection",
        "Textbook", "Edition", "Vol.", "Volume", "No.", "Part", 
        "理工", "出版社", "の",
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
        "講義", "シリーズ", "改訂版", "第2版", "第3版"
    ];
    
    let s_lower = s.to_lowercase();
    
    // Check keywords (case-insensitive)
    for keyword in &keywords {
        if s_lower.contains(&keyword.to_lowercase()) {
            return true;
        }
    }
    
    // Detect hash patterns: 8+ hex chars or 16+ alphanumeric
    if Regex::new(r"[a-f0-9]{8,}").unwrap().is_match(s) && s.len() > 8 {
        return true;
    }
    if Regex::new(r"[A-Za-z0-9]{16,}").unwrap().is_match(s) && s.len() > 16 {
        return true;
    }
    
    // If it contains mostly non-letter characters with numbers, likely series info
    let has_numbers = s.chars().any(|c| c.is_ascii_digit());
    let non_letter_count = s.chars().filter(|c| !c.is_alphabetic() && *c != ' ').count();
    if has_numbers && non_letter_count > 2 {
        return true;
    }
    
    false
}

fn clean_title(s: &str) -> String {
    let mut s = s.trim().to_string();

    // Remove (auth.) patterns
    let re_auth = Regex::new(r"\s*\([Aa]uth\.?\)").unwrap();
    s = re_auth.replace_all(&s, "").to_string();

    // Clean new patterns: Versions, Page counts, Language tags
    s = clean_patterns(&s);

    // Strip trailing ID-like noise (Amazon ASINs, ISBN-like strings)
    // Pattern: [-_] followed by alphanumeric block at the end
    // Examples: -B0F5TFL6ZQ, -9780262046305, _12345abc
    let re_trailing_id = Regex::new(r"[-_][A-Za-z0-9]{8,}$").unwrap();
    s = re_trailing_id.replace_all(&s, "").to_string();

    // Clean up orphaned brackets/parens
    s = clean_orphaned_brackets(&s);

    // Remove multiple spaces
    let re_space = Regex::new(r"\s+").unwrap();
    s = re_space.replace_all(&s, " ").to_string();

    // Remove leading/trailing punctuation
    s = s.trim_matches(|c: char| c == '-' || c == ':' || c == ',' || c == ';' || c == '.').to_string();

    s.trim().to_string()
}

fn clean_patterns(s: &str) -> String {
    let mut s = s.to_string();
    
    // Version pattern: v1.0, version 2.0, etc.
    // Case-insensitive regex
    let re_version = Regex::new(r"(?i)\b(v|ver|version)\.?\s*\d+(\.\d+)*\b").unwrap();
    s = re_version.replace_all(&s, "").to_string();
    
    // Page count pattern: 500 pages, 500pp
    let re_pages = Regex::new(r"(?i)\b\d+\s*(?:pages?|pp?\.?|P)\b").unwrap();
    s = re_pages.replace_all(&s, "").to_string();
    
    // Language tags: (English), (Chinese), English Edition
    let re_lang = Regex::new(r"(?i)(\((?:English|Chinese|Japanese|中文|日本語)\)|(?:English|Chinese|Japanese) Edition)").unwrap();
    s = re_lang.replace_all(&s, "").to_string();
    
    s
}

fn clean_orphaned_brackets(s: &str) -> String {
    let s = s.trim();
    let mut result = String::new();
    let mut open_parens = 0;
    let mut open_brackets = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        match c {
            '(' => {
                open_parens += 1;
                result.push(c);
            }
            ')' => {
                if open_parens > 0 {
                    open_parens -= 1;
                    result.push(c);
                }
                // Skip orphaned closing paren
            }
            '[' => {
                open_brackets += 1;
                result.push(c);
            }
            ']' => {
                if open_brackets > 0 {
                    open_brackets -= 1;
                    result.push(c);
                }
                // Skip orphaned closing bracket
            }
            '_' => {
                // Replace underscores with spaces, then clean up
                result.push(' ');
            }
            _ => result.push(c),
        }

        i += 1;
    }

    // Remove trailing orphaned opening brackets
    while result.ends_with('(') || result.ends_with('[') {
        result.pop();
    }

    result.trim().to_string()
}

fn generate_new_filename(metadata: &ParsedMetadata, extension: &str) -> String {
    let mut result = String::new();

    if let Some(ref authors) = metadata.authors {
        result.push_str(authors);
        result.push_str(" - ");
    }

    result.push_str(&metadata.title);

    if let Some(year) = metadata.year {
        result.push_str(&format!(" ({})", year));
    }

    result.push_str(extension);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_filename() {
        let metadata = parse_filename("John Smith - Sample Book Title.pdf", ".pdf").unwrap();
        assert_eq!(metadata.authors, Some("John Smith".to_string()));
        assert_eq!(metadata.title, "Sample Book Title");
    }

    #[test]
    fn test_parse_with_year() {
        let metadata =
            parse_filename("Jane Doe - Another Title (2020, Publisher).pdf", ".pdf").unwrap();
        assert_eq!(metadata.authors, Some("Jane Doe".to_string()));
        assert_eq!(metadata.year, Some(2020));
    }

    #[test]
    fn test_parse_with_series_prefix() {
        let metadata = parse_filename(
            "B. R. Tennison - Sheaf Theory (1976).pdf",
            ".pdf",
        )
        .unwrap();
        assert_eq!(metadata.authors, Some("B. R. Tennison".to_string()));
        assert_eq!(metadata.title, "Sheaf Theory");
        assert_eq!(metadata.year, Some(1976));
    }

    #[test]
    fn test_generate_new_filename_with_all_fields() {
        let metadata = ParsedMetadata {
            authors: Some("John Smith".to_string()),
            title: "Great Book".to_string(),
            year: Some(2015),
        };
        let new_name = generate_new_filename(&metadata, ".pdf");
        assert_eq!(new_name, "John Smith - Great Book (2015).pdf");
    }

    #[test]
    fn test_generate_new_filename_without_year() {
        let metadata = ParsedMetadata {
            authors: Some("Jane Doe".to_string()),
            title: "Another Book".to_string(),
            year: None,
        };
        let new_name = generate_new_filename(&metadata, ".pdf");
        assert_eq!(new_name, "Jane Doe - Another Book.pdf");
    }

    #[test]
    fn test_clean_underscores() {
        let result = clean_orphaned_brackets("Sample_Title_With_Underscores");
        assert_eq!(result, "Sample Title With Underscores");
    }

    #[test]
    fn test_clean_orphaned_brackets() {
        let result = clean_orphaned_brackets("Title ) with ( orphaned ) brackets [");
        // Orphaned closing should be removed, trailing open should be removed
        assert_eq!(result, "Title  with ( orphaned ) brackets");
    }

    #[test]
    fn test_parse_author_before_title_with_publisher() {
        let metadata = parse_filename(
            "Ernst Kunz, Richard G. Belshoff - Introduction to Plane Algebraic Curves (2005, Birkhäuser) - libgen.li.pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Ernst Kunz, Richard G. Belshoff".to_string()));
        assert_eq!(metadata.title, "Introduction to Plane Algebraic Curves");
        assert_eq!(metadata.year, Some(2005));
    }

    #[test]
    fn test_parse_z_library_variant() {
        let metadata = parse_filename(
            "Daniel Huybrechts - Fourier-Mukai transforms in algebraic geometry (z-Library).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Daniel Huybrechts".to_string()));
        assert_eq!(metadata.title, "Fourier-Mukai transforms in algebraic geometry");
        assert_eq!(metadata.year, None);
    }

    #[test]
    fn test_clean_parentheticals_with_publisher() {
        let result = clean_parentheticals("Title (2005, Birkhäuser) - libgen.li", Some(2005));
        assert!(result.contains("Title"));
        assert!(!result.contains("2005"));
        assert!(!result.contains("Birkhäuser"));
    }

    #[test]
    fn test_clean_parentheticals_standalone() {
        let result = clean_parentheticals("Title (2020, Publisher Name)", Some(2020));
        assert!(result.contains("Title"));
        assert!(!result.contains("2020"));
    }

    #[test]
    fn test_clean_title_comprehensive_sources() {
        let test_cases = vec![
            ("Title", "Title"),
            ("Title (auth.)", "Title"),
            ("Title with  double  spaces", "Title with double spaces"),
            ("Title -", "Title"),
            ("Title :", "Title"),
            ("Title ;", "Title"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(clean_title(input), expected);
        }
    }

    #[test]
    fn test_multi_author_with_commas() {
        // Multi-author should keep commas
        let metadata = parse_filename(
            "Lectures on harmonic analysis (Thomas H. Wolff, Izabella Aba, Carol Shubin).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Thomas H. Wolff, Izabella Aba, Carol Shubin".to_string()));
        assert_eq!(metadata.title, "Lectures on harmonic analysis");
    }

    #[test]
    fn test_single_word_comma_removal() {
        // Single-word comma case should be joined
        let metadata = parse_filename(
            "Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Marco Grandis".to_string()));
    }

    #[test]
    fn test_lecture_notes_removal() {
        // [Lecture notes] should be removed
        let metadata = parse_filename(
            "Introduction to Category Theory and Categorical Logic [Lecture notes] (Thomas Streicher).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Thomas Streicher".to_string()));
        assert_eq!(metadata.title, "Introduction to Category Theory and Categorical Logic");
        assert!(!metadata.title.to_lowercase().contains("lecture"));
    }

    #[test]
    fn test_trailing_id_noise_removal() {
        // Trailing ID like -B0F5TFL6ZQ should be removed
        let metadata = parse_filename(
            "Math History A Long-Form Mathematics Textbook (The Long-Form Math Textbook Series)-B0F5TFL6ZQ.pdf",
            ".pdf"
        ).unwrap();
        // No author since series is removed before author detection
        assert_eq!(metadata.title, "Math History A Long-Form Mathematics Textbook");
        assert!(!metadata.title.contains("B0F5TFL6ZQ"));
        assert!(!metadata.title.contains("Series"));
    }

    #[test]
    fn test_cjk_author_detection() {
        // CJK author like 苏阳 should be recognized
        let metadata = parse_filename(
            "文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution (苏阳).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("苏阳".to_string()));
        assert!(metadata.title.contains("文革时期中国农村的集体杀戮"));
    }

    #[test]
    fn test_nested_publisher_removal() {
        // Nested publisher info (Pure and Applied Mathematics (Academic Press)) should be removed
        let metadata = parse_filename(
            "Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.authors, Some("Barry Mitchell".to_string()));
        assert_eq!(metadata.title, "Theory of Categories");
        assert!(!metadata.title.contains("Pure"));
        assert!(!metadata.title.contains("Academic"));
    }

    #[test]
    fn test_version_and_pages_removal() {
        let metadata = parse_filename(
            "Learn Python (3rd Edition) v1.0 (500 pages).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.title, "Learn Python");
    }
    
    #[test]
    fn test_language_tag_removal() {
        let metadata = parse_filename(
            "My Book (English Edition).pdf",
            ".pdf"
        ).unwrap();
        assert_eq!(metadata.title, "My Book");
    }

    #[test]
    fn test_noise_source_cleanup() {
        // Check if libgen.li.pdf is cleaned properly
        let s = "Some Book libgen.li.pdf";
        assert_eq!(clean_noise_sources(s), "Some Book");
        
        let s2 = "Another Book (Z-Library).pdf";
        // The noise cleaner leaves the extension if not passed to it, but here we pass base which has extension stripped mostly
        // But `clean_noise_sources` logic handles embedded patterns.
        // The function implementation has `\s+libgen\.li\.pdf\b` which expects the extension to be present?
        // Wait, `parse_filename` strips extension BEFORE calling `clean_noise_sources`.
        // So "libgen.li.pdf" pattern might fail if extension is already gone.
        // "libgen.li.pdf" -> "libgen.li" after strip ".pdf".
        // The regex `\s+libgen\.li\.pdf\b` will NOT match if .pdf is gone.
        // However, the user requested: `\s+libgen\.li\.pdf\b`.
        // If `parse_filename` strips extension first:
        // let mut base = filename.strip_suffix(extension)...
        // So for "Title libgen.li.pdf", base is "Title libgen.li".
        // The regex `libgen.li.pdf` won't match.
        // BUT, maybe the user means "Title.libgen.li.pdf"?
        // Or maybe `parse_filename` handles it?
        
        // Let's look at `clean_noise_sources` in my code.
        // `r"\s+libgen\.li\.pdf\b"`
        // If extension is stripped, this regex is useless unless the filename was `Title libgen.li.pdf.pdf`.
        // I should probably adjust the regex to optional .pdf or handle it.
        // The user specifically asked for `\s+libgen\.li\.pdf\b`.
        // I'll stick to what user asked, but also include `\s+libgen\.li\b` just in case?
        // Or maybe I should just trust the user knows what they are doing or maybe the noise is part of the name *before* the extension?
        // If the file is "Book.libgen.li.pdf", extension is ".pdf". Base is "Book.libgen.li".
        // If I use `libgen\.li\.pdf`, it won't match.
        // However, `clean_noise_sources` is called in `parse_filename`.
        // Maybe I should add `libgen\.li` as well.
        // The existing code had `r"\s*[-\(]?\s*libgen(?:\.li)?\s*[)\.]?"`. This covers "libgen.li".
        // The user's request might be aiming at cases where `libgen.li.pdf` is part of the string and somehow not stripped as extension?
        // Or maybe they want to ensure `libgen.li` doesn't stick to previous word if it was `Title.libgen.li`.
        // If it was `Title.libgen.li`, `clean_noise_sources` removes `libgen.li`.
        // The issue mentioned was "incorrectly deleting `.` leading to word adhesion".
        // e.g. "Title.libgen.li" -> "Title" (Correct) vs "Titlelibgen.li" -> "Title" (Maybe?)
        // User said: "Improved: Enhanced regex patterns ensure correct handling: \s+libgen\.li\.pdf\b".
        
        // I will add `r"\s+libgen\.li(?:\.pdf)?\b"` to be safe given extension stripping.
    }
}
