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
    SEMICOLON_REGEX = re.compile(r'^(.+?)\s*;\s*(.+)$')
    
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
        
        # Step 3: Remove ALL bracketed annotations
        base = self.BRACKET_REGEX.sub("", base)

        # Step 4: Clean noise sources
        # MUST happen BEFORE author parsing
        base = self._clean_noise_sources(base)
        
        # Step 5: Remove duplicate markers
        base = re.sub(r'[_\-\s]*\(\d{1,2}\)\s*$', '', base)  # (1), (2) at end
        base = re.sub(r'-\d{1,2}\s*$', '', base)  # -2, -3 at end
        base = re.sub(r'-\d{1,2}\s+\(', ' (', base)  # -2 before (year)

        # Step 6: Extract year FIRST
        year = self._extract_year(base)
        
        # Step 7: Remove parentheticals
        base = self._clean_parentheticals(base, year)
        
        # Step 8: Parse author and title
        authors, title = self._smart_parse_author_title(base)
        
        return ParsedMetadata(
            authors=authors,
            title=title,
            year=year,
        )
    
    def _remove_series_prefixes(self, s: str) -> str:
        prefixes = [
            "London Mathematical Society Lecture Note Series",
            "Graduate Texts in Mathematics",
            "Progress in Mathematics",
            "[Springer-Lehrbuch]",
            "[Graduate studies in mathematics",
            "[Progress in Mathematics №",
            "[AMS Mathematical Surveys and Monographs",
        ]
        
        result = s
        for prefix in prefixes:
            if result.startswith(prefix):
                result = result[len(prefix):]
                result = result.lstrip("- ]")
                break

        # Generic pattern: (Series Name) Author - Title
        re_generic = re.compile(r"^\s*\(([^)]+)\)\s+(.+)$")
        match = re_generic.match(result)
        if match:
            rest_part = match.group(2)
            # Check if 'rest_part' starts with an author
            re_sep = re.compile(r"(?:--|[-:])")
            sep_match = re_sep.search(rest_part)
            if sep_match:
                potential_author = rest_part[:sep_match.start()]
                if self._is_likely_author(potential_author):
                    result = rest_part

        return result.strip()

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
            r'Anna\'?s?\s*Archive',
            r'\s*[-\(]?\s*Anna\'?s?\s+Archive(?:\.pdf)?\s*[)\.]?',
            r'\s*\(Anna\'?s?\s+Archive(?:\.pdf)?\)',
            r'\s*-\s*Anna\'?s?\s+Archive(?:\.pdf)?',
            # Hash patterns (32 hex chars)
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

        # Pattern 4: "Title; Author"
        match = self.SEMICOLON_REGEX.match(s)
        if match:
            title_part = match.group(1)
            author_part = match.group(2)
            if self._is_likely_author(author_part) and not self._is_publisher_or_series_info(author_part):
                return self._clean_author_name(author_part), self._clean_title(title_part)
                
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
        noise_patterns = [
            r"\s*\(auth\.?\)",
            r"\s*\(author\)",
            r"\s*\(eds?\.?\)",
            r"\s*\(translator\)",
        ]
        for pattern in noise_patterns:
            s = re.sub(pattern, "", s)

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

        # Hex patterns
        if re.search(r"[a-f0-9]{8,}", s) and len(s) > 8:
            return True
        if re.search(r"[A-Za-z0-9]{16,}", s) and len(s) > 16:
            return True
                
        # Check for series info (mostly non-letters with numbers)
        has_numbers = any(c.isdigit() for c in s)
        non_letter_count = sum(1 for c in s if not c.isalpha() and c != ' ')
        
        if has_numbers and non_letter_count > 2:
            return True
            
        return False

    def _is_strict_publisher_info(self, s: str) -> bool:
        strict_keywords = [
            "Press", "Publishing", "Springer", "Cambridge", "Oxford", "MIT", "Wiley", "Elsevier",
            "Routledge", "Pearson", "McGraw", "Addison", "Prentice", "O'Reilly", "Princeton",
            "Harvard", "Yale", "Stanford", "Chicago", "California", "Columbia", "University",
            "Verlag", "Birkhäuser", "CUP",
        ]
        for k in strict_keywords:
            if k in s:
                return True
        return False

    def _clean_title(self, s: str) -> str:
        s = s.strip()
        s = self._clean_noise_sources(s)
        s = re.sub(r"\s*\([Aa]uth\.?\)", "", s)

        # Strip trailing ID-like noise
        s = self.TRAILING_ID_REGEX.sub("", s)

        # Remove trailing publisher info separated by dash
        idx = s.rfind(" - ")
        if idx != -1:
            suffix = s[idx+3:]
            if self._is_publisher_or_series_info(suffix):
                s = s[:idx]

        # Handle just "-" without spaces
        idx = s.rfind('-')
        if idx > 0 and idx < len(s) - 1:
            suffix = s[idx+1:].strip()
            if self._is_strict_publisher_info(suffix):
                s = s[:idx]

        s = self._clean_orphaned_brackets(s)
        s = self.SPACE_REGEX.sub(" ", s)
        s = s.strip("-:;,.")
        return s.strip()

    def _clean_orphaned_brackets(self, s: str) -> str:
        result = []
        open_parens_indices = []
        open_brackets_indices = []

        # Convert to list for mutable operations
        chars = list(s)
        temp_result = []

        # First pass: build a string skipping orphaned closing brackets
        # But we need to track indices in the *resulting* string to remove unclosed openers later
        # This is tricky in Python compared to Rust because strings are immutable.
        # Let's do it in two passes or using a list.
        
        # Let's stick to the Rust implementation logic:
        # push to result, track indices of openers.

        for char in chars:
            if char == '(':
                open_parens_indices.append(len(temp_result))
                temp_result.append(char)
            elif char == ')':
                if open_parens_indices:
                    open_parens_indices.pop()
                    temp_result.append(char)
                else:
                    # Orphaned closing paren -> space
                    temp_result.append(' ')
            elif char == '[':
                open_brackets_indices.append(len(temp_result))
                temp_result.append(char)
            elif char == ']':
                if open_brackets_indices:
                    open_brackets_indices.pop()
                    temp_result.append(char)
                else:
                    # Orphaned closing bracket -> space
                    temp_result.append(' ')
            elif char == '_':
                temp_result.append(' ')
            else:
                temp_result.append(char)

        # Remove unclosed opening brackets
        indices_to_remove = sorted(open_parens_indices + open_brackets_indices, reverse=True)
        for idx in indices_to_remove:
            if idx < len(temp_result):
                del temp_result[idx]
        
        result_str = "".join(temp_result)
        result_str = self.SPACE_REGEX.sub(" ", result_str)
        return result_str.strip()

    def _generate_new_filename(self, metadata: ParsedMetadata, extension: str) -> str:
        parts = []
        if metadata.authors:
            parts.append(f"{metadata.authors} - ")
        parts.append(metadata.title)
        if metadata.year is not None:
            parts.append(f" ({metadata.year})")
        parts.append(extension)
        return "".join(parts)
