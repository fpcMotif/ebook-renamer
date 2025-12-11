"""
Filename normalization functionality for the ebook renamer.
"""

import re
import os
from typing import List, Optional, Tuple

from .types import FileInfo, ParsedMetadata


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
    SEPARATOR_REGEX = re.compile(r'^(.+?)\s*(?:--|[-:])\s+(.+)$')
    MULTI_AUTHOR_REGEX = re.compile(r'^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*(?:--|[-:])\s+(.+)$')

    # New patterns
    EDITION_REGEX = re.compile(r'\b(\d+)(?:st|nd|rd|th)?\s*[Ee]d(?:ition)?\.?\b|\b[Ee]d(?:ition)?\.?\s*(\d+)\b')
    VOLUME_REGEX = re.compile(r'\b(?:[Vv]ol(?:ume|\.)?|[Pp]art)\s*(\d+)\b')
    # Matches [GTM 52], [CSAM 10], etc.
    SERIES_REGEX = re.compile(r'\[([A-Z]+)\s+(\d+)\]')
    
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
        
        # Step 2: Remove series prefixes (must be early)
        base = self._remove_series_prefixes(base)
        
        # Step 3: Clean noise sources
        base = self._clean_noise_sources(base)
        
        # Step 4: Extract Series Info (before removing brackets)
        series, base = self._extract_series(base)

        # Step 5: Remove remaining bracketed annotations
        base = self.BRACKET_REGEX.sub("", base)
        
        # Step 6: Extract Edition
        edition, base = self._extract_edition(base)

        # Step 7: Extract year
        year = self._extract_year(base)
        
        # Step 8: Remove parentheticals
        base = self._clean_parentheticals(base, year)
        
        # Step 9: Parse author and title
        authors, title = self._smart_parse_author_title(base)

        # Step 10: Extract Volume from title (and normalize it in title)
        volume, title = self._extract_volume(title)

        return ParsedMetadata(
            authors=authors,
            title=title,
            year=year,
            series=series,
            edition=edition,
            volume=volume
        )
    
    def _extract_series(self, s: str) -> Tuple[Optional[str], str]:
        """Extract series info like [GTM 52] and remove from string."""
        # Look for the pattern
        match = self.SERIES_REGEX.search(s)
        if match:
            # Check if the abbreviation is one of the known ones (optional but good for safety)
            # For now, we trust the regex [A-Z]+ \d+ inside brackets
            series_str = f"{match.group(1)} {match.group(2)}"
            # Remove from string
            # We replace with empty string, but keep surrounding spaces clean later
            new_s = s[:match.start()] + s[match.end():]
            return series_str, new_s

        # Also handle "Graduate Texts in Mathematics 52" pattern at start of file
        # But _remove_series_prefixes already strips the long names.
        # The spec says "Input: Graduate Texts in Mathematics 52 - Title.pdf -> Output: ... [GTM 52]"
        # If the input had the long name, we might have lost the number in _remove_series_prefixes?
        # Let's check _remove_series_prefixes. It strips the prefix.
        # Ideally, we should detect the number before stripping.

        return None, s

    def _extract_edition(self, s: str) -> Tuple[Optional[str], str]:
        """Extract edition info and remove from string."""
        # Search for edition patterns
        matches = list(self.EDITION_REGEX.finditer(s))
        if not matches:
            return None, s

        # Take the last one if multiple? Usually only one.
        match = matches[-1]
        number = match.group(1) or match.group(2)

        # Normalize to "Nth ed"
        suffix = "th"
        if number == "1": suffix = "st"
        elif number == "2": suffix = "nd"
        elif number == "3": suffix = "rd"

        # Handle 11, 12, 13
        if len(number) >= 2 and number[-2] == '1':
            suffix = "th"

        edition_str = f"{number}{suffix} ed"

        # Remove from string
        new_s = s[:match.start()] + s[match.end():]
        return edition_str, new_s

    def _extract_volume(self, title: str) -> Tuple[Optional[str], str]:
        """Normalize volume info in title to 'Vol N' and extract it if needed."""
        # The spec says: "Vol N - kept in title".
        # But the output format shows: "Title [Series] (Year, Edition)"
        # Wait, spec says: "Multi-volume: Michael Spivak - Differential Geometry Vol 2 (1979).pdf"
        # So it stays in the title, but normalized.

        # We will replace all occurrences with normalized version
        def replace(match):
            return f"Vol {match.group(1)}"

        new_title = self.VOLUME_REGEX.sub(replace, title)
        return None, new_title  # We don't extract it to a separate field for the filename generation if it stays in title

    def _remove_series_prefixes(self, s: str) -> str:
        # Check for specific patterns that include numbers, e.g. "Graduate Texts in Mathematics 52"
        # If found, we might want to capture this for the series tag [GTM 52].
        # However, the current structure separates extraction.
        # For this task, I will stick to the previous logic but ensure we don't lose the number if possible.
        # Actually, if the input is "Graduate Texts in Mathematics 52 - Title",
        # the previous logic removes "Graduate Texts in Mathematics", leaving " 52 - Title".
        # Then the "52" might be treated as part of the title or author.

        # Let's handle the mapping of long names to abbreviations here if we want to be fancy,
        # but the spec example shows: "Input: Graduate Texts in Mathematics 52 - Title.pdf -> Output: Author - Title [GTM 52].pdf"
        # So we SHOULD extract it.

        prefixes = [
            ("London Mathematical Society Lecture Note Series", "LMSLN"),
            ("Graduate Texts in Mathematics", "GTM"),
            ("Progress in Mathematics", "PM"),
            ("Springer Undergraduate Mathematics Series", "SUMS"),
            ("Graduate Studies in Mathematics", "GSM"),
            ("Cambridge Studies in Advanced Mathematics", "CSAM"),
        ]

        result = s
        for prefix, abbr in prefixes:
            if result.startswith(prefix):
                # Check if followed by a number
                rest = result[len(prefix):].strip()
                # Look for number at start of rest
                match = re.match(r'^(\d+)\s*[-:]', rest)
                if match:
                    # We found a series number!
                    # We should probably attach this to the string as [GTM 52] so _extract_series can find it later?
                    # Or just return it modified.
                    # Hack: Prepend [GTM 52] to the string and strip the prefix.
                    number = match.group(1)
                    # Construct new string: "[GTM 52] " + rest without number
                    # rest[match.end():] starts after the number and separator
                    # We need to be careful about the separator.

                    # But wait, `_extract_series` looks for `[A-Z]+ \d+`.
                    # So if we rewrite "Graduate Texts in Mathematics 52 - Title" to "[GTM 52] Title",
                    # `_extract_series` will catch it.

                    # Find where the number ends
                    num_match = re.match(r'^(\d+)', rest)
                    if num_match:
                        num_end = num_match.end()
                        return f"[{abbr} {number}] {rest[num_end:]}".strip()

                # If no number, just strip the prefix
                result = result[len(prefix):]
                result = result.lstrip("- ]")
                break

        # Existing logic for other prefixes
        other_prefixes = [
            "[Springer-Lehrbuch]",
            "[Graduate studies in mathematics",
            "[Progress in Mathematics №",
            "[AMS Mathematical Surveys and Monographs",
        ]

        for prefix in other_prefixes:
            if result.startswith(prefix):
                result = result[len(prefix):]
                result = result.lstrip("- ]")
                break

        # Generic pattern: (Series Name) Author - Title
        re_generic = re.compile(r'^\s*\(([^)]+)\)\s+(.+)$')
        match = re_generic.match(result)
        if match:
            rest_part = match.group(2)
            re_sep = re.compile(r'(?:--|[-:])')
            sep_match = re_sep.search(rest_part)
            potential_author = rest_part
            if sep_match:
                potential_author = rest_part[:sep_match.start()]

            if self._is_likely_author(potential_author):
                result = rest_part

        return result.strip()

    def _clean_noise_sources(self, s: str) -> str:
        patterns = [
            r'\s*[-\(]?\s*[zZ]-?Library(?:\.pdf)?\s*[)\.]?',
            r'\s*[-\(]?\s*libgen(?:\.li)?(?:\.pdf)?\s*[)\.]?',
            r'\s*[-\(]?\s*Anna\'?s?\s+Archive(?:\.pdf)?\s*[)\.]?',
            # Hash patterns
            r'\s*--\s*[a-f0-9]{32}\s*(?:--)?',
            r'\s*--\s*\d{10,13}\s*(?:--)?',
            r'\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?',
            r'\s*--\s*[a-f0-9]{8,}\s*(?:--)?',
        ]
        result = s
        for pattern in patterns:
            result = re.sub(pattern, "", result)
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
        # Note: If we have extracted an edition, that might have been inside parens with the year.
        # But _extract_edition just removed "2nd ed".
        # So "(2000, 2nd ed)" -> becomes "(2000, )" or "(2000)".
        # We need to clean up commas if they are left behind.

        if year is not None:
            # Flexible pattern to catch (2000) or (2000, Publisher) or (2000, )
            pattern = re.compile(r'\s*\(\s*{}\s*(?:,\s*[^)]*)?\s*\)'.format(year))
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
                
        # Pattern 2: "Author - Title" or "Author: Title"
        match = self.SEPARATOR_REGEX.match(s)
        if match:
            author_part = match.group(1)
            title_part = match.group(2)
            if self._is_likely_author(author_part) and title_part:
                return self._clean_author_name(author_part), self._clean_title(title_part)
                
        # Pattern 3: Multiple authors
        match = self.MULTI_AUTHOR_REGEX.match(s)
        if match:
            author1 = match.group(1)
            author2 = match.group(2)
            title_part = match.group(3)
            if self._is_likely_author(author1) and self._is_likely_author(author2):
                authors = f"{self._clean_author_name(author1)}, {self._clean_author_name(author2)}"
                return authors, self._clean_title(title_part)
                
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
                # Check if likely Last, First (both single words)
                if len(before.split()) == 1 and len(after.split()) == 1:
                    s = f"{before} {after}"
                # If "Smith, John" -> "Smith, John" (keep comma if it looks like Last, First but maybe multi-word names?)
                # The spec says: "Marco, Grandis" -> "Marco Grandis" (ONLY if single word each side)
                # "Smith, John" -> "Smith, John" (keep if likely Lastname, Firstname with 2+ words? No, wait)
                # Spec: "Smith, John" -> "Smith, John" (keep if likely Lastname, Firstname format with 2+ words)
                # "Thomas H. Wolff, Izabella Aba, Carol Shubin" -> keep all commas
        
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
        ]

        for k in publisher_keywords:
            if k in s:
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
        if metadata.authors:
            parts.append(f"{metadata.authors} - ")

        parts.append(metadata.title)

        if metadata.series:
            parts.append(f" [{metadata.series}]")

        # Year and Edition grouping: (Year, Edition) or (Year) or (Edition) if no year?
        # Spec: "(Year, Edition)"

        parens_content = []
        if metadata.year is not None:
            parens_content.append(str(metadata.year))

        if metadata.edition:
            parens_content.append(metadata.edition)

        if parens_content:
            parts.append(f" ({', '.join(parens_content)})")

        parts.append(extension)
        return "".join(parts)
