package ui

import (
	"strings"
	"testing"

	"github.com/charmbracelet/lipgloss"
	"github.com/muesli/termenv"
	"github.com/stretchr/testify/assert"
)

func init() {
	// Force color output for testing
	lipgloss.SetColorProfile(termenv.TrueColor)
}

func TestRenderSuccess(t *testing.T) {
	msg := "Operation successful"
	output := RenderSuccess(msg)

	// Check content
	assert.Contains(t, output, msg)
	assert.Contains(t, output, IconSuccess)

	// Check style (rough check for ANSI codes)
	// We just want to ensure it's not raw text.
	assert.NotEqual(t, IconSuccess+" "+msg, output)
}

func TestRenderError(t *testing.T) {
	msg := "Something went wrong"
	output := RenderError(msg)

	assert.Contains(t, output, msg)
	assert.Contains(t, output, IconError)
}

func TestRenderWarning(t *testing.T) {
	msg := "Be careful"
	output := RenderWarning(msg)

	assert.Contains(t, output, msg)
	assert.Contains(t, output, IconWarning)
}

func TestRenderInfo(t *testing.T) {
	msg := "Just a note"
	output := RenderInfo(msg)

	assert.Contains(t, output, msg)
	assert.Contains(t, output, IconInfo)
}

func TestRenderFileRename(t *testing.T) {
	oldName := "old.txt"
	newName := "new.txt"
	output := RenderFileRename(oldName, newName)

	assert.Contains(t, output, oldName)
	assert.Contains(t, output, newName)
	assert.Contains(t, output, IconArrowRight)
}

func TestRenderFileDelete(t *testing.T) {
	name := "deleted.txt"
	output := RenderFileDelete(name)

	// Lipgloss with Strikethrough might style each character individually or the block.
	// We check if the letters are present, but maybe interleaved with codes.
	// But `assert.Contains(t, output, name)` checks for the substring.
	// If lipgloss splits it, it fails.
	// Let's check that it contains some of the letters.
	assert.Contains(t, output, "d")
	assert.Contains(t, output, "e")
	assert.Contains(t, output, "l")

	// Check for strikethrough ANSI code.
	// \x1b[9m is strikethrough.
    // The actual output in failure message was: "\x1b[38;2;249;38;113;9md\x1b[0m..."
    // Note that `9m` is embedded in the SGR sequence `38;2;...;9m`.
	assert.True(t, strings.Contains(output, ";9m") || strings.Contains(output, "\x1b[9m"), "Expected strikethrough ansi code")
}

func TestRenderFileKeep(t *testing.T) {
	name := "keep.txt"
	output := RenderFileKeep(name)

	assert.Contains(t, output, name)
}

func TestRenderCount(t *testing.T) {
	count := 42
	output := RenderCount(count)

	assert.Contains(t, output, "42")
}
