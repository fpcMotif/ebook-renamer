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
    SEPARATOR_REGEX = re.compile(r'^(.+?)\s*[-:]\s+(.+)$')
    MULTI_AUTHOR_REGEX = re.compile(r'^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*[-:]\s+(.+)$')
    SEMICOLON_REGEX = re.compile(r'^(.+?)\s*;\s*(.+)$')
    
    # Cleaning patterns
    AUTH_NOISE_REGEX = re.compile(r'\s*\((?:[Aa]uth\.?|[Aa]uthor|[Ee]ds?\.?|[Tt]ranslator)\)')
    TRAILING_AUTH_REGEX = re.compile(r'\s*\([Aa]uth\.?\)')
    EMPTY_PAREN_REGEX = re.compile(r'\(\s*\)')

    # New Pattern Regexes
    VERSION_REGEX = re.compile(r'(?i)\b(v|ver|version)\.?\s*\d+(\.\d+)*\b')
    PAGES_REGEX = re.compile(r'(?i)\b\d+\s*(?:pages?|pp?\.?|p)\b')
    LANG_EDITION_REGEX = re.compile(r'(?i)\b(English|Chinese|Japanese)\s+Edition\b')

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
        base = self._clean_noise_sources(base)
        
        # Step 5: Remove duplicate markers
        base = self._remove_duplicate_markers(base)
        
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
        return result.strip()

    def _clean_noise_sources(self, s: str) -> str:
        patterns = [
            # Precise matches (Improved)
            r'\s+libgen\.li$',             # Ends with libgen.li
            r'\s*[-\(]?\s*[zZ]-?Library$', # Ends with Z-Library

            r'\s*[-\(]?\s*[zZ]-?Library\s*[)\.]?',
            r'\s*\([zZ]-?Library\)',
            r'\s*-\s*[zZ]-?Library',
            # libgen variants
            r'\s*[-\(]?\s*libgen(?:\.li)?\s*[)\.]?',
            r'\s*\(libgen(?:\.li)?\)',
            r'\s*-\s*libgen(?:\.li)?',
            # Anna's Archive variants
            r'Anna\'?s?\s*Archive',
            r'\s*[-\(]?\s*Anna\'?s?\s+Archive\s*[)\.]?',
            r'\s*\(Anna\'?s?\s+Archive\)',
            r'\s*-\s*Anna\'?s?\s+Archive',
            # Hash patterns
            r'\s*--\s*[a-f0-9]{32}\s*(?:--)?',
            r'\s*--\s*\d{10,13}\s*(?:--)?',
            r'\s*--\s*[A-Za-z0-9]{16,}\s*(?:--)?',
            r'\s*--\s*[a-f0-9]{8,}\s*(?:--)?',
        ]
        result = s
        for i in range(3):
            before = result
            for pattern in patterns:
                result = re.sub(pattern, "", result)
            if result == before:
                break
        return result.strip()

    def _remove_duplicate_markers(self, s: str) -> str:
        # (1), (2) at end
        s = re.sub(r'[-\s]*\(\d{1,2}\)\s*$', '', s)
        # -2, -3 at end
        s = re.sub(r'-\d{1,2}\s*$', '', s)
        # -2 before (year)
        s = re.sub(r'-\d{1,2}\s+\(', ' (', s)
        return s

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
        s = self.AUTH_NOISE_REGEX.sub("", s)
        
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

    def _remove_common_patterns(self, s: str) -> str:
        s = self.VERSION_REGEX.sub("", s)
        s = self.PAGES_REGEX.sub("", s)
        s = self.LANG_EDITION_REGEX.sub("", s)
        return s

    def _clean_title(self, s: str) -> str:
        s = s.strip()

        # Clean noise sources
        s = self._clean_noise_sources(s)

        # Remove (auth.)
        s = self.TRAILING_AUTH_REGEX.sub("", s)

        # Strip trailing ID-like noise
        s = self.TRAILING_ID_REGEX.sub("", s)

        # Remove common patterns
        s = self._remove_common_patterns(s)

        # Clean orphaned brackets
        s = self._clean_orphaned_brackets(s)

        # Remove empty parens
        s = self.EMPTY_PAREN_REGEX.sub("", s)

        s = self.SPACE_REGEX.sub(" ", s)
        s = s.strip("-:;,.")
        return s.strip()

    def _is_publisher_or_series_info(self, s: str) -> bool:
        s_lower = s.lower()

        publisher_keywords = [
            # Publishers
            "press", "publishing", "academic press", "springer", "cambridge", "oxford", "mit press",
            "wiley", "pearson", "mcgraw-hill", "elsevier", "taylor & francis",
            # General Types
            "fiction", "novel", "handbook", "manual", "guide", "reference",
            "cookbook", "workbook", "encyclopedia", "dictionary", "atlas", "anthology",
            "biography", "memoir", "essay", "poetry", "drama", "short stories",
            # Academic Types
            "thesis", "dissertation", "proceedings", "conference", "symposium", "workshop",
            "report", "technical report", "white paper", "preprint", "manuscript",
            "lecture", "course notes", "study guide", "solutions manual",
            # Series/Editions
            "series", "textbook series", "graduate texts", "graduate studies", "lecture notes",
            "pure and applied", "mathematics", "foundations of", "monographs", "studies", "collection",
            "textbook", "edition", "vol.", "volume", "no.", "part",
            "revised edition", "updated edition", "expanded edition",
            "abridged", "unabridged", "complete edition", "anniversary edition",
            "collector's edition", "special edition", "1st ed", "2nd ed", "3rd ed",
            # Format/Quality
            "ocr", "scanned", "retail", "searchable", "bookmarked", "optimized",
            "compressed", "high quality", "hq", "drm-free", "no drm", "cracked",
            "kindle edition", "pdf version", "epub version", "mobi version",
            # Chinese
            "理工", "出版社", "小说", "教材", "教程", "手册", "指南", "参考书", "文集", "论文集",
            "丛书", "系列", "修订版", "第二版", "第三版", "增订版",
            # Japanese
            "の", "小説", "教科書", "テキスト", "ハンドブック", "マニュアル", "ガイド",
            "講義", "シリーズ", "改訂版", "第2版", "第3版",
            # Noise
            "z-library", "libgen", "anna's archive",
            # Languages
            "english", "chinese", "japanese",
        ]
        
        for k in publisher_keywords:
            if k in s_lower:
                return True

        # Check regex patterns
        if self.VERSION_REGEX.search(s):
            return True
        if self.PAGES_REGEX.search(s):
            return True

        # Detect hash patterns
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
