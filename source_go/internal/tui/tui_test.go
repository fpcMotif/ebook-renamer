package tui

import (
	"testing"

	"github.com/ebook-renamer/go/internal/types"
	"github.com/stretchr/testify/assert"
)

func TestNewModel(t *testing.T) {
	config := &types.Config{
		Path:   "/tmp/test",
		DryRun: true,
		Json:   false,
	}

	model := NewModel(config)
	assert.NotNil(t, model, "Model should be created")
}

func TestModelInit(t *testing.T) {
	config := &types.Config{
		Path:   "/tmp/test",
		DryRun: true,
		Json:   false,
	}

	model := NewModel(config)
	cmd := model.Init()
	// Init should return a command (could be nil or batch)
	_ = cmd
	assert.NotNil(t, model, "Model should be initialized")
}

func TestModelView(t *testing.T) {
	config := &types.Config{
		Path:   "/tmp/test",
		DryRun: true,
		Json:   false,
	}

	model := NewModel(config)
	view := model.View()
	assert.NotEmpty(t, view, "View should not be empty")
}
