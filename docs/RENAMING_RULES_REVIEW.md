# Renaming Rules Review

**Date:** 2025-11-21
**Reviewer:** Jules (AI Assistant)
**Status:** Implemented

## Overview

This review documents the changes made to the ebook renaming rules to improve accuracy, coverage, and robustness. The changes primarily focus on better noise removal, expanded genre/publisher detection, and case-insensitive matching.

## Key Improvements

### 1. Expanded Keyword Lists

The list of keywords used to detect and remove publisher/series/genre information has been significantly expanded (from ~30 to ~120 keywords).

**New Categories Added:**
- **Major Publishers:** Wiley, Pearson, McGraw-Hill, etc.
- **General Book Types:** Fiction, Novel, Guide, Reference, Biography, etc.
- **Academic Types:** Thesis, Dissertation, Proceedings, Lecture, etc.
- **Editions:** Revised Edition, 1st ed, 2nd ed, etc.
- **Formats/Quality:** OCR, Scanned, Retail, DRM-free, etc.
- **Multilingual:** Chinese (小说, 教材) and Japanese (小説, 教科書) keywords.

### 2. Case-Insensitive Matching

Previously, keyword matching was case-sensitive, leading to missed detections (e.g., "fiction" vs "Fiction").
**Change:** All keyword matching is now case-insensitive.

### 3. Pattern Recognition

New regex patterns were added to identify and remove dynamic metadata that cannot be captured by static keywords alone.

- **Version Patterns:** `v1.0`, `ver 2.0`, `version 3.5`
- **Page Counts:** `500 pages`, `200pp`, `300 P`
- **Language Tags:** `English Edition`, `Chinese Edition`

### 4. Robust Noise Cleanup

Refined regex patterns for identifying file-sharing platform noise (`libgen`, `Z-Library`, `Anna's Archive`).
- Improved handling of `libgen.li` and `Z-Library` suffixes.
- Prevention of accidental deletion of periods that caused word sticking.

### 5. Cross-Language Consistency

All changes have been implemented and verified in:
- **Rust** (`src/normalizer.rs`) - Primary implementation
- **Go** (`source_go/internal/normalizer/normalizer.go`)
- **Python** (`source_py/ebook_renamer/normalizer.py`)

## Implementation Details

### Rust
- Updated `is_publisher_or_series_info` to use `to_lowercase()` and check new regexes.
- Added `remove_common_patterns` function called by `clean_title`.
- Added comprehensive tests in `src/normalizer.rs`.

### Go
- Updated `isPublisherOrSeriesInfo` with lowercase check and new keywords.
- Added `removeCommonPatterns` function.
- Updated `cleanTitle` to call pattern removal.
- Added tests in `source_go/internal/normalizer/normalizer_test.go`.

### Python
- Updated `_is_publisher_or_series_info` with lowercase check and keywords.
- Added `_remove_common_patterns` method.
- Updated `_clean_title` to use new logic.
- Added tests in `source_py/tests/test_normalizer.py`.

## Verification

Tests were run for all three languages:
- **Rust:** All tests passed (including new cases).
- **Go:** All tests passed.
- **Python:** All tests passed.

The system is now more robust against edge cases like:
- `Great Novel (Fiction) (John Doe).pdf` → `John Doe - Great Novel.pdf`
- `Learn Python (3rd Edition).pdf` → `Learn Python.pdf`
- `Title libgen.li.pdf` → `Title.pdf`

## Next Steps

- Monitor user feedback for any false positives with the expanded keyword list.
- Consider adding a configuration file to allow users to customize the keyword list without recompiling.
