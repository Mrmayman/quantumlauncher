// QuantumLauncher TUI - Logs Tab (advanced, migrated from ui.rs)

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use textwrap::Options as TwOptions;

/// Render the global logs tab with viewport and auto-follow behavior
pub fn render_logs_tab(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default().borders(Borders::ALL).title(" Game Logs ");

    // Compute inner content area for accurate width/height
    let inner = block.inner(area);
    let inner_height = inner.height.max(1);
    let inner_width = inner.width.max(1);
    app.logs_visible_lines = inner_height as usize; // visible ROWS

    // Hard-wrap all log lines to rows so scrolling is by rows, not raw lines
    let wrap_opts = TwOptions::new(inner_width as usize).break_words(true);
    let mut rows: Vec<String> = Vec::new();
    for l in &app.game_logs {
        if l.is_empty() {
            rows.push(String::new());
        } else {
            for seg in textwrap::wrap(l, wrap_opts.clone()) {
                rows.push(seg.into_owned());
            }
        }
    }

    let total_rows = rows.len();
    let end = total_rows;
    let max_offset = total_rows.saturating_sub(app.logs_visible_lines);
    if app.logs_offset > max_offset {
        app.logs_offset = max_offset;
    }
    let start = end.saturating_sub(app.logs_visible_lines + app.logs_offset);

    let log_text = if rows.is_empty() {
        vec![
            Line::from("No game logs to display."),
            Line::from(""),
            Line::from("Launch a Minecraft instance to see logs here."),
            Line::from(""),
            Line::from("Recent activity:"),
            Line::from(app.status_message.clone()),
        ]
    } else {
        rows[start..end]
            .iter()
            .cloned()
            .map(Line::from)
            .collect::<Vec<_>>()
    };

    let paragraph = Paragraph::new(log_text).block(block);
    f.render_widget(paragraph, area);
}

/// Render the per-instance logs tab with viewport and auto-follow
pub fn render_instance_logs(f: &mut Frame, area: Rect, app: &mut App, instance_name: &str) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::text::Span;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Log controls
            Constraint::Min(0),    // Log viewer
        ])
        .split(area);

    // Top: Log controls
    let controls_block = Block::default()
        .borders(Borders::ALL)
        .title(" Log Controls ")
        .border_style(Style::default().fg(Color::Green));

    let controls_content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::Yellow)),
            Span::raw("All  "),
            Span::styled("Export  ", Style::default().fg(Color::Blue)),
            Span::styled("Clear  ", Style::default().fg(Color::Red)),
            Span::styled("Live  ", Style::default().fg(Color::Green)),
            Span::styled("Pause", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Scroll: ", Style::default().fg(Color::Yellow)),
            Span::raw("↑/↓, PageUp/PageDown, Home/End, Mouse wheel | "),
            Span::styled("Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw("Auto-follow at bottom; scroll up to pause"),
        ]),
    ])
    .block(controls_block);

    f.render_widget(controls_content, chunks[0]);

    // Bottom: Log viewer
    let logs_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Logs for {} ", instance_name))
        .border_style(Style::default().fg(Color::Cyan));

    // Compute inner rect and visible rows/width
    let inner = logs_block.inner(chunks[1]);
    let inner_height = inner.height.max(1);
    let inner_width = inner.width.max(1);
    app.instance_logs_visible_lines = inner_height as usize; // visible ROWS

    // Determine which slice of the instance buffer to show based on offset (row-based)
    let content_lines: Vec<Line> = if let Some(buf) = app.instance_logs.get(instance_name) {
        let wrap_opts = TwOptions::new(inner_width as usize).break_words(true);
        let mut rows: Vec<String> = Vec::new();
        for l in buf.iter() {
            if l.is_empty() {
                rows.push(String::new());
            } else {
                for seg in textwrap::wrap(l, wrap_opts.clone()) {
                    rows.push(seg.into_owned());
                }
            }
        }
        let total_rows = rows.len();
        let end = total_rows;
        let max_offset = total_rows.saturating_sub(app.instance_logs_visible_lines);
        if app.instance_logs_offset > max_offset {
            app.instance_logs_offset = max_offset;
        }
        let start = end.saturating_sub(app.instance_logs_visible_lines + app.instance_logs_offset);
        if total_rows == 0 {
            vec![
                Line::from(""),
                Line::from("No logs yet. Launch the instance to see output here."),
            ]
        } else {
            rows[start..end].iter().cloned().map(Line::from).collect()
        }
    } else {
        vec![
            Line::from(""),
            Line::from("No logs yet. Launch the instance to see output here."),
        ]
    };

    let logs_content = Paragraph::new(content_lines).block(logs_block);

    f.render_widget(logs_content, chunks[1]);
}
