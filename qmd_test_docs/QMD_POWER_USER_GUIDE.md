# QMD Power User Guide

## Overview
QMD is a local-first search engine combining BM25 keyword search, vector embeddings, and LLM reranking for markdown documents.

## Setup Complete ✓
- ✅ qmd installed globally via bun
- ✅ MCP server configured in `.mcp.json`
- ✅ Enabled in `~/.claude/settings.json`
- ✅ Test collection created with 7 documents
- ✅ Embeddings generated (11 chunks)
- ✅ **Next step:** Restart Claude Code to activate MCP tools

## Quick Reference

### Core Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `qmd search` | Fast BM25 keyword search | `qmd search "goroutine" -n 5` |
| `qmd vsearch` | Semantic vector search | `qmd vsearch "concurrent programming"` |
| `qmd query` | Hybrid (best quality) | `qmd query "error handling best practices"` |
| `qmd get` | Retrieve specific doc | `qmd get "qmd://notes/file.md"` |
| `qmd multi-get` | Batch retrieval | `qmd multi-get "notes/*.md"` |
| `qmd ls` | List collection files | `qmd ls test_docs` |
| `qmd status` | Index health check | `qmd status` |

### Output Formats for AI Integration

```bash
# JSON - Best for Claude Code processing
qmd search "authentication" --json -n 10

# CSV - For spreadsheet import
qmd search "errors" --csv -c test_docs

# Files - List with docid, score, path
qmd search "database" --files

# Markdown - Human-readable reports
qmd search "concurrency" --md

# XML - Structured export
qmd search "api" --xml
```

### Search Options

```bash
# Limit results
qmd search "query" -n 5

# Filter by collection
qmd search "query" -c test_docs

# Minimum score threshold
qmd search "query" --min-score 0.5

# Get all matches (use with --min-score)
qmd search "query" --all --min-score 0.3

# Full document instead of snippet
qmd search "query" --full

# Include line numbers
qmd search "query" --line-numbers
```

### Document Retrieval

```bash
# Get by path
qmd get "qmd://test_docs/tutorials/rust-error-handling.md"

# Get with line limit
qmd get "qmd://test_docs/api/auth.md" -l 50

# Get from specific line
qmd get "file.md:100" -l 50

# Get by docid (from search results)
qmd get "#abc123"

# Multi-get with glob
qmd multi-get "tutorials/**/*.md" --json

# Multi-get with file size limit
qmd multi-get "**/*.md" --max-bytes 10240
```

### Collection Management

```bash
# Add new collection
qmd collection add ~/my-notes --name notes --mask "**/*.md"

# Add context metadata (improves search)
qmd context add qmd://notes "Personal knowledge base"
qmd context add qmd://notes/work "Work-related documentation"

# List all collections
qmd collection list

# Rename collection
qmd collection rename old_name new_name

# Remove collection
qmd collection remove old_name

# Update index after file changes
qmd update

# Update with git pull first
qmd update --pull

# Generate/update embeddings
qmd embed

# Force re-embed all docs
qmd embed -f
```

### Advanced Patterns

#### Searching Within Specific Collection
```bash
# Only search in test_docs
qmd search "authentication" -c test_docs --json
```

#### Export High-Quality Results
```bash
# Get top 30 results above 0.4 score
qmd query "database optimization" --all --min-score 0.4 --json
```

#### Batch Document Retrieval
```bash
# Get all tutorial files
qmd multi-get "qmd://test_docs/tutorials/*.md" --json

# Get multiple specific files
qmd multi-get "file1.md, file2.md, file3.md" -l 100
```

#### Fuzzy File Finding
```bash
# qmd get has fuzzy matching with suggestions
qmd get "rust-eror.md"  # Suggests: rust-error-handling.md
```

## MCP Tools (After Restart)

Once you restart Claude Code, these tools become available:

### `qmd_search` - Fast BM25 Keyword Search
```typescript
qmd_search({
  query: "goroutine channels",
  collection: "test_docs",  // optional
  limit: 10,
  min_score: 0.3
})
```

### `qmd_vsearch` - Semantic Vector Search
```typescript
qmd_vsearch({
  query: "handling errors gracefully",
  collection: "test_docs",
  limit: 5
})
```

### `qmd_query` - Hybrid Search (Best Quality)
```typescript
qmd_query({
  query: "best practices for concurrent programming",
  collection: "test_docs",
  limit: 20,
  min_score: 0.4
})
```

### `qmd_get` - Retrieve Single Document
```typescript
qmd_get({
  path: "qmd://test_docs/tutorials/rust-error-handling.md",
  max_lines: 100
})
```

