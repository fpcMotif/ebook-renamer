package ui

import (
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/bubbles/progress"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// ProgressBar represents a styled progress bar
type ProgressBar struct {
	progress progress.Model
	current  int
	total    int
	label    string
	mu       sync.Mutex
}

// NewProgressBar creates a new progress bar
func NewProgressBar(total int, label string) *ProgressBar {
	p := progress.New(
		progress.WithDefaultGradient(),
		progress.WithWidth(40),
		progress.WithoutPercentage(),
	)
	p.FullColor = string(ColorPrimary)
	p.EmptyColor = string(ColorMuted)

	return &ProgressBar{
		progress: p,
		current:  0,
		total:    total,
		label:    label,
	}
}

// Increment increments the progress
func (pb *ProgressBar) Increment() {
	pb.mu.Lock()
	defer pb.mu.Unlock()
	if pb.current < pb.total {
		pb.current++
	}
}

// SetCurrent sets the current value
func (pb *ProgressBar) SetCurrent(n int) {
	pb.mu.Lock()
	defer pb.mu.Unlock()
	pb.current = n
}

// View returns the rendered progress bar
func (pb *ProgressBar) View() string {
	pb.mu.Lock()
	defer pb.mu.Unlock()

	percent := float64(pb.current) / float64(pb.total)
	if pb.total == 0 {
		percent = 0
	}

	bar := pb.progress.ViewAs(percent)
	countStr := CountStyle.Render(fmt.Sprintf("%d/%d", pb.current, pb.total))

	return fmt.Sprintf("%s %s %s", InfoStyle.Render(pb.label), bar, countStr)
}

// Spinner model for file operations
type SpinnerModel struct {
	spinner  spinner.Model
	message  string
	done     bool
	err      error
	quitting bool
}

type spinnerTickMsg time.Time
type spinnerDoneMsg struct{ err error }

// NewSpinner creates a new spinner with a message
func NewSpinner(message string) SpinnerModel {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(ColorSecondary)
	return SpinnerModel{
		spinner: s,
		message: message,
	}
}

func (m SpinnerModel) Init() tea.Cmd {
	return m.spinner.Tick
}

func (m SpinnerModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		if msg.String() == "q" || msg.String() == "ctrl+c" {
			m.quitting = true
			return m, tea.Quit
		}

	case spinnerDoneMsg:
		m.done = true
		m.err = msg.err
		return m, tea.Quit

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		return m, cmd
	}

	return m, nil
}

func (m SpinnerModel) View() string {
	if m.done {
		if m.err != nil {
			return RenderError(m.err.Error())
		}
		return RenderSuccess(m.message)
	}
	return fmt.Sprintf("%s %s", m.spinner.View(), InfoStyle.Render(m.message))
}

// OperationSummary displays a summary of operations
type OperationSummary struct {
	Renames          int
	Duplicates       int
	DuplicatesDeleted int
	SmallFiles       int
	CorruptedFiles   int
	FailedDownloads  int
	TodoItems        int
	Errors           int
}

// View returns the formatted summary
func (s *OperationSummary) View() string {
	var sb strings.Builder

	// Header
	sb.WriteString(TitleStyle.Render("📊 Operation Summary") + "\n")
	sb.WriteString(strings.Repeat("─", 40) + "\n\n")

	// Stats
	if s.Renames > 0 {
		sb.WriteString(fmt.Sprintf("  %s Files renamed:       %s\n",
			IconRename, RenderCount(s.Renames)))
	}

	if s.Duplicates > 0 {
		sb.WriteString(fmt.Sprintf("  %s Duplicates found:    %s\n",
			IconDuplicate, RenderCount(s.Duplicates)))
	}

	if s.DuplicatesDeleted > 0 {
		sb.WriteString(fmt.Sprintf("  %s Duplicates deleted:  %s\n",
			IconDelete, RenderCount(s.DuplicatesDeleted)))
	}

	if s.SmallFiles > 0 {
		sb.WriteString(fmt.Sprintf("  %s Small files:         %s\n",
			IconTiny, WarningStyle.Render(fmt.Sprintf("%d", s.SmallFiles))))
	}

	if s.CorruptedFiles > 0 {
		sb.WriteString(fmt.Sprintf("  %s Corrupted files:     %s\n",
			IconBroken, ErrorStyle.Render(fmt.Sprintf("%d", s.CorruptedFiles))))
	}

	if s.FailedDownloads > 0 {
		sb.WriteString(fmt.Sprintf("  %s Failed downloads:    %s\n",
			IconDownload, WarningStyle.Render(fmt.Sprintf("%d", s.FailedDownloads))))
	}

	if s.TodoItems > 0 {
		sb.WriteString(fmt.Sprintf("  %s Todo items:          %s\n",
			IconUncheck, InfoStyle.Render(fmt.Sprintf("%d", s.TodoItems))))
	}

	if s.Errors > 0 {
		sb.WriteString(fmt.Sprintf("  %s Errors:              %s\n",
			IconError, ErrorStyle.Render(fmt.Sprintf("%d", s.Errors))))
	}

	sb.WriteString("\n" + strings.Repeat("─", 40))

	return BoxStyle.Render(sb.String())
}

// FileTable displays a styled table of files
type FileTable struct {
	Headers []string
	Rows    [][]string
}

// NewFileTable creates a new file table
func NewFileTable(headers []string) *FileTable {
	return &FileTable{
		Headers: headers,
		Rows:    make([][]string, 0),
	}
}

// AddRow adds a row to the table
func (t *FileTable) AddRow(row ...string) {
	t.Rows = append(t.Rows, row)
}

// View renders the table
func (t *FileTable) View() string {
	if len(t.Rows) == 0 {
		return MutedStyle.Render("(no items)")
	}

	// Calculate column widths
	widths := make([]int, len(t.Headers))
	for i, h := range t.Headers {
		widths[i] = len(h)
	}
	for _, row := range t.Rows {
		for i, cell := range row {
			if i < len(widths) && len(cell) > widths[i] {
				widths[i] = min(len(cell), 50) // Cap at 50 chars
			}
		}
	}

	var sb strings.Builder

	// Headers
	headerStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(ColorSecondary).
		BorderStyle(lipgloss.NormalBorder()).
		BorderBottom(true).
		BorderForeground(ColorMuted)

	var headerCells []string
	for i, h := range t.Headers {
		headerCells = append(headerCells, lipgloss.NewStyle().
			Width(widths[i]).
			Render(h))
	}
	sb.WriteString(headerStyle.Render(strings.Join(headerCells, "  ")))
	sb.WriteString("\n")

	// Rows
	for _, row := range t.Rows {
		var cells []string
		for i, cell := range row {
			if i < len(widths) {
				// Truncate if needed
				displayCell := cell
				if len(cell) > widths[i] {
					displayCell = cell[:widths[i]-3] + "..."
				}
				cells = append(cells, lipgloss.NewStyle().
					Width(widths[i]).
					Render(displayCell))
			}
		}
		sb.WriteString(strings.Join(cells, "  "))
		sb.WriteString("\n")
	}

	return sb.String()
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}


