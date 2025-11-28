# Ebook Renamer - Multi-Language Implementation

A tool for batch renaming and organizing downloaded books and arXiv files, implemented in Rust, Go, and Python with perfect cross-language behavioral parity.

## Overview

This project provides three implementations of the same ebook renaming tool:
- **Rust**: Original implementation with maximum performance
- **Go**: Production-ready implementation with excellent dependency management
- **Python**: Readable implementation with easy customization

All three implementations produce identical JSON output and follow the same deterministic behavior.

## Features

- 🔍 **File Scanning**: Recursive directory scanning with configurable depth
- 📝 **Filename Normalization**: Intelligent parsing of author, title, and year
- 🔄 **Duplicate Detection**: MD5-based duplicate detection with smart retention strategy
- 📋 **Todo List Generation**: Automatic generation of `todo.md` for manual review
- ⚡ **JSON Output**: Machine-readable output for automation and testing
- 🌐 **Multi-Platform**: Works on Windows, macOS, and Linux
- ☁️ **Cloud Storage**: Direct integration with Dropbox and Google Drive for remote file management

## Documentation

- **[Formatting Standards](docs/formatting_standards.md)**: Detailed rules for filename normalization
- **[Quick Reference](docs/formatting_quick_reference.md)**: Summary of regex patterns and rules

## Quick Start

### Rust Implementation
```bash
# Build
cargo build --release

# Run (dry-run with JSON output)
./target/release/ebook_renamer --dry-run --json /path/to/books
```

### Go Implementation
```bash
# Build dependencies
cd source_go && go mod tidy

# Build
go build -o ebook-renamer ./cmd/ebook-renamer

# Run
./ebook-renamer --dry-run --json /path/to/books
```

### Python Implementation
```bash
# Run directly
python3 source_py/ebook-renamer.py --dry-run --json /path/to/books
```

### Ruby Implementation
```bash
# Run directly
ruby source_rb/ebook-renamer.rb --dry-run --json /path/to/books
```

## Cloud Storage Integration

The tool now supports direct integration with Dropbox and Google Drive, allowing you to organize your ebook library directly in cloud storage without downloading files locally.

### Supported Cloud Providers

- **Dropbox**: Rename and organize files in your Dropbox account
- **Google Drive**: Rename and organize files in your Google Drive account

### Getting Cloud Access Tokens

