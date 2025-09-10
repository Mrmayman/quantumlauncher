// QuantumLauncher TUI - Instances Tab (advanced)

use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Render the instances tab
pub fn render_instances_tab(f: &mut Frame, area: Rect, app: &mut App) {
    if app.instances.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Instances ");
        let paragraph = Paragraph::new(
            "No instances found.\nPress F5 to refresh or use Tab to navigate to Create tab to make a new instance.",
        )
        .block(block)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .instances
        .iter()
        .map(|instance| {
            let name_spans = vec![
                Span::styled(&instance.name, Style::default().fg(Color::Yellow).bold()),
                Span::raw(" "),
            ];

            ListItem::new(vec![
                Line::from(name_spans),
                Line::from(vec![
                    Span::raw("  Version: "),
                    Span::styled(&instance.version, Style::default().fg(Color::Cyan)),
                    Span::raw(" | Loader: "),
                    Span::styled(&instance.loader, Style::default().fg(Color::Green)),
                    Span::raw(" | Status: "),
                    Span::styled(
                        if instance.is_running {
                            "running"
                        } else {
                            "stopped"
                        },
                        Style::default().fg(if instance.is_running {
                            Color::Red
                        } else {
                            Color::Gray
                        }),
                    ),
                ]),
            ])
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_instance));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Instances (↑/↓ to navigate, Enter to launch) "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut list_state);
}
