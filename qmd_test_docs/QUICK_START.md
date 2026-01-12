# QMD Quick Start ⚡

## 30-Second Summary
qmd is now configured and ready! You have:
- ✅ 7 test documents indexed
- ✅ MCP server configured
- ✅ Embeddings generated
- 🔄 **Action needed:** Restart Claude Code

## Try These Now

### 1. Fast Search
```bash
qmd search "goroutine" -n 5
```

### 2. JSON Output (for AI)
```bash
qmd search "authentication" --json
```

### 3. Get Document
```bash
qmd get "qmd://test_docs/tutorials/rust-error-handling.md"
```

### 4. List Files
```bash
qmd ls test_docs
```

### 5. Run Demo
```bash
cd /Users/f/format/qmd_test_docs
./demo_qmd.sh
```

## After Restart: MCP Tools

Once you restart Claude Code, ask me things like:
- "Search my qmd docs for error handling"
- "Get the Go concurrency tutorial"
- "Find all authentication-related docs"
- "Show me the status of the qmd index"

I'll use these MCP tools:
- `qmd_search()` - Fast keyword search
- `qmd_get()` - Retrieve documents
- `qmd_multi_get()` - Batch retrieval
- `qmd_status()` - Index health

## Index Your Own Docs

```bash
# Add your documentation
qmd collection add ~/your-docs --name my_docs

# Add context (improves search)
qmd context add qmd://my_docs "Description of this collection"

# Generate embeddings
qmd embed

# Search it
qmd search "query" -c my_docs
```

## Key Commands Cheat Sheet

| What You Want | Command |
|---------------|---------|
| Search for keyword | `qmd search "keyword"` |
| Get JSON results | `qmd search "query" --json` |
| Get a file | `qmd get "qmd://path/to/file.md"` |
| Get multiple files | `qmd multi-get "pattern/*.md"` |
| List collection | `qmd ls collection_name` |
| Check status | `qmd status` |
| Update index | `qmd update` |
| Generate embeddings | `qmd embed` |

## Output Formats

```bash
qmd search "query" --json      # JSON for AI
qmd search "query" --files     # CSV list
qmd search "query" --full      # Full content
qmd search "query" --md        # Markdown
```

## Full Guides

1. **`QMD_POWER_USER_GUIDE.md`** - Complete reference
2. **`QMD_SETUP_COMPLETE.md`** - What we did
3. **`demo_qmd.sh`** - Interactive demo

---

**🎯 Next Step:** Restart Claude Code, then ask me to search using qmd!
