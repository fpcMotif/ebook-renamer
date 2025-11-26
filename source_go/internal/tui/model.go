package tui

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/viewport"
	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/ebook-renamer/go/internal/duplicates"
	"github.com/ebook-renamer/go/internal/normalizer"
	"github.com/ebook-renamer/go/internal/scanner"
	"github.com/ebook-renamer/go/internal/todo"
	"github.com/ebook-renamer/go/internal/types"
)

type Step int

const (
	StepScan Step = iota
	StepNormalize
	StepCheckIntegrity
	StepDetectDuplicates
	StepWriteTodo
	StepExecute
	StepDone
)

type errMsg error

type Model struct {
	config    *types.Config
	state     Step
	spinner   spinner.Model
	viewport  viewport.Model
	err       error
	logs      []string

	// Data
	files           []*types.FileInfo
	normalized      []*types.FileInfo
	duplicateGroups [][]string
	cleanFiles      []*types.FileInfo
	todoList        *todo.TodoList
	filesToDelete   []string
}

func NewModel(config *types.Config) Model {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

	vp := viewport.New(80, 10)
	vp.Style = lipgloss.NewStyle().
		BorderStyle(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("62")).
		PaddingRight(2)

	return Model{
		config:   config,
		state:    StepScan,
		spinner:  s,
		viewport: vp,
		logs:     []string{"Starting ebook renamer..."},
	}
}

func (m Model) Init() tea.Cmd {
	return tea.Batch(
		m.spinner.Tick,
		m.scanCmd,
	)
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		if msg.String() == "q" || msg.String() == "ctrl+c" {
			return m, tea.Quit
		}
	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)
	case errMsg:
		m.err = msg
		m.logs = append(m.logs, fmt.Sprintf("Error: %v", msg))
		return m, tea.Quit
	case scanMsg:
		m.files = msg.files
		m.logs = append(m.logs, fmt.Sprintf("Found %d files", len(m.files)))
		m.state = StepNormalize
		cmds = append(cmds, m.normalizeCmd)
	case normalizeMsg:
		m.normalized = msg.normalized
		m.logs = append(m.logs, fmt.Sprintf("Normalized %d files", len(m.normalized)))
		m.state = StepCheckIntegrity
		cmds = append(cmds, m.checkIntegrityCmd)
	case checkIntegrityMsg:
		m.todoList = msg.todoList
		m.filesToDelete = msg.filesToDelete
		m.logs = append(m.logs, "Integrity check complete")
		m.state = StepDetectDuplicates
		cmds = append(cmds, m.detectDuplicatesCmd)
	case duplicatesMsg:
		m.duplicateGroups = msg.groups
		m.cleanFiles = msg.clean
		m.logs = append(m.logs, fmt.Sprintf("Detected %d duplicate groups", len(m.duplicateGroups)))
		m.state = StepWriteTodo
		cmds = append(cmds, m.writeTodoCmd)
	case writeTodoMsg:
		m.logs = append(m.logs, "Written todo.md")
		if m.config.DryRun {
			m.state = StepDone
			cmds = append(cmds, tea.Quit)
		} else {
			m.state = StepExecute
			cmds = append(cmds, m.executeCmd)
		}
	case executeMsg:
		m.logs = append(m.logs, "Execution complete")
		m.state = StepDone
		cmds = append(cmds, tea.Quit)
	}

	m.viewport.SetContent(strings.Join(m.logs, "\n"))
	m.viewport, cmd = m.viewport.Update(msg)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

func (m Model) View() string {
	if m.err != nil {
		return fmt.Sprintf("Error: %v\n", m.err)
	}

	s := "\n"
	s += titleStyle.Render("Ebook Renamer") + "\n\n"

	steps := []string{"Scanning", "Normalizing", "Checking Integrity", "Detecting Duplicates", "Writing Todo", "Executing"}
	for i, step := range steps {
		if Step(i) < m.state {
			s += fmt.Sprintf(" %s %s\n", checkMark, step)
		} else if Step(i) == m.state {
			s += fmt.Sprintf(" %s %s\n", m.spinner.View(), step)
		} else {
			s += fmt.Sprintf("   %s\n", statusStyle.Render(step))
		}
	}

	s += "\n"
	s += m.viewport.View()
	s += "\nPress q to quit.\n"

	return s
}

// Commands and Messages

type scanMsg struct {
	files []*types.FileInfo
}

