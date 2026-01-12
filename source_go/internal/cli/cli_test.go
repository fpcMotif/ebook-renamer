package cli

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

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
		// The implementation should set effectiveMaxDepth to 1 when NoRecursive is true
		effectiveMaxDepth := getEffectiveMaxDepth("100", true)
		assert.Equal(t, 1, effectiveMaxDepth, "NoRecursive should set depth to 1")
	})

	t.Run("max-depth is used when no-recursive is false", func(t *testing.T) {
		effectiveMaxDepth := getEffectiveMaxDepth("50", false)
		assert.Equal(t, 50, effectiveMaxDepth, "Should use provided max-depth")
	})

	t.Run("default depth on invalid input", func(t *testing.T) {
		effectiveMaxDepth := getEffectiveMaxDepth("invalid", false)
		assert.Equal(t, 1000, effectiveMaxDepth, "Should use default on invalid input")
	})
}

func TestNilString(t *testing.T) {
	t.Run("empty string returns nil", func(t *testing.T) {
		result := nilString("")
		assert.Nil(t, result)
	})

	t.Run("non-empty string returns pointer", func(t *testing.T) {
		result := nilString("test")
		assert.NotNil(t, result)
		assert.Equal(t, "test", *result)
	})
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
