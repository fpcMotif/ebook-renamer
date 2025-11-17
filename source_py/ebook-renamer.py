#!/usr/bin/env python3
"""
Ebook Renamer - Python Implementation

A tool for batch renaming and organizing downloaded books and arXiv files.
This Python implementation maintains perfect behavioral parity with the Rust version.
"""

import sys
from ebook_renamer.cli import main

if __name__ == "__main__":
    sys.exit(main())
