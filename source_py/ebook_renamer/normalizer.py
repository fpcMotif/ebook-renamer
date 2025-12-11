"""
Filename normalization functionality for the ebook renamer.
"""

import re
import os
from typing import List, Optional, Tuple

from .types import FileInfo, ParsedMetadata


# Series abbreviation mappings
SERIES_MAPPINGS = [
    ("Graduate Texts in Mathematics", "GTM"),
    ("Cambridge Studies in Advanced Mathematics", "CSAM"),
    ("London Mathematical Society Lecture Note Series", "LMSLN"),
    ("Progress in Mathematics", "PM"),
    ("Springer Undergraduate Mathematics Series", "SUMS"),
    ("Graduate Studies in Mathematics", "GSM"),
    ("AMS Mathematical Surveys and Monographs", "AMS-MSM"),
    ("Oxford Graduate Texts in Mathematics", "OGTM"),
    ("Springer Monographs in Mathematics", "SMM"),
]


class Normalizer:
    """Handles filename normalization according to the specification."""
    
    # Regex patterns
    YEAR_REGEX = re.compile(r'\b(?:19|20)\d{2}\b')
    AUTH_REGEX = re.compile(r'\s*\([Aa]uth\.?\).*')
    SPACE_REGEX = re.compile(r'\s{2,}')
    BRACKET_REGEX = re.compile(r'\s*\[[^\]]*\]')
    TRAILING_ID_REGEX = re.compile(r'[-_][A-Za-z0-9]{8,}$')
    SIMPLE_PAREN_REGEX = re.compile(r'\([^)]+\)')
    # Matches simple nested parens: ( ... ( ... ) ... )
    NESTED_PAREN_REGEX = re.compile(r'\([^()]*(?:\([^()]*\)[^()]*)*\)')
    TRAILING_AUTHOR_REGEX = re.compile(r'^(.+?)\s*\(([^)]+)\)\s*$')
    # Updated to handle double-dash separator
    SEPARATOR_REGEX = re.compile(r'^(.+?)\s*(?:--|[-:])\s+(.+)$')
    MULTI_AUTHOR_REGEX = re.compile(r'^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*(?:--|[-:])\s+(.+)$')
    SEMICOLON_REGEX = re.compile(r'^(.+?)\s*;\s*(.+)$')
    
    # Duplicate marker patterns
    DUP_MARKER_END_REGEX = re.compile(r'[-\s]*\(\d{1,2}\)\s*$')
    DUP_MARKER_DASH_END_REGEX = re.compile(r'-\d{1,2}\s*$')
    DUP_MARKER_BEFORE_YEAR_REGEX = re.compile(r'-\d{1,2}\s+\(')
    
    # Edition patterns
    EDITION_PATTERNS = [
        re.compile(r'(\d+)(?:st|nd|rd|th)\s+[Ee]dition'),
        re.compile(r'(\d+)(?:st|nd|rd|th)\s+[Ee]d\.?'),
        re.compile(r'[Ee]dition\s+(\d+)'),
    ]
    
    # Volume patterns: (pattern, already_normalized)
    VOLUME_PATTERNS = [
        (re.compile(r'\bVol\.?\s+(\d+)\b'), True),
        (re.compile(r'\bVolume\s+(\d+)\b'), False),
        (re.compile(r'\bPart\s+(\d+)\b'), False),
    ]
    
    def normalize_files(self, files: List[FileInfo]) -> List[FileInfo]:
        """Normalize filenames according to the specification."""
        result = []
        
        for file in files:
            # Skip normalization for failed/damaged files
            if file.is_failed_download or file.is_too_small:
                result.append(file)
                continue
            
            metadata = self._parse_filename(file.original_name, file.extension)
            new_name = self._generate_new_filename(metadata, file.extension)
            
            # Update file info
            file.new_name = new_name
            dir_name = os.path.dirname(file.original_path)
            file.new_path = os.path.join(dir_name, new_name)
            result.append(file)
        
        return result
    
    def _parse_filename(self, filename: str, extension: str) -> ParsedMetadata:
        """Parse a filename into metadata components."""
        # Step 1: Remove extension
        base = filename
        if base.endswith(".download"):
            base = base[:-len(".download")]
        if base.endswith(extension):
            base = base[:-len(extension)]
        base = base.strip()
        
        # Step 2: Extract series information (before removal)
        series_info, base = self._extract_series_info(base)
        
        # Step 3: Remove ALL bracketed annotations
        base = self.BRACKET_REGEX.sub("", base)
        
        # Step 4: Clean noise sources
        base = self._clean_noise_sources(base)
        
        # Step 5: Remove duplicate markers
        base = self._remove_duplicate_markers(base)
        
        # Step 6: Extract edition information
        edition_info, base = self._extract_edition(base)
        
        # Step 7: Extract year FIRST
        year = self._extract_year(base)
        
        # Step 8: Remove parentheticals
        base = self._clean_parentheticals(base, year)
        
        # Step 9: Extract volume information from title
        volume_info, base = self._extract_volume(base)
        
        # Step 10: Parse author and title
        authors, title = self._smart_parse_author_title(base)
        
        return ParsedMetadata(
            authors=authors,
            title=title,
            year=year,
            series=series_info,
            edition=edition_info,
            volume=volume_info,
        )
    
    def _extract_series_info(self, s: str) -> Tuple[Optional[str], str]:
        """Extract series information and return (series_info, remaining_string)."""
        result = s

        # Pattern 1: "Series Name Volume - Author - Title"
        for series_name, abbr in SERIES_MAPPINGS:
            pattern = re.compile(rf'^{re.escape(series_name)}\s*(\d+)\s*[-\s]', re.IGNORECASE)
            match = pattern.match(result)
            if match:
                series_info = f"{abbr} {match.group(1)}"
                result = pattern.sub("", result)
                return series_info, result.strip()

        # Pattern 2: "Series Name - Author - Title" (no volume number)
        for series_name, _ in SERIES_MAPPINGS:
            pattern = re.compile(rf'^{re.escape(series_name)}\s*-\s*', re.IGNORECASE)
            if pattern.match(result):
                result = pattern.sub("", result)
                return None, result.strip()

        # Pattern 3: "(Series Name Volume) Author - Title"
        re_paren_series = re.compile(r'^\s*\(([^)]+?)\s+(\d+)\)\s*')
        match = re_paren_series.match(result)
        if match:
            series_part = match.group(1)
            volume_part = match.group(2)

            for series_name, abbr in SERIES_MAPPINGS:
                if series_name.lower() in series_part.lower():
                    series_info = f"{abbr} {volume_part}"
                    result = re_paren_series.sub("", result)
                    return series_info, result.strip()

        # Pattern 4: "[Series Name Volume]" in brackets
        re_bracket_series = re.compile(r'\s*\[([^\]]+?)\s+(\d+)\]')
        match = re_bracket_series.search(result)
        if match:
            series_part = match.group(1)
            volume_part = match.group(2)

            for series_name, abbr in SERIES_MAPPINGS:
                if series_name.lower() in series_part.lower():
                    series_info = f"{abbr} {volume_part}"
                    result = re_bracket_series.sub("", result)
                    return series_info, result.strip()

        # Fallback: Generic pattern (Series Name) Author - Title without volume
        re_generic = re.compile(r'^\s*\(([^)]+)\)\s+(.+)$')
        match = re_generic.match(result)
        if match:
            rest_part = match.group(2)

            # Check if 'rest_part' starts with an author
            re_sep = re.compile(r'(?:--|[-:])')
            sep_match = re_sep.search(rest_part)
            potential_author = rest_part
            if sep_match:
                potential_author = rest_part[:sep_match.start()]

            if self._is_likely_author(potential_author):
                return None, rest_part.strip()

        return None, result.strip()

    def _remove_duplicate_markers(self, s: str) -> str:
        """Remove duplicate markers like -2, (1), etc."""
        # (1), (2) at end
        s = self.DUP_MARKER_END_REGEX.sub("", s)
        # -2, -3 at end
        s = self.DUP_MARKER_DASH_END_REGEX.sub("", s)
        # -2 before (year)
        s = self.DUP_MARKER_BEFORE_YEAR_REGEX.sub(" (", s)
        return s

    def _extract_edition(self, s: str) -> Tuple[Optional[str], str]:
        """Extract edition information and return (edition_info, remaining_string)."""
        result = s

        for pattern in self.EDITION_PATTERNS:
            match = pattern.search(result)
            if match:
                num_str = match.group(1)
                suffix = self._get_ordinal_suffix(num_str)
                edition_info = f"{num_str}{suffix} ed"
                result = pattern.sub("", result)
                return edition_info, result.strip()

        return None, result.strip()

    def _get_ordinal_suffix(self, num_str: str) -> str:
        """Return the ordinal suffix for a number."""
        if num_str == "1":
            return "st"
        elif num_str == "2":
            return "nd"
        elif num_str == "3":
            return "rd"
        else:
            return "th"

    def _extract_volume(self, s: str) -> Tuple[Optional[str], str]:
        """Extract volume information and return (volume_info, normalized_string)."""
        for pattern, already_normalized in self.VOLUME_PATTERNS:
            match = pattern.search(s)
            if match:
                volume_info = f"Vol {match.group(1)}"
                if not already_normalized:
                    # Replace "Volume N" or "Part N" with "Vol N"
                    normalized_text = pattern.sub(volume_info, s)
                else:
                    normalized_text = s
                return volume_info, normalized_text

        return None, s

    def _clean_noise_sources(self, s: str) -> str:
        patterns = [
            # Z-Library variants
            r'\s*[-\(]?\s*[zZ]-?Library(?:\.pdf)?\s*[)\.]?',
            r'\s*\([zZ]-?Library(?:\.pdf)?\)',
            r'\s*-\s*[zZ]-?Library(?:\.pdf)?',
            # libgen variants
            r'\s*[-\(]?\s*libgen(?:\.li)?(?:\.pdf)?\s*[)\.]?',
            r'\s*\(libgen(?:\.li)?(?:\.pdf)?\)',
            r'\s*-\s*libgen(?:\.li)?(?:\.pdf)?',
            # Anna's Archive variants
            r"Anna'?s?\s*Archive",
            r"\s*[-\(]?\s*Anna'?s?\s+Archive\s*[)\.]?",
            r"\s*\(Anna'?s?\s+Archive\)",
            r"\s*-\s*Anna'?s?\s+Archive",
            # Hash patterns (MD5/SHA hashes)
            r'\s*--\s*[a-f0-9]{32}\s*(?:--)?',
            # ISBN-like patterns (10-13 digits)
            r'\s*--\s*\d{10,13}\s*(?:--)?',
            # Long alphanumeric IDs (16+ chars)
            r'\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?',
            # Shorter hash patterns (8+ hex chars)
            r'\s*--\s*[a-f0-9]{8,}\s*(?:--)?',
            # "Uploaded by"
            r'\s*[-\(]?\s*[Uu]ploaded by\s+[^)\-]+[)\.]?',
            r'\s*-\s*[Uu]ploaded by\s+[^)\-]+',
            # "Via ..."
            r'\s*[-\(]?\s*[Vv]ia\s+[^)\-]+[)\.]?',
            # Website URLs
            r'\s*[-\(]?\s*w{3}\.[a-zA-Z0-9-]+\.[a-z]{2,}\s*[)\.]?',
            r'\s*[-\(]?\s*[a-zA-Z0-9-]+\.(?:com|org|net|edu|io)\s*[)\.]?',
        ]
        result = s
        # Apply patterns multiple times to handle consecutive patterns
        for _ in range(3):
            before = result
            for pattern in patterns:
                result = re.sub(pattern, "", result)
            if result == before:
                break
        return result.strip()

    def _extract_year(self, s: str) -> Optional[int]:
        """Extract the last year found in the string."""
        matches = self.YEAR_REGEX.findall(s)
        if not matches:
            return None
        return int(matches[-1])

    def _clean_parentheticals(self, s: str, year: Optional[int]) -> str:
        result = s
        
        # Pattern 1: Remove (YYYY, Publisher) or (YYYY)
        if year is not None:
            pattern = re.compile(r'\s*\(\s*{}\s*(?:,\s*[^)]+)?\s*\)'.format(year))
            result = pattern.sub("", result)
            
        # Pattern 2: Remove nested parentheticals with publisher keywords
        while True:
            changed = False
            def replace_nested(match):
                nonlocal changed
                content = match.group(0)
                if self._is_publisher_or_series_info(content):
                    changed = True
                    return ""
                return content
            
            new_result = self.NESTED_PAREN_REGEX.sub(replace_nested, result)
            if not changed:
                break
            result = new_result
            
        # Pattern 3: Remove simple parentheticals with publisher keywords
        def replace_simple(match):
            content = match.group(0)
            if self._is_publisher_or_series_info(content):
                return ""
            return content
            
        result = self.SIMPLE_PAREN_REGEX.sub(replace_simple, result)
        result = self.SPACE_REGEX.sub(" ", result)
        return result.strip()

    def _smart_parse_author_title(self, s: str) -> Tuple[Optional[str], str]:
        s = s.strip()
        
        # Pattern 1: "Title (Author)"
        match = self.TRAILING_AUTHOR_REGEX.match(s)
        if match:
            title_part = match.group(1)
            author_part = match.group(2)
            if self._is_likely_author(author_part) and not self._is_publisher_or_series_info("("+author_part+")"):
                return self._clean_author_name(author_part), self._clean_title(title_part)
                
        # Pattern 2: "Author - Title" or "Author: Title" or "Author -- Title"
        match = self.SEPARATOR_REGEX.match(s)
        if match:
            author_part = match.group(1)
            title_part = match.group(2)
            if self._is_likely_author(author_part) and title_part:
                return self._clean_author_name(author_part), self._clean_title(title_part)
                
        # Pattern 3: Multiple authors separated by commas, then dash
        match = self.MULTI_AUTHOR_REGEX.match(s)
        if match:
            author1 = match.group(1)
            author2 = match.group(2)
            title_part = match.group(3)
            if self._is_likely_author(author1) and self._is_likely_author(author2):
                authors = f"{self._clean_author_name(author1)}, {self._clean_author_name(author2)}"
                return authors, self._clean_title(title_part)

        # Pattern 4: "Title; Author" (semicolon separator, author at end)
        match = self.SEMICOLON_REGEX.match(s)
        if match:
            title_part = match.group(1)
            author_part = match.group(2)
            if self._is_likely_author(author_part) and not self._is_publisher_or_series_info(author_part):
                return self._clean_author_name(author_part), self._clean_title(title_part)
                
        # Pattern 5: No clear author, treat as title only
        return None, self._clean_title(s)

    def _is_likely_author(self, s: str) -> bool:
        s = s.strip()
        if len(s) < 2:
            return False
            
        non_author_keywords = [
            "auth.", "translator", "translated by", "z-library", "libgen", "anna's archive", "2-library",
        ]
        s_lower = s.lower()
        for k in non_author_keywords:
            if k in s_lower:
                return False
                
        # Check if digits only
        if all(c.isdigit() or c in '-_' for c in s):
            return False
            
        # Check if name-like (uppercase Latin OR non-Latin letter)
        has_uppercase = any(c.isupper() for c in s)
        # Basic check for non-ASCII letters (covers CJK, etc.)
        has_non_latin = any(ord(c) > 127 and c.isalpha() for c in s)
        
        return has_uppercase or has_non_latin

    def _clean_author_name(self, s: str) -> str:
        s = s.strip()
        s = self.AUTH_REGEX.sub("", s)
        
        comma_count = s.count(",")
        if comma_count == 1:
            parts = s.split(", ")
            if len(parts) == 2:
                before = parts[0].strip()
                after = parts[1].strip()
                if len(before.split()) == 1 and len(after.split()) == 1:
                    s = f"{before} {after}"
        
        s = self.SPACE_REGEX.sub(" ", s)
        return s.strip()

    def _clean_title(self, s: str) -> str:
        s = s.strip()
        s = self._clean_noise_sources(s)
        s = self.AUTH_REGEX.sub("", s)
        s = self.TRAILING_ID_REGEX.sub("", s)

        # Remove trailing publisher info separated by dash
        # e.g. "Title - Publisher"
        idx = s.rfind(" - ")
        if idx != -1:
            suffix = s[idx+3:]
            if self._is_publisher_or_series_info(suffix):
                s = s[:idx]

        # Also handle just "-" without spaces if it looks like publisher
        idx = s.rfind("-")
        if idx != -1 and idx > 0 and idx < len(s) - 1:
            suffix = s[idx+1:].strip()
            # Use stricter check for non-spaced dash to avoid stripping parts of title
            if self._is_strict_publisher_info(suffix):
                s = s[:idx]

        s = self._clean_orphaned_brackets(s)
        s = self.SPACE_REGEX.sub(" ", s)
        s = s.strip("-:;,.")
        return s.strip()

    def _is_publisher_or_series_info(self, s: str) -> bool:
        publisher_keywords = [
            "Press", "Publishing", "Academic Press", "Springer", "Cambridge", "Oxford", "MIT Press",
            "Series", "Textbook Series", "Graduate Texts", "Graduate Studies", "Lecture Notes",
            "Pure and Applied", "Mathematics", "Foundations of", "Monographs", "Studies", "Collection",
            "Textbook", "Edition", "Vol.", "Volume", "No.", "Part", "理工", "出版社", "の",
            "Z-Library", "libgen", "Anna's Archive",
        ]

        for k in publisher_keywords:
            if k in s:
                return True

        # Detect hash patterns: 8+ hex chars or 16+ alphanumeric
        if re.search(r'[a-f0-9]{8,}', s) and len(s) > 8:
            return True
        if re.search(r'[A-Za-z0-9]{16,}', s) and len(s) > 16:
            return True

        # Check for series info (mostly non-letters with numbers)
        has_numbers = any(c.isdigit() for c in s)
        non_letter_count = sum(1 for c in s if not c.isalpha() and c != ' ')

        if has_numbers and non_letter_count > 2:
            return True

        return False

    def _is_strict_publisher_info(self, s: str) -> bool:
        """Stricter version for suffix stripping (no parens)."""
        strict_keywords = [
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
        ]

        for keyword in strict_keywords:
            if keyword in s:
                return True
        return False

    def _clean_orphaned_brackets(self, s: str) -> str:
        result = []
        open_parens = 0
        open_brackets = 0
        
        for char in s:
            if char == '(':
                open_parens += 1
                result.append(char)
            elif char == ')':
                if open_parens > 0:
                    open_parens -= 1
                    result.append(char)
            elif char == '[':
                open_brackets += 1
                result.append(char)
            elif char == ']':
                if open_brackets > 0:
                    open_brackets -= 1
                    result.append(char)
            elif char == '_':
                result.append(' ')
            else:
                result.append(char)
        
        result_str = ''.join(result)
        while result_str.endswith('(') or result_str.endswith('['):
            result_str = result_str[:-1]
            
        return result_str

    def _generate_new_filename(self, metadata: ParsedMetadata, extension: str) -> str:
        parts = []
        
        # Author(s)
        if metadata.authors:
            parts.append(f"{metadata.authors} - ")
        
        # Title (volume is kept in title if present)
        parts.append(metadata.title)
        
        # Series info in brackets
        if metadata.series:
            parts.append(f" [{metadata.series}]")
        
        # Year and Edition in parentheses
        if metadata.year is not None and metadata.edition:
            parts.append(f" ({metadata.year}, {metadata.edition})")
        elif metadata.year is not None:
            parts.append(f" ({metadata.year})")
        elif metadata.edition:
            parts.append(f" ({metadata.edition})")
        
        parts.append(extension)
        return "".join(parts)
