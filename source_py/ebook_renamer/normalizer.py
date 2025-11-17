"""
Filename normalization functionality for the ebook renamer.
"""

import re
from typing import List, Optional

from .types import FileInfo, ParsedMetadata


class Normalizer:
    """Handles filename normalization according to the specification."""
    
    # Series prefixes to remove
    SERIES_PREFIXES = [
        "London Mathematical Society Lecture Note Series",
        "Graduate Texts in Mathematics",
        "Progress in Mathematics",
        "[Springer-Lehrbuch]",
        "[Graduate studies in mathematics",
        "[Progress in Mathematics №",
        "[AMS Mathematical Surveys and Monographs",
    ]
    
    # Source indicators to remove
    SOURCE_INDICATORS = [
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
    ]
    
    # Non-author keywords to filter out
    NON_AUTHOR_KEYWORDS = [
        "auth.",
        "translator",
        "translated by",
        "Z-Library",
        "libgen",
        "Anna's Archive",
        "2-Library",
    ]
    
    # Regex patterns
    YEAR_REGEX = re.compile(r'\b(19|20)\d{2}\b')
    YEAR_WITH_PUB_REGEX = re.compile(r'\s*\(\s*(19|20)\d{2}\s*(?:,\s*[^)]+)?\s*\)')
    YEAR_COMMA_REGEX = re.compile(r'\s*(19|20)\d{2}\s*,\s*[^,]+$')
    AUTH_REGEX = re.compile(r'\s*\([Aa]uth\.?\).*')
    SPACE_REGEX = re.compile(r'\s+')
    
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
            file.new_path = f"{self._get_dir_name(file.original_path)}/{new_name}"
            result.append(file)
        
        return result
    
    def _parse_filename(self, filename: str, extension: str) -> ParsedMetadata:
        """Parse a filename into metadata components."""
        # Remove extension and any .download suffix
        base = filename
        if base.endswith(".download"):
            base = base[:-len(".download")]
        if base.endswith(extension):
            base = base[:-len(extension)]
        base = base.strip()
        
        # Clean up obvious noise/series prefixes
        base = self._strip_prefix_noise(base)
        
        # Clean source indicators BEFORE parsing authors and titles
        base = self._clean_source_indicators(base)
        
        # Extract year (4 digits: 19xx or 20xx)
        year = self._extract_year(base)
        
        # Remove year and surrounding brackets/parens from base for further processing
        base_without_year = self._remove_year_from_string(base)
        
        # Try to split authors and title by common separators
        authors, title = self._split_authors_and_title(base_without_year)
        
        return ParsedMetadata(
            authors=authors,
            title=title,
            year=year,
        )
    
    def _strip_prefix_noise(self, s: str) -> str:
        """Remove series prefixes."""
        for prefix in self.SERIES_PREFIXES:
            if s.startswith(prefix):
                s = s[len(prefix):]
                # Remove leading spaces or dashes
                s = s.lstrip(" -")
                break
        return s
    
    def _clean_source_indicators(self, s: str) -> str:
        """Remove source indicators."""
        for pattern in self.SOURCE_INDICATORS:
            if s.endswith(pattern):
                s = s[:-len(pattern)]
        return s.strip()
    
    def _extract_year(self, s: str) -> Optional[int]:
        """Extract the last year found in the string."""
        matches = self.YEAR_REGEX.findall(s)
        if not matches:
            return None
        
        # Return the last year found (usually most relevant)
        last_match = matches[-1]
        try:
            return int(last_match)
        except ValueError:
            return None
    
    def _remove_year_from_string(self, s: str) -> str:
        """Remove year patterns from the string."""
        # Remove year patterns but keep the rest of the string
        # Pattern: (YYYY, Publisher) or (YYYY) or YYYY,
        result = self.YEAR_WITH_PUB_REGEX.sub("", s)
        result = self.YEAR_COMMA_REGEX.sub("", result)
        return result
    
    def _split_authors_and_title(self, s: str) -> tuple[Optional[str], str]:
        """Split authors and title from the cleaned string."""
        # Check for trailing (author) pattern
        last_open_paren = s.rfind("(")
        if last_open_paren != -1 and s.endswith(")"):
            potential_author = s[last_open_paren+1:-1].strip()
            if self._is_likely_author(potential_author):
                title = s[:last_open_paren].strip()
                return potential_author, title
        
        # Check for " - " separator (most common)
        last_dash = s.rfind(" - ")
        if last_dash != -1:
            maybe_author = s[:last_dash].strip()
            maybe_title = s[last_dash+3:].strip()
            
            if self._is_likely_author(maybe_author) and maybe_title != "":
                clean_author = self._clean_author_name(maybe_author)
                clean_title = self._clean_title(maybe_title)
                return clean_author, clean_title
        
        # Check for ":" separator
        colon_index = s.find(":")
        if colon_index != -1:
            maybe_author = s[:colon_index].strip()
            maybe_title = s[colon_index+1:].strip()
            
            if self._is_likely_author(maybe_author) and maybe_title != "":
                clean_author = self._clean_author_name(maybe_author)
                clean_title = self._clean_title(maybe_title)
                return clean_author, clean_title
        
        # If no clear separator, treat entire string as title
        return None, self._clean_title(s)
    
    def _is_likely_author(self, s: str) -> bool:
        """Determine if a string is likely an author name."""
        s = s.strip()
        
        # Too short to be an author
        if len(s) < 2:
            return False
        
        # Filter out obvious non-author phrases
        s_lower = s.lower()
        for keyword in self.NON_AUTHOR_KEYWORDS:
            if keyword in s_lower:
                return False
        
        # Check if looks like a name (has at least one uppercase letter)
        for char in s:
            if char.isupper():
                return True
        
        return False
    
    def _clean_author_name(self, s: str) -> str:
        """Clean up author name."""
        s = s.strip()
        
        # Remove trailing (auth.) etc.
        s = self.AUTH_REGEX.sub("", s)
        
        return s.strip()
    
    def _clean_title(self, s: str) -> str:
        """Clean up title."""
        s = s.strip()
        
        # Remove trailing source markers
        for pattern in self.SOURCE_INDICATORS:
            if s.endswith(pattern):
                s = s[:-len(pattern)]
        
        # Remove trailing .download suffix
        while s.endswith(".download"):
            s = s[:-len(".download")]
        
        # Remove (auth.) and similar patterns
        s = self.AUTH_REGEX.sub("", s)
        
        # Clean up orphaned brackets/parens
        s = self._clean_orphaned_brackets(s)
        
        # Remove multiple spaces
        s = self.SPACE_REGEX.sub(" ", s)
        
        # Remove leading/trailing punctuation
        s = s.rstrip("-:;,")
        s = s.lstrip("-:;,")
        
        return s.strip()
    
    def _clean_orphaned_brackets(self, s: str) -> str:
        """Remove orphaned brackets and replace underscores."""
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
                result.append(' ')  # Replace underscores with spaces
            else:
                result.append(char)
        
        # Remove trailing orphaned opening brackets
        result_str = ''.join(result)
        while result_str.endswith('(') or result_str.endswith('['):
            result_str = result_str[:-1]
        
        return result_str
    
    def _generate_new_filename(self, metadata: ParsedMetadata, extension: str) -> str:
        """Generate the new filename from metadata."""
        parts = []
        
        if metadata.authors:
            parts.append(metadata.authors)
        
        parts.append(metadata.title)
        
        if metadata.year is not None:
            parts.append(f"({metadata.year})")
        
        parts.append(extension)
        
        return ''.join(parts)
    
    def _get_dir_name(self, path: str) -> str:
        """Get directory name from path."""
        return path.rsplit('/', 1)[0] if '/' in path else '.'
