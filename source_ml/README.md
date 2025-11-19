# OCaml Implementation

This is a minimal OCaml implementation showing the basic structure and logging approach.

## Building

```bash
cd source_ml
dune build
```

## Running

```bash
dune exec ebook_renamer -- /path/to/ebooks
```

Or run the built executable directly:
```bash
./_build/default/bin/main.exe /path/to/ebooks
```

## Status

This is a placeholder implementation demonstrating:
- Basic OCaml project structure with Dune
- Logging to stderr with timestamps
- Command-line argument handling

A full implementation would require additional modules for:
- CLI argument parsing (using Cmdliner)
- File system scanning
- Filename normalization
- Duplicate detection
- Todo list generation
