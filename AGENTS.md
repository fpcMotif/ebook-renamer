# Repository Guidelines

## Project Structure & Module Organization
Multi-language ebook renamer with deterministic cross-language parity.
- `src/`: Rust reference implementation.
- `source_go/`, `source_py/`, `source_rb/`: Go/Python/Ruby ports (see READMEs for status).
- `source_zig/`, `source_hs/`, `source_ml/`: Zig/Haskell/OCaml placeholders.
- `docs/`: Formatting rules and specs.
- `tests/tools/`: Cross-language harness and fixture scripts.
- `target/`, `test_output/`: Build/test outputs.

## Port READMEs
- `source_go/README.md`
- `source_py/README.md`
- `source_rb/README.md`
- `source_zig/README.md`
- `source_hs/README.md`
- `source_ml/README.md`

## Build, Test, and Development Commands
- `cargo build --release`
- `./target/release/ebook_renamer --dry-run --json /path/to/books`
- `cd source_go && go mod tidy && go build -o ebook-renamer ./cmd/ebook-renamer`
- `python3 source_py/ebook-renamer.py --dry-run --json /path/to/books`
- `ruby source_rb/ebook-renamer.rb --dry-run --json /path/to/books`
- `cd source_hs && stack build` (or `cabal build`)
- `cd source_ml && dune build`
- `cd source_zig && zig build`
- `./tests/tools/test_cross_language.sh /path/to/test/files`

## Coding Style & Naming Conventions
- Keep behavior and JSON output identical across implementations; update ports together.
- Use language-standard formatters and match existing module layouts.
- Filename format lives in `docs/formatting_standards.md` (example: `Author - Title [Series Vol] (Year, Edition).ext`).

## Testing Guidelines
- Rust: `cargo test`.
- Go: `cd source_go && go test ./...`.
- Python: `cd source_py && python3 -m pytest`.
- Ruby: `cd source_rb && ruby -Ilib:test test/test_*.rb`.
- Haskell placeholder: no suite yet; wire to `cd source_hs && stack test` or `cabal test` when added.
- OCaml placeholder: no tests yet; use `cd source_ml && dune runtest` once added.
- Zig placeholder: no tests yet; add a `test` step to `source_zig/build.zig` (then `zig build test`) or run `zig test src/main.zig`.
- Golden flow: `python3 tests/tools/import_from_downloads.py --downloads ~/Downloads --output test_fixtures`, `python3 tests/tools/generate_noise.py --clean-dir test_fixtures/clean --output-dir test_fixtures/noisy`, then `python3 tests/tools/build_golden_from_rust.py --target-dir test_fixtures/noisy --output-dir test_results`.

## Commit & Pull Request Guidelines
- Use prefixes: `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:` (example: `feat: add MOBI handling`).
- Branches: `feature/`, `fix/`, `refactor/`, `docs/`, `test/`.
- PRs: summary, test results (at least `cargo test` + cross-language harness), note behavior/output changes.
