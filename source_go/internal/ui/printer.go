package ui

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/ebook-renamer/go/internal/types"
)

// Printer handles all console output with rich styling
type Printer struct {
	out     io.Writer
	verbose bool
	json    bool
}

// NewPrinter creates a new printer
func NewPrinter(verbose, json bool) *Printer {
	return &Printer{
		out:     os.Stdout,
		verbose: verbose,
		json:    json,
	}
}

// Banner prints the application banner
func (p *Printer) Banner() {
	if p.json {
		return
	}

	banner := lipgloss.NewStyle().
		Bold(true).
		Foreground(ColorSecondary).
		Render(`
   ╔═══════════════════════════════════════╗
   ║      📚 Ebook Renamer v1.0           ║
   ║   Batch rename & organize ebooks      ║
   ╚═══════════════════════════════════════╝
`)
	fmt.Fprintln(p.out, banner)
}

// DryRunBanner prints the dry run mode banner
func (p *Printer) DryRunBanner() {
	if p.json {
		return
	}

	banner := lipgloss.NewStyle().
		Bold(true).
		Background(ColorWarning).
		Foreground(ColorDark).
		Padding(0, 2).
		Render("🔍 DRY RUN MODE - No changes will be made")

	fmt.Fprintln(p.out)
	fmt.Fprintln(p.out, banner)
	fmt.Fprintln(p.out)
}

// Section prints a section header
func (p *Printer) Section(title string) {
	if p.json {
		return
	}

	header := SectionStyle.Render(title)
	fmt.Fprintln(p.out, header)
}

// ScanStart prints scan start message
func (p *Printer) ScanStart(path string) {
	if p.json {
		return
	}

	fmt.Fprintln(p.out, InfoStyle.Render(fmt.Sprintf("%s Scanning: ", IconSearch))+
		FilePathStyle.Render(path))
}

// ScanComplete prints scan completion message
func (p *Printer) ScanComplete(count int) {
	if p.json {
		return
	}

	fmt.Fprintln(p.out, RenderSuccess(fmt.Sprintf("Found %s files to process",
		CountStyle.Render(fmt.Sprintf("%d", count)))))
}

// PrintRenames prints the rename operations
func (p *Printer) PrintRenames(files []*types.FileInfo) {
	if p.json {
		return
	}

	var renames []*types.FileInfo
	for _, f := range files {
		if f.NewName != nil {
			renames = append(renames, f)
		}
	}

	if len(renames) == 0 {
		fmt.Fprintln(p.out, MutedStyle.Render("  (no renames needed)"))
		return
	}

	p.Section(fmt.Sprintf("%s Files to Rename (%d)", IconRename, len(renames)))

	for i, f := range renames {
		if i >= 20 && !p.verbose {
			remaining := len(renames) - 20
			fmt.Fprintln(p.out, MutedStyle.Render(fmt.Sprintf("  ... and %d more", remaining)))
			break
		}

		oldName := filepath.Base(f.OriginalPath)
		newName := *f.NewName

		// Truncate long names
		maxLen := 40
		if len(oldName) > maxLen {
			oldName = oldName[:maxLen-3] + "..."
		}
		if len(newName) > maxLen {
			newName = newName[:maxLen-3] + "..."
		}

		fmt.Fprintf(p.out, "  %s %s %s %s\n",
			MutedStyle.Render(fmt.Sprintf("%3d.", i+1)),
			FilePathStyle.Render(oldName),
			ArrowStyle.Render(IconArrowRight),
			NewNameStyle.Render(newName))
	}
	fmt.Fprintln(p.out)
}

