// QuantumLauncher TUI - Header Widget

use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        block::Title, Block, Borders, Tabs,
    },
    Frame,
};
use ql_core::LAUNCHER_VERSION_NAME;

use crate::tui::app::{App, TabId};

/// Render the header with tabs
pub fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let tabs = vec![
        Line::from("Instances"),
        Line::from("Create"),
        Line::from("Settings"),
        Line::from("Accounts"),
        Line::from("Logs"),
    ];

    let selected_tab = match app.current_tab {
        TabId::Instances => 0,
        TabId::Create => 1,
        TabId::Settings => 2,
        TabId::Accounts => 3,
        TabId::Logs => 4,
    };

    let tabs = Tabs::new(tabs)
        .block(Block::default().borders(Borders::ALL).title(
            Title::from(format!(" QuantumLauncher TUI {} ", LAUNCHER_VERSION_NAME))
                .alignment(ratatui::layout::Alignment::Center)
        ))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).bold())
        .select(selected_tab)
        .divider("│");

    f.render_widget(tabs, area);
}
