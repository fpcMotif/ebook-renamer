# Development Memory

## Project Structure
Multi-language ebook renamer (Rust, Go, Python, Ruby, Haskell, Zig, OCaml)
- Rust: Primary implementation in `/Users/f/format/`
- Go: `/Users/f/format/source_go/`
- Python: `/Users/f/format/source_py/`
- Test data: `/Users/f/format/test_output/`

## Build Commands
```bash
# Rust
cargo build --release
cargo test
cargo run -- --dry-run --json /path

# Go
cd source_go && go build -o ebook-renamer ./cmd/ebook-renamer
cd source_go && go test ./...

# Python
python3 source_py/ebook-renamer.py --dry-run --json /path
cd source_py && python3 -m pytest

# Cross-language verification
./tests/tools/test_cross_language.sh /path
```

## LSP Setup Commands
```bash
# Install LSP servers
rustup component add rust-analyzer
go install golang.org/x/tools/gopls@latest
pip install python-lsp-server
```
