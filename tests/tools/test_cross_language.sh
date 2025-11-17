#!/bin/bash

# Cross-language consistency test harness
# Tests Rust, Go, and Python implementations against the same test data

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
RUST_BINARY="$PROJECT_ROOT/target/debug/ebook_renamer"
GO_BINARY="$PROJECT_ROOT/source_go/ebook-renamer"
PYTHON_SCRIPT="$PROJECT_ROOT/source_py/ebook-renamer.py"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Cross-Language Consistency Test ===${NC}"

# Check if test directory provided
if [ $# -eq 0 ]; then
    echo -e "${RED}Error: Please provide a test directory${NC}"
    echo "Usage: $0 <test-directory>"
    exit 1
fi

TEST_DIR="$1"
OUTPUT_DIR="$PROJECT_ROOT/test_output"

if [ ! -d "$TEST_DIR" ]; then
    echo -e "${RED}Error: Test directory does not exist: $TEST_DIR${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "Testing directory: $TEST_DIR"
echo "Output directory: $OUTPUT_DIR"
echo

# Function to run implementation and capture JSON
run_implementation() {
    local name="$1"
    local cmd="$2"
    local output_file="$3"
    
    echo -e "${YELLOW}Running $name...${NC}"
    if eval "$cmd" > "$output_file" 2>/dev/null; then
        echo -e "${GREEN}✓ $name completed${NC}"
        return 0
    else
        echo -e "${RED}✗ $name failed${NC}"
        return 1
    fi
}

# Run Rust implementation
if [ -f "$RUST_BINARY" ]; then
    run_implementation "Rust" "\"$RUST_BINARY\" --dry-run --json \"$TEST_DIR\"" "$OUTPUT_DIR/rust_output.json"
    RUST_SUCCESS=$?
else
    echo -e "${RED}✗ Rust binary not found: $RUST_BINARY${NC}"
    RUST_SUCCESS=1
fi

# Run Go implementation
if [ -f "$GO_BINARY" ]; then
    run_implementation "Go" "\"$GO_BINARY\" --dry-run --json \"$TEST_DIR\"" "$OUTPUT_DIR/go_output.json"
    GO_SUCCESS=$?
else
    echo -e "${RED}✗ Go binary not found: $GO_BINARY${NC}"
    GO_SUCCESS=1
fi

# Run Python implementation
if [ -f "$PYTHON_SCRIPT" ]; then
    run_implementation "Python" "python3 \"$PYTHON_SCRIPT\" --dry-run --json \"$TEST_DIR\"" "$OUTPUT_DIR/python_output.json"
    PYTHON_SUCCESS=$?
else
    echo -e "${RED}✗ Python script not found: $PYTHON_SCRIPT${NC}"
    PYTHON_SUCCESS=1
fi

echo

# Compare outputs
if [ $RUST_SUCCESS -eq 1 ] || [ $GO_SUCCESS -eq 1 ]; then
    echo -e "${RED}Cannot compare outputs - some implementations failed${NC}"
    exit 1
fi

echo -e "${YELLOW}=== Comparing Outputs ===${NC}"

# Normalize outputs (remove trailing newlines for comparison)
normalize_output() {
    local file="$1"
    # Remove trailing newlines and normalize whitespace
    sed '$ s/[[:space:]]*$//' "$file" | \
    sed 's/[[:space:]]\+/ /g' | \
    tr -d '\n' > "${file}.normalized"
}

normalize_output "$OUTPUT_DIR/rust_output.json"
normalize_output "$OUTPUT_DIR/go_output.json"

# Compare Rust vs Go
if diff "$OUTPUT_DIR/rust_output.json.normalized" "$OUTPUT_DIR/go_output.json.normalized" > /dev/null; then
    echo -e "${GREEN}✓ Rust and Go outputs match${NC}"
else
    echo -e "${RED}✗ Rust and Go outputs differ${NC}"
    echo "Differences:"
    diff "$OUTPUT_DIR/rust_output.json" "$OUTPUT_DIR/go_output.json" || true
fi

# Compare with Python if available
if [ $PYTHON_SUCCESS -eq 0 ]; then
    normalize_output "$OUTPUT_DIR/python_output.json"
    
    if diff "$OUTPUT_DIR/rust_output.json.normalized" "$OUTPUT_DIR/python_output.json.normalized" > /dev/null; then
        echo -e "${GREEN}✓ Rust and Python outputs match${NC}"
    else
        echo -e "${RED}✗ Rust and Python outputs differ${NC}"
        echo "Differences:"
        diff "$OUTPUT_DIR/rust_output.json" "$OUTPUT_DIR/python_output.json" || true
    fi
fi

# Cleanup
rm -f "$OUTPUT_DIR"/*.normalized

echo
echo -e "${GREEN}=== Test Complete ===${NC}"
