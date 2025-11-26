package ui

import (
	"fmt"

	"github.com/charmbracelet/lipgloss"
)

// Color Palette - Monokai-inspired theme
var (
	ColorPrimary   = lipgloss.Color("#A6E22E") // Green
	ColorSecondary = lipgloss.Color("#66D9EF") // Cyan
	ColorAccent    = lipgloss.Color("#F92672") // Magenta/Pink
	ColorWarning   = lipgloss.Color("#FD971F") // Orange
	ColorError     = lipgloss.Color("#F92672") // Red/Pink
	ColorMuted     = lipgloss.Color("#75715E") // Gray
	ColorHighlight = lipgloss.Color("#E6DB74") // Yellow
	ColorWhite     = lipgloss.Color("#F8F8F2") // White
	ColorDark      = lipgloss.Color("#272822") // Dark background
)

// Base Styles
var (
	// Title styles
	TitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorSecondary).
			MarginBottom(1)

	SubtitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorMuted)

	// Section header
	SectionStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorHighlight).
			BorderStyle(lipgloss.NormalBorder()).
			BorderBottom(true).
			BorderForeground(ColorMuted).
			MarginTop(1).
			MarginBottom(1)

	// Success message
	SuccessStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorPrimary)

	// Warning message
	WarningStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorWarning)

	// Error message
	ErrorStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorError)

	// Info message
	InfoStyle = lipgloss.NewStyle().
			Foreground(ColorSecondary)

	// Muted text
	MutedStyle = lipgloss.NewStyle().
			Foreground(ColorMuted)

	// File path style
	FilePathStyle = lipgloss.NewStyle().
			Foreground(ColorWhite)

	// New file name style
	NewNameStyle = lipgloss.NewStyle().
			Foreground(ColorSecondary).
			Bold(true)

	// Arrow style
	ArrowStyle = lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)

	// Delete style
	DeleteStyle = lipgloss.NewStyle().
			Foreground(ColorError).
			Strikethrough(true)

	// Keep style
	KeepStyle = lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Bold(true)

	// Count/number style
	CountStyle = lipgloss.NewStyle().
			Foreground(ColorHighlight).
			Bold(true)

	// Box styles
	BoxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorMuted).
			Padding(1, 2)

	// Summary box
	SummaryBoxStyle = lipgloss.NewStyle().
			Border(lipgloss.DoubleBorder()).
			BorderForeground(ColorSecondary).
			Padding(1, 2).
			MarginTop(1)

	// Badge styles
	BadgeSuccess = lipgloss.NewStyle().
			Background(ColorPrimary).
			Foreground(ColorDark).
			Bold(true).
			Padding(0, 1)

	BadgeWarning = lipgloss.NewStyle().
			Background(ColorWarning).
			Foreground(ColorDark).
			Bold(true).
			Padding(0, 1)

	BadgeError = lipgloss.NewStyle().
			Background(ColorError).
			Foreground(ColorWhite).
			Bold(true).
			Padding(0, 1)

	BadgeInfo = lipgloss.NewStyle().
			Background(ColorSecondary).
			Foreground(ColorDark).
			Bold(true).
			Padding(0, 1)
)

// Icons
const (
	IconSuccess     = "✓"
	IconError       = "✗"
	IconWarning     = "⚠"
	IconInfo        = "ℹ"
	IconFile        = "📄"
	IconFolder      = "📁"
	IconRename      = "✏️"
	IconDelete      = "🗑️"
	IconDuplicate   = "🔄"
	IconSearch      = "🔍"
	IconCheck       = "☑"
	IconUncheck     = "☐"
	IconArrowRight  = "→"
	IconArrowDouble = "⟹"
	IconDot         = "•"
	IconStar        = "★"
	IconSpinner     = "◐"
	IconBook        = "📚"
	IconDownload    = "⬇"
	IconBroken      = "💔"
	IconTiny        = "🔬"
	IconClean       = "🧹"
)

// Helper functions
func RenderSuccess(msg string) string {
	return SuccessStyle.Render(IconSuccess+" ") + msg
}

func RenderError(msg string) string {
	return ErrorStyle.Render(IconError+" ") + msg
}

func RenderWarning(msg string) string {
	return WarningStyle.Render(IconWarning+" ") + msg
}

func RenderInfo(msg string) string {
	return InfoStyle.Render(IconInfo+" ") + msg
}

func RenderFileRename(oldName, newName string) string {
	return FilePathStyle.Render(oldName) + " " +
		ArrowStyle.Render(IconArrowRight) + " " +
		NewNameStyle.Render(newName)
}

func RenderFileDelete(name string) string {
	return DeleteStyle.Render(name)
}

func RenderFileKeep(name string) string {
	return KeepStyle.Render(name)
}

func RenderCount(count int) string {
	return CountStyle.Render(fmt.Sprintf("%d", count))
}

