// QuantumLauncher TUI - Footer Widget

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Render the footer with status and help
pub fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(35), Constraint::Length(20)])
        .split(area);

    // Status message
    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[0]);

    // Default account info
    let account_info = if let Some(current_account) = &app.current_account {
        // Extract the base username without modifiers
        let display_name = if current_account.contains(" (elyby)") {
            current_account.replace(" (elyby)", "")
        } else if current_account.contains(" (littleskin)") {
            current_account.replace(" (littleskin)", "")
        } else {
            current_account.clone()
        };
        format!("Default: {}", display_name)
    } else {
        "No default account".to_string()
    };
    
    let account = Paragraph::new(account_info)
        .block(Block::default().borders(Borders::ALL).title(" Account "))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(account, chunks[1]);

    // Help/keybinds
    let help = Paragraph::new("'?' help | 'q' quit")
        .block(Block::default().borders(Borders::ALL).title(" Keys "))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