#### Dropbox
1. Go to [Dropbox App Console](https://www.dropbox.com/developers/apps)
2. Create a new app or use an existing one
3. Generate an access token
4. Use the token with `--cloud-token` or set `CLOUD_ACCESS_TOKEN` environment variable

#### Google Drive
1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or use an existing one
3. Enable Google Drive API
4. Create OAuth 2.0 credentials
5. Obtain an access token
6. Use the token with `--cloud-token` or set `CLOUD_ACCESS_TOKEN` environment variable

### Cloud Usage Examples

```bash
# Dropbox: Dry run to see what would be renamed
cargo run -- --cloud-provider dropbox --cloud-token YOUR_TOKEN --cloud-path /Books --dry-run

# Dropbox: Actually rename files
cargo run -- --cloud-provider dropbox --cloud-token YOUR_TOKEN --cloud-path /Books

# Google Drive: Organize ebooks in a specific folder
cargo run -- --cloud-provider google-drive --cloud-token YOUR_TOKEN --cloud-path /MyEbooks --dry-run

# Using environment variable for token
export CLOUD_ACCESS_TOKEN=your_token_here
cargo run -- --cloud-provider dropbox --cloud-path /Books
```

### Important Notes for Cloud Storage

- **No MD5 Hashing**: Duplicate detection is skipped in cloud mode to avoid downloading files
- **File Formats**: Supports PDF, EPUB, DJVU, TXT, and MOBI files
- **Dry Run Recommended**: Always use `--dry-run` first to preview changes
- **Rate Limits**: Cloud APIs have rate limits; the tool respects them but may be slower for large libraries

## CLI Reference

All implementations share the same CLI interface:

```
ebook-renamer [OPTIONS] [PATH]

Arguments:
  PATH                  Directory to scan (default: current directory)

Options:
  -d, --dry-run         Show changes without applying them
  --json                Output in JSON format
  --max-depth N         Maximum directory depth (default: unlimited)
  --no-recursive        Only scan top-level directory
  --extensions EXT      Comma-separated extensions (default: pdf,epub,txt,djvu)
  --no-delete           Don't delete duplicates, only list them
  --todo-file PATH      Custom todo.md location
  --delete-small        Delete files < 1KB instead of adding to todo
  --preserve-unicode    Preserve non-Latin scripts
  --verbose             Enable verbose logging
  --skip-cloud-hash     Skip MD5 hash computation (for cloud storage)
  --cloud-provider P    Cloud provider: dropbox or google-drive
  --cloud-token TOKEN   Access token for cloud storage API
  --cloud-path PATH     Cloud storage path to process (default: /)
```

## JSON Output Schema

When `--json` flag is used, the tool outputs structured JSON:

```json
{
  "renames": [
    {
      "from": "original/path.pdf",
      "to": "normalized/path.pdf", 
      "reason": "normalized"
    }
  ],
  "duplicate_deletes": [
    {
      "keep": "file/to/keep.pdf",
      "delete": ["duplicate1.pdf", "duplicate2.pdf"]
    }
  ],
  "small_or_corrupted_deletes": [
    {
      "path": "small/file.pdf",
      "issue": "deleted"
    }
  ],
  "todo_items": [
    {
      "category": "failed_download",
      "file": "incomplete.download",
      "message": "重新下载: incomplete.download (未完成下载)"
    }
  ]
}
```

## Testing and Validation

### Cross-Language Testing
```bash
# Test all implementations against the same data
./tests/tools/test_cross_language.sh /path/to/test/files
```

### Golden Reference Testing
```bash
# Generate test fixtures from real downloads
python3 tests/tools/import_from_downloads.py --downloads ~/Downloads --output test_fixtures

# Create noise variations for testing
python3 tests/tools/generate_noise.py --clean-dir test_fixtures/clean --output-dir test_fixtures/noisy

# Generate golden JSON reference
python3 tests/tools/build_golden_from_rust.py --target-dir test_fixtures/noisy --output-dir test_results
```

## Architecture

### Module Structure
All implementations follow the same modular architecture:

```
├── CLI Module          # Argument parsing and orchestration
├── Scanner Module      # File system scanning and filtering
├── Normalizer Module   # Filename parsing and normalization
├── Duplicates Module   # MD5-based duplicate detection
├── Todo Module         # Todo list generation and management
└── JSON Output Module  # Structured output with deterministic sorting
```

### Deterministic Behavior
- All arrays are sorted deterministically for cross-language consistency
- Paths use POSIX-style separators in JSON output
- Empty arrays are output as `[]` not `null`
- Todo items sorted by category then filename

## File Processing Rules

### Supported Extensions
- **Duplicates**: `.pdf`, `.epub`, `.txt`, `.djvu`
- **Failed Downloads**: `.download`, `.crdownload`
- **All Formats**: `.pdf`, `.epub`, `.txt`, `.mobi`, `.djvu`, `.download`, `.crdownload`

### Normalization Rules
1. Remove series prefixes (e.g., "Graduate Texts in Mathematics")
2. Remove source indicators (e.g., "- libgen.li", "(Z-Library)")
3. Extract year from various formats: `(2020)`, `(2020, Publisher)`, `2020, Publisher`
4. Split authors and title using: `" - "`, `":"`, or trailing `(author)`
5. Clean orphaned brackets and replace underscores with spaces
6. Output format: `Author - Title (Year).ext`

### Duplicate Detection Strategy
1. Filter to allowed formats (`.pdf`, `.epub`, `.txt`)
2. Group by MD5 hash
3. For each group, keep file with highest priority:
   - **Priority 1**: Already normalized files
   - **Priority 2**: Files in shallowest directory
   - **Priority 3**: Most recently modified files

## Development

### Adding New Implementations
To add a new language implementation:

1. Follow the modular structure above
2. Implement the exact same CLI interface
3. Ensure JSON output matches the schema exactly
4. Add to the cross-language test harness
5. Validate against golden reference

### Running Tests
```bash
# Rust tests
cargo test

# Go tests  
cd source_go && go test ./...

# Python tests (if implemented)
cd source_py && python3 -m pytest
```

## Performance

| Implementation | Build Time | Runtime | Binary Size | Dependencies |
|---|---|---|---|---|
| Rust | ~30s | Fastest | ~8MB | Minimal |
| Go | ~5s | Fast | ~15MB | Standard library + cobra |
| Python | N/A | Moderate | N/A | Standard library |

## License

This project is licensed under the MIT License.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Ensure all three implementations pass cross-language tests
4. Submit a pull request

## Git & Version Control Guide

### What is Git?

Git is a distributed version control system (DVCS) that tracks changes in your codebase over time. Unlike centralized systems, every developer has a complete copy of the repository history, enabling offline work, fast operations, and powerful branching capabilities.

### Core Git Concepts

#### Repository Structure
- **Working Directory**: Your current project files
- **Staging Area (Index)**: Files prepared for the next commit
- **Local Repository**: Complete version history stored in `.git/`
- **Remote Repository**: Hosted version (e.g., on GitHub)

#### Key Terms
- **Commit**: A snapshot of your project at a specific point in time, with a unique SHA hash
- **Branch**: An independent line of development (default: `master` or `main`)
- **HEAD**: Pointer to your current commit/branch
- **Remote**: A version of your repository hosted elsewhere
- **Origin**: Default name for the primary remote repository

### Essential Git Commands

#### Initial Setup
```bash
# Configure your identity
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"

# Initialize a new repository
git init

# Clone an existing repository
git clone https://github.com/user/repo.git
```

#### Daily Workflow
```bash
# Check status of working directory
git status

# Stage specific files
git add file1.rs file2.go

# Stage all changes
git add .

# Create a commit
git commit -m "Add feature X with tests"

# Push to remote
git push origin main

# Pull latest changes
git pull origin main
```

#### Branching Strategy
```bash
# Create and switch to new branch
git checkout -b feature/normalize-unicode

# Switch between branches
git checkout main
git checkout feature/normalize-unicode

# List all branches
git branch -a

# Merge branch into current branch
git merge feature/normalize-unicode

# Delete merged branch
git branch -d feature/normalize-unicode
```

#### Inspecting History
```bash
# View commit history
git log --oneline --graph --all

# See what changed in a commit
git show abc123

# Compare working directory to last commit
git diff

# Compare staged changes
git diff --cached

# Compare branches
git diff main..feature/normalize-unicode
```

#### Undoing Changes
```bash
# Discard changes in working directory
git restore file.rs
git checkout -- file.rs  # older syntax

# Unstage files (keep changes)
git restore --staged file.rs
git reset HEAD file.rs   # older syntax

# Undo last commit (keep changes staged)
git reset --soft HEAD~1

# Undo last commit (keep changes unstaged)
git reset HEAD~1

# Undo last commit (discard all changes)
git reset --hard HEAD~1

# Revert a commit (creates new commit)
git revert abc123
```

### GitHub Workflows

#### Creating a New Repository
```bash
# Using GitHub CLI
gh repo create project-name --public --source=. --remote=origin

# Push initial commit
git push -u origin main
```

#### Pull Request Workflow
```bash
# Create feature branch
git checkout -b fix/duplicate-detection-bug

# Make changes and commit
git add .
git commit -m "Fix: resolve edge case in duplicate detection"

# Push branch to remote
git push origin fix/duplicate-detection-bug

# Create PR using GitHub CLI
gh pr create --title "Fix duplicate detection edge case" --body "Resolves issue with identical MD5 hashes"

# After PR is approved and merged
git checkout main
git pull origin main
git branch -d fix/duplicate-detection-bug
```

#### Keeping Your Fork Updated
```bash
# Add upstream remote (original repository)
git remote add upstream https://github.com/original-owner/repo.git

# Fetch upstream changes
git fetch upstream

# Merge upstream changes into your main
git checkout main
git merge upstream/main

# Push updated main to your fork
git push origin main
```

### Best Practices

#### Commit Messages
- **Format**: `Type: Brief description (50 chars max)`
- **Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- **Examples**:
  - `feat: add support for MOBI format`
  - `fix: handle Unicode in filename normalization`
  - `docs: update installation instructions`
  - `refactor: extract duplicate detection to separate module`

#### Commit Hygiene
1. **Commit often**: Small, focused commits are easier to review and revert
2. **One concern per commit**: Don't mix feature additions with refactoring
3. **Test before committing**: Ensure `cargo test` passes
4. **Write descriptive messages**: Future you will thank present you

#### Branch Naming
- `feature/description` - New features
- `fix/description` - Bug fixes
- `refactor/description` - Code improvements
- `docs/description` - Documentation updates
- `test/description` - Test additions

#### What NOT to Commit
```gitignore
# Build artifacts
/target/
*.o
*.so

# Dependencies
/node_modules/
Cargo.lock  # only for libraries

# IDE files
.vscode/
.idea/
*.swp

# Secrets and credentials
.env
*.key
*.pem
config.secret.json

# OS files
.DS_Store
Thumbs.db
```

### Advanced Git Techniques

#### Interactive Rebase
```bash
# Rewrite last 3 commits
git rebase -i HEAD~3

# Commands: pick, reword, edit, squash, fixup, drop
```

#### Cherry-Picking
```bash
# Apply specific commit from another branch
git cherry-pick abc123
```

#### Stashing
```bash
# Save work in progress
git stash

# List stashes
git stash list

# Apply most recent stash
git stash pop

# Apply specific stash
git stash apply stash@{2}
```

#### Bisect (Find Buggy Commit)
```bash
git bisect start
git bisect bad           # Current commit is bad
git bisect good abc123   # Known good commit
# Git checks out middle commit
cargo test               # Test if bug exists
git bisect good/bad      # Mark result
# Repeat until bug is found
git bisect reset
```

### Troubleshooting

#### Merge Conflicts
```bash
# When conflicts occur
git status  # See conflicted files

# Edit files to resolve conflicts (look for <<<<<<<, =======, >>>>>>>)
vim conflicted-file.rs

# Mark as resolved
git add conflicted-file.rs

# Complete merge
git commit
```

#### Accidentally Committed Secrets
```bash
# Remove file from history (DANGEROUS)
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch path/to/secret.key" \
  --prune-empty --tag-name-filter cat -- --all

# Or use BFG Repo-Cleaner (recommended)
bfg --delete-files secret.key

# Force push (requires coordination with team)
git push origin --force --all
```

### Git Hooks for This Project

Create `.git/hooks/pre-commit`:
```bash
#!/bin/sh
# Run tests before allowing commit
cargo test
if [ $? -ne 0 ]; then
  echo "Tests failed. Commit aborted."
  exit 1
fi

# Run Go tests
cd source_go && go test ./...
if [ $? -ne 0 ]; then
  echo "Go tests failed. Commit aborted."
  exit 1
fi
```

### Resources

- [Official Git Documentation](https://git-scm.com/doc)
- [Pro Git Book](https://git-scm.com/book/en/v2) (free online)
- [GitHub Guides](https://guides.github.com/)
- [Atlassian Git Tutorials](https://www.atlassian.com/git/tutorials)

---

*All three implementations maintain perfect behavioral parity through comprehensive testing and deterministic output specifications.*
