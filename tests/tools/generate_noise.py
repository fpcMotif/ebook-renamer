#!/usr/bin/env python3
"""
Generate noise variations from clean ebook files.
This script creates realistic test data by adding various noise patterns
to clean filenames, simulating real-world download scenarios.
"""

import argparse
import random
import shutil
from pathlib import Path
from typing import List


class NoiseGenerator:
    """Generate noise variations for ebook filenames."""
    
    # Series prefixes to add
    SERIES_PREFIXES = [
        "London Mathematical Society Lecture Note Series",
        "Graduate Texts in Mathematics",
        "Progress in Mathematics",
        "[Springer-Lehrbuch]",
        "[Graduate studies in mathematics",
        "[Progress in Mathematics №",
        "[AMS Mathematical Surveys and Monographs",
    ]
    
    # Source indicators to add
    SOURCE_SUFFIXES = [
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
    
    # Publishers to add with years
    PUBLISHERS = [
        "Springer",
        "Cambridge University Press",
        "Oxford University Press",
        "MIT Press",
        "Wiley",
        "Birkhäuser",
        "AMS",
        "Elsevier",
    ]
    
    def __init__(self, random_seed: int = None):
        if random_seed is not None:
            random.seed(random_seed)
    
    def parse_clean_filename(self, filename: str) -> dict:
        """Parse a clean filename into components."""
        # Remove extension
        stem = Path(filename).stem
        
        # Parse "Author - Title (Year)" format
        authors = None
        title = stem
        year = None
        
        # Extract year if present
        import re
        year_match = re.search(r'\((\d{4})\)', stem)
        if year_match:
            year = int(year_match.group(1))
            title = stem[:year_match.start()].rstrip()
        
        # Split author and title
        if " - " in title:
            parts = title.split(" - ", 1)
            authors = parts[0]
            title = parts[1]
        
        return {
            "authors": authors,
            "title": title,
            "year": year,
            "extension": Path(filename).suffix.lower()
        }
    
    def add_series_prefix(self, title: str) -> str:
        """Add a series prefix to the title."""
        prefix = random.choice(self.SERIES_PREFIXES)
        separator = random.choice([" - ", " "])
        return f"{prefix}{separator}{title}"
    
    def add_source_suffix(self, filename: str, extension: str) -> str:
        """Add a source indicator suffix."""
        suffix = random.choice(self.SOURCE_SUFFIXES)
        # Some suffixes already include the extension
        if suffix.endswith(".pdf"):
            return filename + suffix
        else:
            return filename + suffix + extension
    
    def add_year_variations(self, title: str, year: int) -> str:
        """Add year with publisher or different formats."""
        if not year:
            return title
        
        variation = random.choice([
            f"{title} ({year}, {random.choice(self.PUBLISHERS)})",
            f"{title} ({year})",
            f"{title} {year}, {random.choice(self.PUBLISHERS)}",
        ])
        return variation
    
    def add_bracket_variations(self, title: str) -> str:
        """Add orphaned brackets or mismatched parens."""
        variations = [
            f"{title} (",
            f"{title} [",
            f"{title} )",
            f"{title} ]",
            f"{title} ((extra))",
            f"{title} [extra bracket",
        ]
        return random.choice(variations)
    
    def add_underscore_noise(self, title: str) -> str:
        """Replace spaces with underscores or add extra underscores."""
        variations = [
            title.replace(" ", "_"),
            title.replace(" ", "_") + "_",
            f"_{title}",
            f"{title}_",
        ]
        return random.choice(variations)
    
    def create_failed_download(self, filename: str) -> str:
        """Create a failed download version."""
        return filename + ".download"
    
    def create_crdownload_file(self, filename: str) -> str:
        """Create a Chrome failed download version."""
        return filename + ".crdownload"
    
    def create_small_file(self, filename: str, output_dir: Path) -> Path:
        """Create a small version of the file (< 1KB)."""
        output_path = output_dir / f"small_{filename}"
        # Create a tiny file
        with open(output_path, 'wb') as f:
            f.write(b'x' * 100)  # 100 bytes
        return output_path
    
    def create_corrupted_pdf(self, filename: str, output_dir: Path) -> Path:
        """Create a corrupted PDF file."""
        output_path = output_dir / f"corrupted_{filename}"
        # Create a file that's not a valid PDF
        with open(output_path, 'wb') as f:
            f.write(b'This is not a PDF file at all!')
        return output_path
    
    def generate_variations(self, clean_file: Path, output_dir: Path, max_variations: int = 5) -> List[Path]:
        """Generate noise variations for a clean file."""
        variations = []
        
        # Parse the clean filename
        parsed = self.parse_clean_filename(clean_file.name)
        
        # Copy original content for variations
        with open(clean_file, 'rb') as src:
            original_content = src.read()
        
        for i in range(max_variations):
            noise_type = random.choice([
                'series_prefix',
                'source_suffix', 
                'year_variation',
                'bracket_noise',
                'underscore_noise',
                'failed_download',
                'crdownload',
            ])
            
            # Build noisy filename
            title = parsed['title']
            authors = parsed['authors']
            year = parsed['year']
            extension = parsed['extension']
            
            if noise_type == 'series_prefix':
                title = self.add_series_prefix(title)
            elif noise_type == 'source_suffix':
                filename = f"{authors} - {title}" if authors else title
                if year:
                    filename += f" ({year})"
                filename = self.add_source_suffix(filename, extension)
            elif noise_type == 'year_variation':
                title = self.add_year_variations(title, year)
            elif noise_type == 'bracket_noise':
                title = self.add_bracket_variations(title)
            elif noise_type == 'underscore_noise':
                title = self.add_underscore_noise(title)
            elif noise_type == 'failed_download':
                filename = f"{authors} - {title}" if authors else title
                if year:
                    filename += f" ({year})"
                filename = self.create_failed_download(filename)
            elif noise_type == 'crdownload':
                filename = f"{authors} - {title}" if authors else title
                if year:
                    filename += f" ({year})"
                filename = self.create_crdownload_file(filename)
            
            if noise_type not in ['failed_download', 'crdownload']:
                filename = f"{authors} - {title}" if authors else title
                if year:
                    filename += f" ({year})"
                filename += extension
            
            # Sanitize filename
            filename = filename.replace('/', '_').replace('\\', '_')
            
            output_path = output_dir / f"noise_{i+1}_{filename}"
            
            # Write the variation
            with open(output_path, 'wb') as f:
                if noise_type in ['failed_download', 'crdownload']:
                    # Failed downloads get minimal content
                    f.write(b'Partial download content')
                else:
                    f.write(original_content)
            
            variations.append(output_path)
        
        # Also create some small and corrupted files
        if extension == '.pdf':
            variations.append(self.create_small_file(clean_file.name, output_dir))
            variations.append(self.create_corrupted_pdf(clean_file.name, output_dir))
        
        return variations
    
    def create_duplicate_files(self, source_file: Path, output_dir: Path, num_duplicates: int = 2) -> List[Path]:
        """Create duplicate files with different names but same content."""
        duplicates = []
        
        with open(source_file, 'rb') as src:
            content = src.read()
        
        for i in range(num_duplicates):
            # Preserve relative directory structure
            relative_path = source_file.relative_to(source_file.parent)  # Just the filename
            
            variations = [
                f"Copy_{i+1}_{source_file.name}",
                f"{source_file.stem}_duplicate_{i+1}{Path(source_file).suffix.lower()}",
            ]
            
            duplicate_name = random.choice(variations)
            duplicate_path = output_dir / duplicate_name
            
            with open(duplicate_path, 'wb') as f:
                f.write(content)
            
            duplicates.append(duplicate_path)
        
        return duplicates


def main():
    parser = argparse.ArgumentParser(description="Generate noise variations from clean ebook files")
    parser.add_argument(
        "--clean-dir",
        required=True,
        help="Directory containing clean ebook files"
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="Directory to write noisy variations"
    )
    parser.add_argument(
        "--max-variations",
        type=int,
        default=5,
        help="Maximum noise variations per clean file"
    )
    parser.add_argument(
        "--create-duplicates",
        action="store_true",
        help="Create duplicate files for testing duplicate detection"
    )
    parser.add_argument(
        "--random-seed",
        type=int,
        default=42,
        help="Random seed for reproducible results"
    )
    
    args = parser.parse_args()
    
    clean_dir = Path(args.clean_dir)
    output_dir = Path(args.output_dir)
    
    if not clean_dir.exists():
        print(f"Clean directory does not exist: {clean_dir}")
        return 1
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Find clean files
    clean_files = list(clean_dir.glob("*.pdf")) + list(clean_dir.glob("*.epub")) + list(clean_dir.glob("*.txt"))
    
    if not clean_files:
        print("No clean ebook files found")
        return 0
    
    print(f"Found {len(clean_files)} clean files")
    
    generator = NoiseGenerator(args.random_seed)
    
    total_variations = 0
    
    # Generate variations for each clean file
    for clean_file in clean_files:
        print(f"Processing {clean_file.name}...")
        variations = generator.generate_variations(clean_file, output_dir, args.max_variations)
        total_variations += len(variations)
        
        # Create duplicates if requested
        if args.create_duplicates:
            duplicates = generator.create_duplicate_files(clean_file, output_dir, 2)
            total_variations += len(duplicates)
    
    print(f"\n✓ Generated {total_variations} noisy files in {output_dir}")
    return 0


if __name__ == "__main__":
    exit(main())
