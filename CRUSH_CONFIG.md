# Crush Agent Configuration Guide

## Overview
This guide explains how to configure the Crush agent for Python, Go, and Rust development, particularly for the ebook-renamer multi-language project.

## Understanding LSP and MCP

### LSP (Language Server Protocol)
- **What it is**: Protocol used by editors (VS Code, Neovim) to provide language features
- **Client**: Your text editor
- **Server**: Language-specific analyzer (rust-analyzer, gopls, pylsp)
- **Usage**: Not directly used by the Crush agent

### MCP (Model Context Protocol)
- **What it is**: Protocol for giving AI assistants context about your codebase
- **Client**: The Crush agent (this is me!)
- **Server**: Your project via configuration files
- **Usage**: Enables me to understand project structure, commands, and requirements

## Configuration Files for Crush Agent

### 1. CLAUDE.md (Already exists)
**Purpose**: Primary documentation for Claude/Crush
**Location**: `/Users/f/format/CLAUDE.md`
**Key Content**:
- Project overview and behavioral parity requirement
- Build commands for all languages
- Module architecture
- Deterministic behavior rules
- Cross-language testing procedures

### 2. memory.md (Created)
**Purpose**: Quick command reference for me
**Location**: `/Users/f/format/memory.md`
**Key Content**:
- Build commands (cargo, go, python)
- Test commands
- Cross-language verification
- LSP server installation commands

### 3. .cursorrules (Optional - for Cursor users)
**Purpose**: Cursor-specific instructions
**Location**: `/Users/f/format/.cursorrules`
**Key Content**:
- Same behavioral parity requirements
- Structure reference
- Quick command cheatsheet

### 4. .cursor/mcp.json (Optional - Cursor-specific)
**Purpose**: MCP configuration for Cursor
**Location**: `/Users/f/format/.cursor/mcp.json`
**Content**: Editor settings and file associations

## How to Configure Crush Agent

### Method 1: Using Memory Files (Recommended)

The memory.md file I've created contains all the commands I need:

```bash
# When you ask me to build Rust:
cargo build --release

# When you ask me to test Go:
cd source_go && go test ./...

# When you ask me to run Python:
python3 source_py/ebook-renamer.py --dry-run --json /path
```

### Method 2: Using .cursorrules for Enhanced Context

I've already created `.cursorrules` which tells me:
- This is a multi-language project
- Behavioral parity across 7 languages is mandatory
- I must update ALL implementations when changing core logic
- Run cross-language tests after changes

### Method 3: Project-Specific Context

The CLAUDE.md file provides comprehensive project context:
- Normalization rules
- Duplicate detection logic
- JSON output requirements
- Testing procedures

## Setting Up Your Local Editor (For You, Not Crush)

Since you're unfamiliar with LSP, here's how to set up your text editor:

### Option 1: VS Code (Easiest)
1. Install VS Code
2. Install extensions:
   - "rust-lang.rust-analyzer"
   - "golang.go"
   - "ms-python.python"
3. The `.vscode/settings.json` file I created will auto-configure everything

### Option 2: Neovim (More Control)
1. Install Neovim
2. Install packer.nvim or similar
3. Add configuration for rust-analyzer, gopls, pylsp
4. Use file I provide below

## Testing Your Configuration

### Verify Rust LSP
```bash
# In terminal:
cargo check

# In VS Code/Nvim:
# Open src/normalizer.rs
# Try: hover over function names, go-to-definition
# Should show type info and documentation
```

### Verify Go LSP
```bash
# In terminal:
cd source_go
go vet ./...

# In VS Code/Nvim:
# Open source_go/internal/normalizer/normalizer.go
# Try: hover over types, find references
```

### Verify Python LSP
```bash
# In terminal:
cd source_py
python3 -m py_compile ebook_renamer/normalizer.py

# In VS Code/Nvim:
# Open source_py/ebook_renamer/normalizer.py
# Try: hover over functions, parameter info
```

### Verify Cross-Language Testing (Critical for This Project)
```bash
# This is the most important test:
./tests/tools/test_cross_language.sh test_fixtures/noisy

# Should show: "All implementations produce identical output!"
```

## Troubleshooting

### Common Issues

1. **"Command not found: cargo"**
   - Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Restart terminal

2. **"Command not found: go"**
   - Install Go: https://go.dev/doc/install

3. **"ModuleNotFoundError" in Python**
   - Install rich: `pip install rich`

4. **Tests fail with "behavioral parity" error**
   - Check CLAUDE.md
   - All implementations must be updated together
   - Check sorting rules in CLAUDE.md

## Summary: What These Configurations Do

### For Me (Crush Agent)
- `CLAUDE.md` + `memory.md` = I know all the commands and structure
- `.cursorrules` = I know about the behavioral parity requirement
- `mcp.json` = Editor-specific file associations (optional)

### For You (Developer)
- LSP servers (rust-analyzer, gopls, pylsp) = Code intelligence in your editor
- Memory files = Quick reference for commands
- Test scripts = Verify everything works

## Quick Reference: Most Important Commands

```bash
# Build everything
cargo build --release
cd source_go && go build -o ebook-renamer ./cmd/ebook-renamer

# Test everything
cargo test
cd source_go && go test ./...
cd ../source_py && python3 -m pytest

# Verify cross-language parity (DO THIS OFTEN)
cd ..
./tests/tools/test_cross_language.sh test_fixtures/noisy
```

## Need More Help?

If you're still unfamiliar with LSP/MCP:

1. **LSP is editor-only**: You don't need to configure it for me (Crush) - just for your text editor

2. **MCP is about context**: The files I've created give me the context I need

3. **The critical thing**: Run cross-language tests after any change - that's enforced by behavioral parity requirement

4. **When in doubt**: Check `CLAUDE.md` - it has everything