// PrintDuplicates prints duplicate groups
func (p *Printer) PrintDuplicates(groups [][]string, noDelete bool) {
	if p.json {
		return
	}

	if len(groups) == 0 {
		return
	}

	// Count actual duplicates
	totalDups := 0
	for _, g := range groups {
		if len(g) > 1 {
			totalDups += len(g) - 1
		}
	}

	if totalDups == 0 {
		return
	}

	action := "to delete"
	if noDelete {
		action = "found (no-delete mode)"
	}

	p.Section(fmt.Sprintf("%s Duplicate Files %s (%d)", IconDuplicate, action, totalDups))

	for i, group := range groups {
		if len(group) <= 1 {
			continue
		}

		if i >= 10 && !p.verbose {
			remaining := len(groups) - 10
			fmt.Fprintln(p.out, MutedStyle.Render(fmt.Sprintf("  ... and %d more groups", remaining)))
			break
		}

		fmt.Fprintln(p.out, SubtitleStyle.Render(fmt.Sprintf("  Group %d:", i+1)))
		for j, path := range group {
			filename := filepath.Base(path)
			if j == 0 {
				// Keep
				fmt.Fprintf(p.out, "    %s %s\n",
					KeepStyle.Render("KEEP  "),
					FilePathStyle.Render(filename))
			} else {
				// Delete
				fmt.Fprintf(p.out, "    %s %s\n",
					DeleteStyle.Render("DELETE"),
					MutedStyle.Render(filename))
			}
		}
	}
	fmt.Fprintln(p.out)
}

// PrintIssues prints problematic files
func (p *Printer) PrintIssues(incomplete, corrupted, small []*types.FileInfo) {
	if p.json {
		return
	}

	total := len(incomplete) + len(corrupted) + len(small)
	if total == 0 {
		fmt.Fprintln(p.out, RenderSuccess("No problematic files found"))
		return
	}

	p.Section(fmt.Sprintf("%s Problematic Files (%d)", IconWarning, total))

	if len(incomplete) > 0 {
		fmt.Fprintln(p.out, WarningStyle.Render(fmt.Sprintf("  %s Incomplete downloads: %d", IconDownload, len(incomplete))))
		p.printFileList(incomplete, 3)
	}

	if len(corrupted) > 0 {
		fmt.Fprintln(p.out, ErrorStyle.Render(fmt.Sprintf("  %s Corrupted files: %d", IconBroken, len(corrupted))))
		p.printFileList(corrupted, 3)
	}

	if len(small) > 0 {
		fmt.Fprintln(p.out, WarningStyle.Render(fmt.Sprintf("  %s Too small (< 1KB): %d", IconTiny, len(small))))
		p.printFileList(small, 3)
	}
	fmt.Fprintln(p.out)
}

func (p *Printer) printFileList(files []*types.FileInfo, max int) {
	for i, f := range files {
		if i >= max && !p.verbose {
			fmt.Fprintln(p.out, MutedStyle.Render(fmt.Sprintf("      ... and %d more", len(files)-max)))
			break
		}
		fmt.Fprintf(p.out, "      %s %s\n",
			MutedStyle.Render(IconDot),
			FilePathStyle.Render(f.OriginalName))
	}
}

// PrintTodoItems prints todo list items
func (p *Printer) PrintTodoItems(items []string) {
	if p.json {
		return
	}

	if len(items) == 0 {
		return
	}

	p.Section(fmt.Sprintf("%s Todo Items (%d)", IconUncheck, len(items)))

	for i, item := range items {
		if i >= 10 && !p.verbose {
			fmt.Fprintln(p.out, MutedStyle.Render(fmt.Sprintf("  ... and %d more items", len(items)-10)))
			break
		}
		fmt.Fprintf(p.out, "  %s %s\n",
			InfoStyle.Render(IconUncheck),
			item)
	}
	fmt.Fprintln(p.out)
}

