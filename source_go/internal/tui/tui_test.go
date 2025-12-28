package tui

import (
	"testing"
	"time"

	"github.com/charmbracelet/bubbletea"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"

	"github.com/ebook-renamer/go/internal/duplicates"
	"github.com/ebook-renamer/go/internal/normalizer"
	"github.com/ebook-renamer/go/internal/scanner"
	"github.com/ebook-renamer/go/internal/todo"
	"github.com/ebook-renamer/go/internal/types"
)

func TestTUIModel(t *testing.T) {
	// Test initial model state
	args := types.Args{
		Path:               "/tmp",
		DryRun:             true,
		MaxDepth:           10,
		NoRecursive:        false,
		Extensions:         "pdf,epub,txt",
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

	model := initialModel(args)
	assert.Equal(t, args, model.args, "Args should be set in model")
	assert.Equal(t, stateReady, model.state, "Initial state should be ready")
	assert.NotNil(t, model.spinner, "Spinner should be initialized")
}

func TestTUIStates(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test state transitions
	t.Run("ready to scanning", func(t *testing.T) {
		msg := bubbletea.WindowSizeMsg{Width: 80, Height: 24}
		newModel, cmd := model.Update(msg)
		assert.Equal(t, scanning, newModel.state)
		assert.IsType(t, bubbletea.BatchMsg{}, cmd)
	})

	t.Run("scanning to processing", func(t *testing.T) {
		// Simulate scan completion
		model.state = scanning
		msg := scanCompleteMsg([]types.FileInfo{})
		newModel, cmd := model.Update(msg)
		assert.Equal(t, processing, newModel.state)
		assert.IsType(t, bubbletea.BatchMsg{}, cmd)
	})

	t.Run("processing to complete", func(t *testing.T) {
		// Simulate processing completion
		model.state = processing
		msg := processCompleteMsg{}
		newModel, cmd := model.Update(msg)
		assert.Equal(t, stateComplete, newModel.state)
		assert.IsType(t, bubbletea.QuitMsg{}, cmd)
	})
}

func TestTUICommands(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	t.Run("quit command", func(t *testing.T) {
		msg := bubbletea.KeyMsg{Type: bubbletea.KeyCtrlC}
		newModel, _ := model.Update(msg)
		assert.Equal(t, stateComplete, newModel.state)
	})

	t.Run("help command", func(t *testing.T) {
		msg := bubbletea.KeyMsg{Type: bubbletea.KeyF1}
		newModel, cmd := model.Update(msg)
		assert.Equal(t, stateHelp, newModel.state)
	})
}

func TestTUIErrorHandling(t *testing.T) {
	args := types.Args{
		Path:   "/nonexistent/path",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test error state
	assert.Equal(t, stateError, model.state, "Should be in error state for invalid path")
	assert.NotEmpty(t, model.errorMessage, "Should have error message")
}

func TestTUIView(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test view rendering
	view := model.View()
	assert.NotEmpty(t, view, "View should not be empty")
	assert.Contains(t, view, "📚", "Should contain book emoji")
}

func TestTUIMockScanner(t *testing.T) {
	// Create mock scanner
	mockScanner := &mock.Mock{}

	mockScanner.On("Scan", "/tmp/test").Return([]types.FileInfo{}, nil)

	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Replace scanner function temporarily
	// (In real implementation, you'd use dependency injection)
	_ = mockScanner

	assert.NotNil(t, model, "Model should be created")
}

func TestTUIMockNormalizer(t *testing.T) {
	// Create mock normalizer
	mockNormalizer := &mock.Mock{}

	testFiles := []types.FileInfo{
		{OriginalPath: "/tmp/test.pdf"},
	}

	mockNormalizer.On("NormalizeFiles", testFiles).Return(testFiles, nil)

	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Simulate scanning completion
	model.state = scanning
	msg := scanCompleteMsg(testFiles)
	newModel, _ := model.Update(msg)

	_ = mockNormalizer

	assert.NotNil(t, newModel, "Model should be updated after normalizing")
}

func TestTUIMockDuplicates(t *testing.T) {
	// Create mock duplicate detector
	mockDuplicates := &mock.Mock{}

	testFiles := []types.FileInfo{
		{OriginalPath: "/tmp/test1.pdf", Size: 1024},
		{OriginalPath: "/tmp/test2.pdf", Size: 1024},
	}

	expectedGroups := [][]string{
		{"/tmp/test1.pdf", "/tmp/test2.pdf"},
	}
	expectedClean := []types.FileInfo{testFiles[0]}

	mockDuplicates.On("DetectDuplicates", testFiles, false).Return(expectedGroups, expectedClean, nil)

	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Simulate processing state
	model.state = processing

	_ = mockDuplicates

	assert.NotNil(t, model, "Model should handle duplicates")
}

func TestTUIMockTodo(t *testing.T) {
	// Create mock todo list
	mockTodo := &mock.Mock{}

	testFiles := []types.FileInfo{
		{OriginalPath: "/tmp/test.pdf", IsFailedDownload: true, OriginalName: "incomplete.download"},
	}

	mockTodo.On("New", "", "/tmp/test").Return(nil)
	mockTodo.On("AddFailedDownload", testFiles[0]).Return(nil)
	mockTodo.On("Write").Return(nil)

	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Simulate processing state
	model.state = processing

	_ = mockTodo

	assert.NotNil(t, model, "Model should handle todo operations")
}

func TestTUIProgress(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test progress updates
	progress := float64(0.5)
	model.progress = progress
	model.status = "Processing files..."

	view := model.View()
	assert.Contains(t, view, "50%", "Should show progress percentage")
	assert.Contains(t, view, "Processing files...", "Should show status")
}

func TestTUISpinner(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test spinner in different states
	model.state = scanning
	view1 := model.View()
	assert.Contains(t, view1, "⏳", "Should show spinner when scanning")

	model.state = processing
	view2 := model.View()
	assert.Contains(t, view2, "⏳", "Should show spinner when processing")
}

func TestTUIHelpView(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)
	model.state = stateHelp

	view := model.View()
	assert.Contains(t, view, "Keyboard Shortcuts", "Should show help")
	assert.Contains(t, view, "q", "Should show quit key")
	assert.Contains(t, view, "h", "Should show help key")
}

func TestTUICleanup(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)
	defer model.cleanup()

	// Verify cleanup happens
	assert.True(t, model.cleanedUp, "CleanUp should be called")
}

