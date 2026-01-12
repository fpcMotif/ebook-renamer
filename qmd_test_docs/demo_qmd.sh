#!/bin/bash
# QMD Demo Script - Practical Examples
# Run this to see qmd in action

set -e  # Exit on error

echo "=================================================="
echo "QMD DEMONSTRATION - Fast Markdown Search"
echo "=================================================="
echo ""

# 1. Basic keyword search
echo "1️⃣  BASIC KEYWORD SEARCH"
echo "   Finding documents about 'goroutine'..."
echo "   Command: qmd search \"goroutine\" -n 2"
echo ""
qmd search "goroutine" -n 2
echo ""
read -p "Press Enter to continue..."
echo ""

# 2. JSON output for AI processing
echo "2️⃣  JSON OUTPUT (for AI/LLM integration)"
echo "   Searching for 'authentication' in JSON format..."
echo "   Command: qmd search \"authentication\" --json -n 2"
echo ""
qmd search "authentication" --json -n 2 | jq '.'
echo ""
read -p "Press Enter to continue..."
echo ""

# 3. Collection filtering
echo "3️⃣  COLLECTION FILTERING"
echo "   Search only in 'test_docs' collection..."
echo "   Command: qmd search \"error\" -c test_docs --files"
echo ""
qmd search "error" -c test_docs --files
echo ""
read -p "Press Enter to continue..."
echo ""

# 4. Document retrieval
echo "4️⃣  DOCUMENT RETRIEVAL"
echo "   Getting full content of a specific document..."
echo "   Command: qmd get \"qmd://test_docs/tutorials/rust-error-handling.md\" -l 15"
echo ""
qmd get "qmd://test_docs/tutorials/rust-error-handling.md" -l 15
echo ""
read -p "Press Enter to continue..."
echo ""

# 5. Multi-get with glob pattern
echo "5️⃣  BATCH RETRIEVAL (multi-get)"
echo "   Getting all tutorial files..."
echo "   Command: qmd multi-get \"qmd://test_docs/tutorials/*.md\" -l 5 --json"
echo ""
qmd multi-get "qmd://test_docs/tutorials/*.md" -l 5 --json | jq '.[].title'
echo ""
read -p "Press Enter to continue..."
echo ""

# 6. Full document search
echo "6️⃣  FULL DOCUMENT SEARCH"
echo "   Finding 'duplicate' with full content..."
echo "   Command: qmd search \"duplicate\" --full -n 1"
echo ""
qmd search "duplicate" --full -n 1 | head -30
echo "   ... (truncated)"
echo ""
read -p "Press Enter to continue..."
echo ""

# 7. Advanced filtering
echo "7️⃣  ADVANCED FILTERING"
echo "   All matches above 0.5 score..."
echo "   Command: qmd search \"database\" --all --min-score 0.5 --files"
echo ""
qmd search "database" --all --min-score 0.5 --files
echo ""
read -p "Press Enter to continue..."
echo ""

# 8. Collection status
echo "8️⃣  INDEX STATUS"
echo "   Command: qmd status"
echo ""
qmd status
echo ""
read -p "Press Enter to continue..."
echo ""

# 9. List collection files
echo "9️⃣  LIST COLLECTION FILES"
echo "   Command: qmd ls test_docs"
echo ""
qmd ls test_docs
echo ""

echo "=================================================="
echo "DEMO COMPLETE!"
echo ""
echo "Try these commands yourself:"
echo "  • qmd search \"your query\" --json"
echo "  • qmd get \"qmd://test_docs/path/to/file.md\""
echo "  • qmd multi-get \"**/*.md\" --json"
echo ""
echo "After restarting Claude Code, use MCP tools:"
echo "  • qmd_search() - Fast keyword search"
echo "  • qmd_get() - Retrieve documents"
echo "  • qmd_query() - Hybrid search with reranking"
echo "=================================================="