func (m Model) scanCmd() tea.Msg {
	s, err := scanner.New(m.config.Path, m.config.MaxDepth)
	if err != nil {
		return errMsg(err)
	}
	files, err := s.Scan()
	if err != nil {
		return errMsg(err)
	}
	return scanMsg{files: files}
}

type normalizeMsg struct {
	normalized []*types.FileInfo
}

func (m Model) normalizeCmd() tea.Msg {
	normalized, err := normalizer.NormalizeFiles(m.files)
	if err != nil {
		return errMsg(err)
	}
	return normalizeMsg{normalized: normalized}
}

type checkIntegrityMsg struct {
	todoList      *todo.TodoList
	filesToDelete []string
}

func (m Model) checkIntegrityCmd() tea.Msg {
	todoFilePath := filepath.Join(m.config.Path, "todo.md")
	if m.config.TodoFile != nil {
		todoFilePath = *m.config.TodoFile
	}

	todoList, err := todo.New(todoFilePath, m.config.Path)
	if err != nil {
		return errMsg(err)
	}

	var incompleteDownloads []*types.FileInfo
	var corruptedFiles []*types.FileInfo
	var smallFiles []*types.FileInfo

	for _, fileInfo := range m.normalized {
		if fileInfo.IsFailedDownload {
			incompleteDownloads = append(incompleteDownloads, fileInfo)
		} else if fileInfo.IsTooSmall {
			smallFiles = append(smallFiles, fileInfo)
		} else {
			if strings.ToLower(fileInfo.Extension) == ".pdf" {
				if err := validatePDFHeader(fileInfo.OriginalPath); err != nil {
					corruptedFiles = append(corruptedFiles, fileInfo)
				}
			}
		}
	}

	shouldCleanup := m.config.AutoCleanup || m.config.DeleteSmall
	var filesToDelete []string

	// Process incomplete
	for _, fileInfo := range incompleteDownloads {
		if shouldCleanup {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else {
			todoList.AddFailedDownload(fileInfo)
		}
	}

	// Process corrupted
	for _, fileInfo := range corruptedFiles {
		if shouldCleanup {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else {
			todoList.AddFileIssue(fileInfo, types.FileIssueCorruptedPdf)
		}
	}

	// Process small
	for _, fileInfo := range smallFiles {
		if m.config.DeleteSmall {
			filesToDelete = append(filesToDelete, fileInfo.OriginalPath)
			todoList.RemoveFileFromTodo(fileInfo.OriginalName)
		} else {
			todoList.AddFailedDownload(fileInfo)
		}
	}

	// Analyze others
	for _, fileInfo := range m.normalized {
		isProblematic := false
		// Check if in any problematic list
		// (Simplified check for now)
		if !isProblematic {
			todoList.AnalyzeFileIntegrity(fileInfo)
		}
	}

	return checkIntegrityMsg{todoList: todoList, filesToDelete: filesToDelete}
}

type duplicatesMsg struct {
	groups [][]string
	clean  []*types.FileInfo
}

func (m Model) detectDuplicatesCmd() tea.Msg {
	groups, clean, err := duplicates.DetectDuplicates(m.normalized)
	if err != nil {
		return errMsg(err)
	}
	return duplicatesMsg{groups: groups, clean: clean}
}

type writeTodoMsg struct{}

func (m Model) writeTodoCmd() tea.Msg {
	if err := m.todoList.Write(); err != nil {
		return errMsg(err)
	}
	return writeTodoMsg{}
}

type executeMsg struct{}

func (m Model) executeCmd() tea.Msg {
	// Execute renames
	for _, fileInfo := range m.cleanFiles {
		if fileInfo.NewName != nil {
			if err := os.Rename(fileInfo.OriginalPath, fileInfo.NewPath); err != nil {
				return errMsg(err)
			}
		}
	}

	// Delete duplicates
	if !m.config.NoDelete {
		for _, group := range m.duplicateGroups {
			if len(group) > 1 {
				for i, path := range group {
					if i > 0 {
						if err := os.Remove(path); err != nil {
							// Log error but continue
						}
					}
				}
			}
		}
	}

	// Delete problematic files
	for _, path := range m.filesToDelete {
		if err := os.Remove(path); err != nil {
			// Log error
		}
	}

	return executeMsg{}
}

func validatePDFHeader(filePath string) error {
	file, err := os.Open(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	header := make([]byte, 5)
	_, err = file.Read(header)
	if err != nil {
		return err
	}

	if string(header) != "%PDF-" {
		return fmt.Errorf("invalid PDF header")
	}

	return nil
}