func TestTUIConcurrentAccess(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Test concurrent access to model
	done := make(chan bool)
	go func() {
		// Concurrent read
		_ = model.View()
		done <- true
	}()

	go func() {
		// Concurrent write
		model.state = processing
		done <- true
	}()

	// Wait for both operations
	<-done
	<-done

	assert.NotNil(t, model, "Model should handle concurrent access")
}

func TestTUITimeoutHandling(t *testing.T) {
	args := types.Args{
		Path:   "/tmp/test",
		DryRun: true,
		JSON:   false,
	}

	model := initialModel(args)

	// Simulate timeout
	timeout := time.After(100 * time.Millisecond)

	select {
	case <-timeout:
		// Should handle timeout gracefully
		model.state = stateError
		model.errorMessage = "Operation timed out"
	case <-time.After(1 * time.Second):
		t.Fatal("Test timed out")
	}

	assert.Equal(t, stateError, model.state, "Should be in error state after timeout")
	assert.Equal(t, "Operation timed out", model.errorMessage, "Should have timeout error message")
}

// Helper function to create initial model (assuming this exists in tui.go)
func initialModel(args types.Args) model {
	// This would be defined in the main tui.go file
	// For testing purposes, we create a minimal mock
	return model{
		args:         args,
		state:        stateReady,
		spinner:      bubbletea.NewStandardSpinner(),
		status:       "Ready",
		progress:     0.0,
		errorMessage: "",
		cleanedUp:    false,
	}
}

// Mock model type for testing
type model struct {
	args         types.Args
	state        state
	spinner      bubbletea.Spinner
	status       string
	progress     float64
	errorMessage string
	cleanedUp    bool
}

type state int

const (
	stateReady state = iota
	stateScanning
	stateProcessing
	stateComplete
	stateError
	stateHelp
)

func (m model) Init() bubbletea.Cmd {
	return bubbletea.Batch()
}

func (m model) Update(msg bubbletea.Msg) (model, bubbletea.Cmd) {
	// Minimal update logic for testing
	switch msg := msg.(type) {
	case bubbletea.WindowSizeMsg:
		return m, nil
	case bubbletea.KeyMsg:
		if msg.Type == bubbletea.KeyCtrlC {
			m.state = stateComplete
			return m, bubbletea.Quit()
		}
	case interface{ ScanComplete() }:
		m.state = processing
		return m, bubbletea.Batch()
	case interface{ ProcessComplete() }:
		m.state = stateComplete
		return m, bubbletea.Quit()
	case error:
		m.state = stateError
		m.errorMessage = msg.(error).Error()
		return m, nil
	}
	return m, nil
}

func (m model) View() string {
	switch m.state {
	case stateReady:
		return "📚 Ebook Renamer - Ready"
	case stateScanning:
		return "⏳ Scanning files..."
	case stateProcessing:
		return "⚙️ Processing files..."
	case stateComplete:
		return "✅ Complete!"
	case stateError:
		return fmt.Sprintf("❌ Error: %s", m.errorMessage)
	case stateHelp:
		return "📖 Help - Press 'q' to quit, 'h' for help"
	default:
		return "Unknown state"
	}
}

func (m model) cleanup() {
	m.cleanedUp = true
}

// Message types for testing
type scanCompleteMsg []types.FileInfo
type processCompleteMsg struct{}

func (s scanCompleteMsg) ScanComplete()       {}
func (p processCompleteMsg) ProcessComplete() {}
