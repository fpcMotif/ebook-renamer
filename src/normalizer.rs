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
    // Remove extension and any .download suffix
    let base = if filename.ends_with(".download") {
        filename.trim_end_matches(".download")
    } else {
        filename
    };

    let base = if base.ends_with(extension) {
        &base[..base.len() - extension.len()]
    } else {
        base
    };

    let base = base.trim();

    // Clean up obvious noise/series prefixes
    let base = strip_prefix_noise(base);

    // Clean source indicators BEFORE parsing authors and titles
    let base = clean_source_indicators(base);

    // Extract year (4 digits: 19xx or 20xx)
    let year = extract_year(&base);

    // Remove year and surrounding brackets/parens from base for further processing
    let base_without_year = remove_year_from_string(&base);

    // Try to split authors and title by common separators
    let (authors, title) = split_authors_and_title(&base_without_year);

    Ok(ParsedMetadata {
        authors,
        title,
        year,
    })
}

fn strip_prefix_noise(s: &str) -> &str {
    let mut s = s;

    // Common series/collection prefixes to remove
    let prefixes = [
        "London Mathematical Society Lecture Note Series",
        "Graduate Texts in Mathematics",
        "Progress in Mathematics",
        "[Springer-Lehrbuch]",
        "[Graduate studies in mathematics",
        "[Progress in Mathematics №",
        "[AMS Mathematical Surveys and Monographs",
    ];

    for prefix in &prefixes {
        if let Some(stripped) = s.strip_prefix(prefix) {
            s = stripped.trim_start_matches(|c: char| c == ' ' || c == '-');
            break;
        }
    }

    s
}

fn clean_source_indicators(s: &str) -> String {
    let mut s = s.to_string();
    
    // Remove trailing source markers - same patterns as clean_title
    let noise_patterns = [
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
    ];

    for pattern in &noise_patterns {
        if s.ends_with(pattern) {
            s = s[..s.len() - pattern.len()].to_string();
        }
    }

    s.trim().to_string()
}

fn extract_year(s: &str) -> Option<u16> {
    let re = Regex::new(r"\b(19|20)\d{2}\b").ok()?;
    
    // Collect all year matches
    let mut years: Vec<u16> = re
        .find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .collect();

    // Return the last year found (usually most relevant)
    years.pop()
}

fn remove_year_from_string(s: &str) -> String {
    // Remove year patterns but keep the rest of the string
    // Pattern: (YYYY, Publisher) or (YYYY) or YYYY,
    let re = Regex::new(r"\s*\(\s*(19|20)\d{2}\s*(?:,\s*[^)]+)?\s*\)").unwrap();
    let result = re.replace(s, "").to_string();
    
    // Also remove standalone year with comma: "2020, Publisher"
    let re2 = Regex::new(r"\s*(19|20)\d{2}\s*,\s*[^,]+$").unwrap();
    re2.replace(&result, "").to_string()
}

fn split_authors_and_title(s: &str) -> (Option<String>, String) {
    // Look for patterns like "Author - Title" or "Author: Title"
    // Also handle " (Author)" at the end

    // First, check for trailing (author) pattern
    if let Some(paren_idx) = s.rfind('(') {
        if s.ends_with(')') {
            let potential_author = s[paren_idx + 1..s.len() - 1].trim();
            if is_likely_author(potential_author) {
                let title = s[..paren_idx].trim().to_string();
                return (Some(potential_author.to_string()), title);
            }
        }
    }

    // Check for " - " separator (most common)
    if let Some(dash_idx) = s.rfind(" - ") {
        let maybe_author = s[..dash_idx].trim();
        let maybe_title = s[dash_idx + 3..].trim();
        
        if is_likely_author(maybe_author) && !maybe_title.is_empty() {
            return (
                Some(clean_author_name(maybe_author)),
                clean_title(maybe_title),
            );
        }
    }

    // Check for ":" separator
    if let Some(colon_idx) = s.find(':') {
        let maybe_author = s[..colon_idx].trim();
        let maybe_title = s[colon_idx + 1..].trim();
        
        if is_likely_author(maybe_author) && !maybe_title.is_empty() {
            return (
                Some(clean_author_name(maybe_author)),
                clean_title(maybe_title),
            );
        }
    }

    // If no clear separator, treat entire string as title
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

    if non_author_keywords.iter().any(|k| s.contains(k)) {
        return false;
    }

    // Check if looks like a name (has at least one uppercase letter)
    s.chars().any(|c| c.is_uppercase())
}

fn clean_author_name(s: &str) -> String {
    let s = s.trim();
    
    // Remove trailing (auth.) etc.
    let re = Regex::new(r"\s*\(auth\.\).*$").unwrap();
    let s = re.replace(s, "").to_string();

    // Normalize author name format: "Firstname Lastname" or "Lastname, Firstname"
    // Keep as-is if already looks good
    s.trim().to_string()
}

fn clean_title(s: &str) -> String {
    let mut s = s.trim().to_string();

    // Remove trailing source markers - more comprehensive patterns
    let noise_patterns = [
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
    ];

    for pattern in &noise_patterns {
        if s.ends_with(pattern) {
            s = s[..s.len() - pattern.len()].to_string();
        }
    }

    // Remove trailing .download suffix
    while s.ends_with(".download") {
        s.pop();
    }

    // Remove (auth.) and similar patterns
    let re_auth = Regex::new(r"\s*\([Aa]uth\.?\)").unwrap();
    s = re_auth.replace_all(&s, "").to_string();

    // Clean up orphaned brackets/parens
    s = clean_orphaned_brackets(&s);

    // Remove multiple spaces
    let re = Regex::new(r"\s+").unwrap();
    s = re.replace_all(&s, " ").to_string();

    // Remove leading/trailing punctuation
    s = s.trim_matches(|c: char| c == '-' || c == ':' || c == ',' || c == ';').to_string();

    s.trim().to_string()
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
            "London Mathematical Society Lecture Note Series B. R. Tennison - Sheaf Theory (1976).pdf",
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
        // Orphaned closing should be removed
        assert!(result.chars().filter(|&c| c == ')').count() <= result.chars().filter(|&c| c == '(').count());
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
    fn test_remove_year_from_string_with_publisher() {
        let result = remove_year_from_string("Title (2005, Birkhäuser) - libgen.li");
        assert_eq!(result, "Title - libgen.li");
    }

    #[test]
    fn test_remove_year_from_string_standalone() {
        let result = remove_year_from_string("Title 2020, Publisher Name");
        assert_eq!(result, "Title");
    }

    #[test]
    fn test_clean_title_comprehensive_sources() {
        let test_cases = vec![
            ("Title - libgen.li", "Title"),
            ("Title - Z-Library", "Title"),
            ("Title - z-Library", "Title"),
            ("Title (libgen.li)", "Title"),
            ("Title libgen.li.pdf", "Title"),
            ("Title Z-Library.pdf", "Title"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(clean_title(input), expected);
        }
    }
}