// PrintDeleteList prints files to be deleted
func (p *Printer) PrintDeleteList(files []string) {
	if p.json {
		return
	}

	if len(files) == 0 {
		return
	}

	p.Section(fmt.Sprintf("%s Files to Delete (%d)", IconDelete, len(files)))

	for i, path := range files {
		if i >= 10 && !p.verbose {
			fmt.Fprintln(p.out, MutedStyle.Render(fmt.Sprintf("  ... and %d more", len(files)-10)))
			break
		}
		fmt.Fprintf(p.out, "  %s %s\n",
			DeleteStyle.Render("DELETE"),
			MutedStyle.Render(filepath.Base(path)))
	}
	fmt.Fprintln(p.out)
}

// PrintCleanupSummary prints cleanup result summary
func (p *Printer) PrintCleanupSummary(result *types.CleanupResult) {
	if p.json {
		return
	}

	total := len(result.DeletedIncomplete) + len(result.DeletedCorrupted) + len(result.DeletedSmall)
	if total == 0 && len(result.FailedDeletions) == 0 {
		return
	}

	p.Section(fmt.Sprintf("%s Cleanup Summary", IconClean))

	if len(result.DeletedIncomplete) > 0 {
		fmt.Fprintln(p.out, RenderSuccess(fmt.Sprintf("Deleted %d incomplete downloads",
			len(result.DeletedIncomplete))))
	}

	if len(result.DeletedCorrupted) > 0 {
		fmt.Fprintln(p.out, RenderSuccess(fmt.Sprintf("Deleted %d corrupted files",
			len(result.DeletedCorrupted))))
	}

	if len(result.DeletedSmall) > 0 {
		fmt.Fprintln(p.out, RenderSuccess(fmt.Sprintf("Deleted %d small files",
			len(result.DeletedSmall))))
	}

	if len(result.FailedDeletions) > 0 {
		fmt.Fprintln(p.out, RenderWarning(fmt.Sprintf("Failed to delete %d files:",
			len(result.FailedDeletions))))
		for _, fd := range result.FailedDeletions {
			fmt.Fprintf(p.out, "  %s %s: %s\n",
				MutedStyle.Render(IconDot),
				FilePathStyle.Render(filepath.Base(fd.Path)),
				ErrorStyle.Render(fd.Error))
		}
	}
	fmt.Fprintln(p.out)
}

// PrintSummary prints the operation summary
func (p *Printer) PrintSummary(summary *OperationSummary) {
	if p.json {
		return
	}

	fmt.Fprintln(p.out, summary.View())
}

// Success prints a success message
func (p *Printer) Success(msg string) {
	if p.json {
		return
	}
	fmt.Fprintln(p.out, RenderSuccess(msg))
}

// Warning prints a warning message
func (p *Printer) Warning(msg string) {
	if p.json {
		return
	}
	fmt.Fprintln(p.out, RenderWarning(msg))
}

// Error prints an error message
func (p *Printer) Error(msg string) {
	if p.json {
		return
	}
	fmt.Fprintln(p.out, RenderError(msg))
}

// Info prints an info message
func (p *Printer) Info(msg string) {
	if p.json {
		return
	}
	fmt.Fprintln(p.out, RenderInfo(msg))
}

// Divider prints a divider line
func (p *Printer) Divider() {
	if p.json {
		return
	}
	fmt.Fprintln(p.out, MutedStyle.Render(strings.Repeat("─", 50)))
}

// Done prints the completion message
func (p *Printer) Done() {
	if p.json {
		return
	}

	done := lipgloss.NewStyle().
		Bold(true).
		Foreground(ColorPrimary).
		Render(fmt.Sprintf("\n%s Operation completed successfully!", IconSuccess))

	fmt.Fprintln(p.out, done)
}

// TodoWritten prints todo.md written message
func (p *Printer) TodoWritten(path string, dryRun bool) {
	if p.json {
		return
	}

	mode := ""
	if dryRun {
		mode = " (dry-run mode)"
	}

	fmt.Fprintln(p.out, RenderSuccess(fmt.Sprintf("todo.md written to %s%s",
		FilePathStyle.Render(path), MutedStyle.Render(mode))))
}

