# Formatting Standards

This document outlines the formatting standards used by the ebook-renamer tool.

## General Philosophy

The goal is to produce clean, consistent filenames that are easy to read and organize.
Format: `Author - Title (Year).ext`

## Rules

### 1. Author Names
- Use "Lastname, Firstname" if detectable, otherwise "Firstname Lastname" is acceptable.
- Multiple authors should be separated by commas.
- `(auth.)`, `(eds.)`, `(translator)` tags are removed.
- CJK authors and special characters are supported.

### 2. Titles
- Subtitles are separated by `-` or `:`.
- Series information is generally removed if it appears as a prefix or in brackets `[]`.
- "Novel", "Fiction", "Guide", and other generic type identifiers in parentheses are removed.

### 3. Year
- Extracted from metadata or filename.
- Placed at the end in parentheses: `(2023)`.

### 4. Noise Removal
The following are actively removed from filenames:
- **Source Identifiers**: `Z-Library`, `libgen`, `Anna's Archive`
- **Hashes & IDs**: MD5 hashes, ISBNs, ASINs (`B0...`)
- **Publisher Keywords**: `Springer`, `Wiley`, `O'Reilly`, `Press`, `Publishing`
- **Book Types**: `Lecture Notes`, `Thesis`, `Handbooks`, `Manuals` (when in parens/brackets)
- **File Quality Tags**: `OCR`, `Scanned`, `Retail`, `HQ`
- **Version Info**: `v1.0`, `3rd Edition`, `Revised`

### 5. Structural Cleaning
- Orphaned brackets `)` or `]` are removed.
- Double spaces are collapsed to single spaces.
- Leading/trailing punctuation is stripped.

## Examples

| Original | Normalized |
|----------|------------|
| `Unknown (Fiction) - The Great Book (2020).pdf` | `Unknown - The Great Book (2020).pdf` |
| `Learning Rust (3rd Edition).epub` | `Learning Rust.epub` |
| `[Springer] Advanced Math (Vol 1).pdf` | `Advanced Math.pdf` |
| `My_Book_Title_v2.0_OCR.pdf` | `My Book Title.pdf` |
| `(Z-Library) Author - Title.pdf` | `Author - Title.pdf` |
