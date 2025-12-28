# Go Implementation

Go port of the ebook renamer, intended to match the Rust behavior and JSON output.

## Build
```bash
cd source_go
go mod tidy
go build -o ebook-renamer ./cmd/ebook-renamer
```

## Run
```bash
./ebook-renamer --dry-run --json /path/to/books
```

## Tests
```bash
go test ./...
```

## References
- `../docs/formatting_standards.md`
- `../docs/spec.md`
