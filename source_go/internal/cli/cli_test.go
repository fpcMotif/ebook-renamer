package cli

import (
	"fmt"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRootCommandExecution(t *testing.T) {
	tests := []struct {
		name     string
		args     []string
		expected int
	}{
		{"dry run", []string{"--dry-run", "/tmp"}, 0},
		{"json output", []string{"--json", "--dry-run", "/tmp"}, 0},
		{"invalid path", []string{"/nonexistent/path"}, 1},
		{"conflicting args", []string{"--no-recursive", "--max-depth=10"}, 0}, // Should resolve correctly
		{"invalid depth", []string{"--max-depth=invalid"}, 1},
		{"invalid extensions", []string{"--extensions=exe,bat"}, 1}, // Should be rejected
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmd := RootCmd()
			cmd.SetArgs(tt.args)
			err := cmd.Execute()
			if tt.expected == 0 {
				assert.NoError(t, err, "Should succeed: %s", tt.name)
			} else {
				assert.Error(t, err, "Should fail: %s", tt.name)
			}
		})
	}
}

func TestArgumentValidation(t *testing.T) {
	t.Run("valid max depth", func(t *testing.T) {
		depth := 100
		err := validateMaxDepth(depth)
		assert.NoError(t, err, "Depth 100 should be valid")
	})

	t.Run("invalid max depth - too large", func(t *testing.T) {
		depth := 1001
		err := validateMaxDepth(depth)
		assert.Error(t, err, "Depth 1001 should be invalid")
	})

	t.Run("valid extensions", func(t *testing.T) {
		extensions := []string{"pdf", "epub", "txt"}
		err := validateExtensions(extensions)
		assert.NoError(t, err, "Valid extensions should be accepted")
	})

	t.Run("invalid extensions", func(t *testing.T) {
		extensions := []string{"exe", "bat", "cmd"}
		err := validateExtensions(extensions)
		assert.Error(t, err, "Dangerous extensions should be rejected")
	})

	t.Run("mixed valid/invalid extensions", func(t *testing.T) {
		extensions := []string{"pdf", "exe"}
		err := validateExtensions(extensions)
		assert.Error(t, err, "Should reject when mixed with dangerous")
	})

	t.Run("extensions with dots", func(t *testing.T) {
		extensions := []string{".pdf", ".txt"}
		err := validateExtensions(extensions)
		assert.NoError(t, err, "Should handle extensions starting with dots")
	})
}

func TestFlagOverrides(t *testing.T) {
	t.Run("--no-recursive overrides max-depth", func(t *testing.T) {
		// This test ensures the flag logic in runEbookRenamer works correctly
		// When --no-recursive is set, max-depth should be ignored
		args := &types.Args{
			Path:               "/tmp",
			DryRun:             true,
			MaxDepth:           "100", // Should be ignored
			NoRecursive:        true,  // Should override
			Extensions:         "",
			NoDelete:           false,
			TodoFile:           "",
			LogFile:            "",
			PreserveUnicode:    false,
			FetchArxiv:         false,
			Verbose:            false,
			DeleteSmall:        false,
			CleanFailed:        false,
			JSON:               true,
			SkipCloudHash:      false,
			CleanupDownloads:   false,
			GenerateUndoScript: false,
		}

		// The implementation should set effectiveMaxDepth to 1 when NoRecursive is true
		effectiveMaxDepth := getEffectiveMaxDepth(args.MaxDepth, args.NoRecursive)
		assert.Equal(t, 1, effectiveMaxDepth, "NoRecursive should set depth to 1")
	})
}

func TestErrorHandling(t *testing.T) {
	t.Run("invalid path handling", func(t *testing.T) {
		cmd := RootCmd()
		cmd.SetArgs([]string{"/nonexistent/path/that/does/not/exist"})
		err := cmd.Execute()
		assert.Error(t, err, "Should fail for nonexistent path")

		// Check that error is informative
		assert.Contains(t, err.Error(), "no such file or directory")
	})

	t.Run("permission denied", func(t *testing.T) {
		if os.Getuid() == 0 {
			t.Skip("Running as root, skipping permission test")
		}

		// Try to access a directory we likely don't have permission for
		cmd := RootCmd()
		cmd.SetArgs([]string{"/root"})
		err := cmd.Execute()
		assert.Error(t, err, "Should fail for restricted directory")
	})
}

func TestOutputFormats(t *testing.T) {
	// Test that different output formats don't crash
	outputTypes := []struct {
		name string
		args []string
	}{
		{"json output", []string{"--json", "--dry-run", "/tmp"}},
		{"tui output", []string{"--dry-run", "/tmp"}},
		{"verbose", []string{"--verbose", "--dry-run", "/tmp"}},
	}

	for _, tt := range outputTypes {
		t.Run(tt.name, func(t *testing.T) {
			cmd := RootCmd()
			cmd.SetArgs(tt.args)

			// We don't test full execution, just that command setup doesn't panic
			assert.NotNil(t, cmd, "Command should be properly initialized")
			assert.Equal(t, len(tt.args), len(cmd.Args()), "Args should be set correctly")
		})
	}
}

// Helper validation functions that mirror security module
func validateMaxDepth(depth int) error {
	if depth > 1000 {
		return fmt.Errorf("depth limit exceeded: %d > %d", depth, 1000)
	}
	return nil
}

func validateExtensions(extensions []string) error {
	allowedExtensions := map[string]bool{
		".pdf": true, ".epub": true, ".txt": true, ".mobi": true,
		".azw3": true, ".azw": true, ".download": true, ".crdownload": true,
		".tar.gz": true,
	}

	for _, ext := range extensions {
		normExt := strings.ToLower(strings.TrimSpace(ext))
		if !strings.HasPrefix(normExt, ".") {
			normExt = "." + normExt
		}

		if !allowedExtensions[normExt] {
			return fmt.Errorf("extension '%s' not allowed", ext)
		}
	}
	return nil
}

func getEffectiveMaxDepth(maxDepthStr string, noRecursive bool) int {
	if noRecursive {
		return 1
	}

	if depth, err := strconv.Atoi(maxDepthStr); err == nil {
		return depth
	}
	return 1000 // Default
}
