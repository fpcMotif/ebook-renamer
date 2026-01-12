# QMD MCP Integration - Quick Reference

## Available MCP Tools (After Restart)

Once you restart Claude Code, the following qmd tools will be available directly:

### 1. `qmd_search` - Fast BM25 keyword search
```
Parameters:
- query: string (search term)
- collection: string (optional, filter to collection)
- limit: number (default: 5)
- output_format: "json" | "files" | "csv" | "full"
```

### 2. `qmd_vsearch` - Semantic vector search
```
Parameters:
- query: string (search term)
- collection: string (optional)
- limit: number (default: 5)
- output_format: "json" | "files" | "csv" | "full"
```

### 3. `qmd_query` - Hybrid search with LLM reranking
```
Parameters:
- query: string (search term)
- collection: string (optional)
- limit: number (default: 5)
- output_format: "json" | "files" | "csv" | "full"
```

### 4. `qmd_get` - Retrieve single document
```
Parameters:
- path: string (qmd:// URI or file path)
- line: number (optional, start line)
- limit: number (optional, max lines)
```

### 5. `qmd_multi_get` - Batch document retrieval
```
Parameters:
- pattern: string (glob pattern or comma-separated paths)
- limit: number (optional, max lines per file)
- max_bytes: number (default: 10240)
- output_format: "json" | "files" | "csv" | "full"
```

### 6. `qmd_status` - Index health check
```
No parameters - returns collection stats and index info
```

## Example Usage in Conversation

### Before (Using Bash):
```
You: "Search for error handling patterns"
Claude: [Uses Bash tool]
        qmd search "error handling" --json
```

### After (Using MCP):
```
You: "Search for error handling patterns"
Claude: [Uses qmd_search MCP tool directly]
        {
          query: "error handling",
          collection: "test_docs",
          limit: 10,
          output_format: "json"
        }
```

## Benefits of MCP Integration

✅ **Native Tool Access** - No bash wrapper needed
✅ **Structured Input/Output** - Type-safe parameters
✅ **Better Error Handling** - Claude knows when searches fail
✅ **Faster Execution** - Direct tool invocation
✅ **Auto-discovery** - Claude knows available collections

## Testing After Restart

Ask Claude to:
1. "Check qmd status" → Should show collections
2. "Search test_docs for 'rust error'" → Should return results
3. "Get the rust-error-handling tutorial" → Should retrieve full doc

## Current Collections

Based on your QMD setup, you have:
- **test_docs** - 7 markdown files (tutorials, API docs, troubleshooting)

Add more collections with:
```bash
qmd collection add ~/your-docs --name my_docs --mask "**/*.md"
qmd embed  # Generate embeddings for semantic search
```

## Configuration Status

✅ `.mcp.json` - Configured with absolute path
✅ `settings.json` - qmd server enabled
✅ `qmd` binary - Installed at `/Users/f/.bun/bin/qmd`
⏳ **Restart Required** - MCP server will load on next startup

---

**Next Step:** Restart Claude Code, then try: `"Search my test docs for goroutines"`
