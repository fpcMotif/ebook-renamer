use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::cli::Args;
use crate::{duplicates, normalizer, scanner, todo, download_recovery};

#[derive(Debug, Clone)]
pub enum AppEvent {
    ScanComplete(Vec<crate::scanner::FileInfo>),
    NormalizeComplete(Vec<crate::scanner::FileInfo>),
    CheckComplete,
    DuplicatesComplete(Vec<Vec<std::path::PathBuf>>),
    Error(String),
    Done,
}

struct App {
    title: String,
    logs: Vec<String>,
    progress: f64,
    state: String,
    done: bool,
}

impl App {
    fn new() -> App {
        App {
            title: "Ebook Renamer".to_string(),
            logs: vec!["Starting...".to_string()],
            progress: 0.0,
            state: "Initializing".to_string(),
            done: false,
        }
    }
}

pub fn run(args: Args) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Channel for events
    let (tx, rx) = mpsc::channel();
    let tx_worker = tx.clone();

    // Spawn worker thread
    thread::spawn(move || {
        if let Err(e) = run_process(args, tx_worker.clone()) {
            let _ = tx_worker.send(AppEvent::Error(e.to_string()));
        }
    });

    // Event loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if let Ok(event) = rx.try_recv() {
                match event {
                    AppEvent::ScanComplete(files) => {
                        app.logs.push(format!("Found {} files", files.len()));
                        app.progress = 0.2;
                        app.state = "Normalizing...".to_string();
                    }
                    AppEvent::NormalizeComplete(files) => {
                        app.logs.push(format!("Normalized {} files", files.len()));
                        app.progress = 0.4;
                        app.state = "Checking Integrity...".to_string();
                    }
                    AppEvent::CheckComplete => {
                        app.logs.push("Integrity check complete".to_string());
                        app.progress = 0.6;
                        app.state = "Detecting Duplicates...".to_string();
                    }
                    AppEvent::DuplicatesComplete(groups) => {
                        app.logs.push(format!("Detected {} duplicate groups", groups.len()));
                        app.progress = 0.8;
                        app.state = "Executing...".to_string();
                    }
                    AppEvent::Error(msg) => {
                        app.logs.push(format!("Error: {}", msg));
                        app.state = "Error".to_string();
                    }
                    AppEvent::Done => {
                        app.logs.push("Done!".to_string());
                        app.progress = 1.0;
                        app.state = "Completed".to_string();
                        app.done = true;
                    }
                }
            }
            last_tick = Instant::now();
        }
        
        if app.done {
             // Optional: auto-quit or wait for q
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_process(mut args: Args, tx: mpsc::Sender<AppEvent>) -> Result<()> {
    // Auto-detect cloud storage and enable skip_cloud_hash if not explicitly set
    if !args.skip_cloud_hash {
        if let Some(provider) = crate::cloud::is_cloud_storage_path(&args.path) {
            args.skip_cloud_hash = true;
            // Send log message about cloud mode
            let msg = format!("⚠️  Detected {} - using metadata-only mode", provider.name());
            tx.send(AppEvent::Error(msg))?;
        }
    }

    // 1. Recovery
    let recovery = download_recovery::DownloadRecovery::new(&args.path, args.cleanup_downloads);
    let _ = recovery.recover_downloads(); // Ignore errors for now or log them

    // 2. Scan
    let effective_max_depth = if args.no_recursive { 1 } else { args.max_depth };
    let mut scanner = scanner::Scanner::new(&args.path, effective_max_depth)?;
    let files = scanner.scan()?;
    tx.send(AppEvent::ScanComplete(files.clone()))?;

    // 3. Normalize
    let normalized = normalizer::normalize_files(files)?;
    tx.send(AppEvent::NormalizeComplete(normalized.clone()))?;

    // 4. Todo / Check
    let mut todo_list = todo::TodoList::new(&args.todo_file, &args.path)?;
    // ... (Simplified logic for TUI demo, ideally copy full logic)
    for file_info in &normalized {
        if !file_info.is_failed_download && !file_info.is_too_small {
             todo_list.analyze_file_integrity(file_info)?;
        }
    }
    tx.send(AppEvent::CheckComplete)?;

    // 5. Duplicates
    let (duplicate_groups, clean_files) = duplicates::detect_duplicates(normalized, args.skip_cloud_hash)?;
    tx.send(AppEvent::DuplicatesComplete(duplicate_groups.clone()))?;

    // 6. Execute
    if !args.dry_run {
        // Execute renames
        for file_info in &clean_files {
            if let Some(ref _new_name) = file_info.new_name {
                std::fs::rename(&file_info.original_path, &file_info.new_path)?;
            }
        }
        // Delete duplicates
        if !args.no_delete {
            for group in &duplicate_groups {
                if group.len() > 1 {
                    for (idx, path) in group.iter().enumerate() {
                        if idx > 0 {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
            }
        }
    }
    
    // Write todo
    todo_list.write()?;

    tx.send(AppEvent::Done)?;
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new(app.title.as_str())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(title, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(app.progress)
        .label(format!("{:.0}%", app.progress * 100.0));
    f.render_widget(gauge, chunks[1]);

    let logs: Vec<ListItem> = app.logs
        .iter()
        .rev()
        .map(|m| {
            let style = if m.starts_with("Error") {
                Style::default().fg(Color::Red)
            } else if m.starts_with("Done") {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(m, style)]))
        })
        .collect();
    
    let logs_list = List::new(logs)
        .block(Block::default().borders(Borders::ALL).title("Logs"));
    f.render_widget(logs_list, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_ui_render_rich_text() {
        // 1. Setup App state
        let mut app = App::new();
        app.logs.push("Starting...".to_string());
        app.logs.push("Error: Something failed".to_string());
        app.logs.push("Done!".to_string());
        app.progress = 1.0;

        // 2. Setup Test Backend (increased height to ensure list items fit)
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        // 3. Render
        terminal.draw(|f| ui(f, &app)).unwrap();

        // 4. Assertions
        let buffer = terminal.backend().buffer();

        // Debug output for failing tests
        println!("Buffer content:");
        for y in 0..buffer.area.height {
            let line_str = (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<String>();
            println!("{:2}: {}", y, line_str);
        }

        // Check for "Status" block title
        assert_area_contains_str(buffer, "Status");

        // Check for "Progress" block title
        assert_area_contains_str(buffer, "Progress");

        // Check for Logs
        // Note: logs are reversed in the UI code
        // Line 0: "Done!" (Green)
        // Line 1: "Error: Something failed" (Red)
        // Line 2: "Starting..." (Default)

        // Find the "Logs" area. Since layout is vertical chunks[2].
        // We look for the strings in the buffer and check their style.

        assert_line_style(buffer, "Done!", Color::Green);
        assert_line_style(buffer, "Error: Something failed", Color::Red);
        assert_line_style(buffer, "Starting...", Color::Reset);
    }

    fn assert_area_contains_str(buffer: &Buffer, s: &str) {
        let mut found = false;
        for y in 0..buffer.area.height {
            let line_str = (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<String>();
            if line_str.contains(s) {
                found = true;
                break;
            }
        }
        assert!(found, "Buffer does not contain '{}'", s);
    }

    fn assert_line_style(buffer: &Buffer, text: &str, expected_fg: Color) {
        // This is a simplified search. It finds the text and checks the style of the first char.
        let mut found = false;
        for y in 0..buffer.area.height {
            let line_len = buffer.area.width;
            let line_cells: Vec<_> = (0..line_len).map(|x| buffer.get(x, y)).collect();
            let line_str: String = line_cells.iter().map(|c| c.symbol()).collect();

            if let Some(idx) = line_str.find(text) {
                let cell = line_cells[idx];
                assert_eq!(cell.fg, expected_fg, "Text '{}' at y={} has wrong color. Expected {:?}, got {:?}", text, y, expected_fg, cell.fg);
                found = true;
                break;
            }
        }
        assert!(found, "Buffer does not contain text '{}'", text);
    }
}
