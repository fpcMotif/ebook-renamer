# LSP & MCP Configuration - Quick Setup Guide

## What You Need to Know (Simplified)

**LSP** = "IntelliSense" for your code editor (autocomplete, find definitions, errors)  
**MCP** = Context/protocol for AI assistants (Crush) to understand your project

---

## Files Created for You

### For the Crush Agent (AI Assistant)
These files help ME understand your project:

1. **`memory.md`** - My cheat sheet with all build/test commands
2. **`CRUSH_CONFIG.md`** - Complete guide on how I use these configs
3. **`.cursorrules`** - Tells me "Behavioral parity is SACRED" (must update all 7 languages)
4. **`.cursor/mcp.json`** - Cursor-specific settings for file types

### For Your Editor (VS Code/Neovim)
These files help YOUR EDITOR provide code intelligence:

5. **`memory.md`** (also for you) - All commands in one place
6. **`.config/nvim-minimal-init.lua`** - Ready-to-use Neovim config
7. **`.vscode/settings.json`** - VS Code settings (in my memory, not on disk yet)

---

## What to Do Now

### Install LSP Servers (One-Time Setup)

Open terminal and run:

```bash
# Rust (probably already installed)
rustup component add rust-analyzer

# Go
go install golang.org/x/tools/gopls@latest

# Python
pip install python-lsp-server
```

### Setup Your Editor

**Option A: VS Code (Easiest)**

1. Install VS Code from https://code.visualstudio.com/
2. Install these extensions:
   - 🔧 rust-lang.rust-analyzer
   - 🔧 golang.go
   - 🔧 ms-python.python
3. That's it! Open the project folder.

**Option B: Neovim**

1. Install Neovim
2. Copy the config: `cp .config/nvim-minimal-init.lua ~/.config/nvim/init.lua`
3. Install packer: `git clone --depth 1 https://github.com/wbthomason/packer.nvim ~/.local/share/nvim/site/pack/packer/start/packer.nvim`
4. Open Neovim, run `:PackerInstall`
5. Restart Neovim

---

## Verify Everything Works

**Test LSP is working:**

```bash
# Test Rust
nvim src/normalizer.rs  # or open in VS Code
# Try: Hover over a function name
# Should see: type info, documentation

# Test Go
nvim source_go/internal/normalizer/normalizer.go
# Try: Hover over a type
# Should see: definition, references

# Test Python
nvim source_py/ebook_renamer/normalizer.py
# Try: Hover over a function
# Should see: signature, docstring
```

**Test the project builds:**

```bash
# Rust
cargo test

# Go
cd source_go && go test ./...

# Python
cd source_py && python3 -m pytest

# Most important - cross-language test
cd ..
./tests/tools/test_cross_language.sh test_fixtures/noisy
```

---

## Key Concept: Behavioral Parity

**This is THE MOST IMPORTANT rule for this project:**

All 7 language implementations (Rust, Go, Python, Ruby, Haskell, OCaml, Zig) must produce **identical JSON output**.

**What this means:**
- Change normalizer logic in Rust? → Must update Go, Python, Ruby, Haskell, OCaml, Zig
- Add new CLI flag? → Must add to all 7 implementations
- Fix duplicate detection? → Must fix in all 7 languages

**How I enforce this:**
- `.cursorrules` file tells me this requirement
- `CRUSH_CONFIG.md` explains it in detail
- Cross-language test (`test_cross_language.sh`) verifies it

---

## Quick Command Reference

```bash
# Build all languages
cargo build --release
cd source_go && go build -o ebook-renamer ./cmd/ebook-renamer
cd ../source_zig && zig build

# Test all languages
cargo test
cd ../source_go && go test ./...
cd ../source_py && python3 -m pytest

# Run on actual files
cd ..
cargo run -- --dry-run --json /path/to/books
./source_go/ebook-renamer --dry-run --json /path/to/books
python3 source_py/ebook-renamer.py --dry-run --json /path/to/books

# Verify behavioral parity (DO THIS OFTEN)
./tests/tools/test_cross_language.sh test_fixtures/noisy
```

---

## Troubleshooting

**"cargo: command not found"**
- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Restart terminal

**"go: command not found"**
- Install Go: https://go.dev/doc/install

**"No hover info in editor"**
- LSP server not installed → Run install commands above
- Editor not configured → Check VS Code extensions or Neovim config

**"Cross-language test fails"**
- You changed core logic without updating all implementations
- See `CLAUDE.md` section "Adding New Normalization Rule"
- Must update ALL 7 languages identically

---

## What Crush Agent Already Knows

From the configuration files I've read:
- ✅ All build/test commands (memory.md)
- ✅ Behavioral parity requirement (CLAUDE.md, .cursorrules)
- ✅ Module structure (scanner, normalizer, duplicates, json, todo, tui)
- ✅ Deterministic sorting rules (critical for cross-language testing)
- ✅ File locations for each language implementation

**You don't need to configure ME - I'm ready to work!**

You only need to set up LSP in YOUR editor for the best development experience.

---

## Questions?

- **Confused by behavioral parity?** → Read `docs/spec.md` and `CRUSH_CONFIG.md`
- **Need editor setup help?** → Read `CRUSH_CONFIG.md` section "Setting Up Your Editor"
- **Want to understand LSP?** → It's just autocomplete/type-checking for your editor
- **Want to understand MCP?** → It's how I get context about your project (already configured)