### `qmd_multi_get` - Retrieve Multiple Documents
```typescript
qmd_multi_get({
  pattern: "qmd://test_docs/tutorials/*.md",
  max_lines: 50,
  max_bytes: 10240
})
```

### `qmd_status` - Index Health
```typescript
qmd_status({})  // No parameters needed
```

## Best Practices

### 1. Collection Organization
```bash
# Separate collections by topic
qmd collection add ~/work-docs --name work --mask "**/*.{md,txt}"
qmd collection add ~/personal-notes --name notes --mask "**/*.md"
qmd collection add ~/code-docs --name code --mask "**/*.{md,rst}"

# Add descriptive context to each
qmd context add qmd://work "Company documentation and meeting notes"
qmd context add qmd://notes "Personal knowledge base and ideas"
```

### 2. Search Strategy
- **Use `search`** for exact keyword matching (fastest)
- **Use `vsearch`** for semantic/conceptual queries
- **Use `query`** for best quality (slower, uses LLM reranking)

### 3. Output Format Selection
- **`--json`** when piping to other tools or Claude Code
- **`--files`** for quick file path listing with scores
- **`--md`** for human-readable reports
- **Default** for interactive terminal use

### 4. Performance Optimization
```bash
# Limit result count for faster responses
qmd search "query" -n 5

# Use collection filter to narrow scope
qmd search "query" -c specific_collection

# Use --files for lightweight output
qmd search "query" --files
```

### 5. Maintenance Routine
```bash
# Weekly: Update index and embeddings
qmd update
qmd embed

# Monthly: Clean up orphaned data
qmd cleanup

# After major changes: Force re-embed
qmd embed -f
```

## Real-World Use Cases

### Use Case 1: Find API Endpoints
```bash
qmd search "POST /auth" -c api_docs --json
```

### Use Case 2: Troubleshooting Guide Lookup
```bash
qmd query "database connection refused" -c troubleshooting -n 3
```

### Use Case 3: Code Example Search
```bash
qmd search "goroutine WaitGroup" -c code_examples --full
```

### Use Case 4: Research Paper Discovery
```bash
qmd vsearch "vector similarity search algorithms" -c research --json
```

### Use Case 5: Batch Export for Context
```bash
# Get all relevant docs for a topic
qmd query "authentication security" --all --min-score 0.5 --json > context.json
```

## Score Interpretation

| Score Range | Meaning | When to Use |
|-------------|---------|-------------|
| 0.8 - 1.0 | Highly relevant | Trust these results |
| 0.5 - 0.8 | Moderately relevant | Good for exploration |
| 0.2 - 0.5 | Somewhat relevant | May need filtering |
| 0.0 - 0.2 | Low relevance | Usually noise |

## Troubleshooting

### No Results Found
```bash
# Check collection has files
qmd ls your_collection

# Verify embeddings exist
qmd status

# Try broader search
qmd search "broad term" -c collection --all
```

### Slow Searches
```bash
# Use BM25 instead of hybrid
qmd search "query" -c collection

# Limit result count
qmd search "query" -n 5

# Use collection filter
qmd search "query" -c specific_collection
```

### Model Download Issues
```bash
# Check models directory
ls ~/.cache/qmd/models/

# Re-try download
qmd embed -f
```

## Integration Examples

### Claude Code Workflow
```typescript
// 1. Search for relevant docs
const results = await qmd_search({
  query: "error handling",
  collection: "tutorials",
  limit: 5
});

// 2. Retrieve full content
const doc = await qmd_get({
  path: results[0].file
});

// 3. Process and summarize
```

### Shell Script Integration
```bash
#!/bin/bash
# Find and process all authentication docs

qmd search "authentication" --files -c api_docs | while IFS=',' read -r docid score path context; do
  echo "Processing: $path (score: $score)"
  qmd get "$path" -l 100 > "processed_$(basename $path)"
done
```

## Next Steps

1. **Restart Claude Code** to activate MCP tools
2. **Index your actual content:**
   ```bash
   qmd collection add ~/your-docs --name my_docs
   qmd embed
   ```
3. **Try the MCP tools in Claude Code:**
   - Ask Claude to search using `qmd_search`
   - Retrieve documents with `qmd_get`
   - Check status with `qmd_status`

## Resources

- GitHub: https://github.com/tobi/qmd
- Models cached in: `~/.cache/qmd/models/`
- Index location: `~/.cache/qmd/index.sqlite`
- Configuration: `.mcp.json` in project root
