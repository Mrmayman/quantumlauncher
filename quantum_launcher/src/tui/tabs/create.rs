// QuantumLauncher TUI - Create Instance Tab (advanced)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::App;

/// Render the create instance tab
pub fn render_create_tab(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Instance name input
            Constraint::Length(3), // Download assets checkbox
            Constraint::Length(3), // Create button
            Constraint::Min(0),    // Version list
        ])
        .margin(1)
        .split(area);

    // Instance name input
    let name_input = Paragraph::new(if app.new_instance_name.is_empty() {
        if app.is_editing_name {
            "Enter instance name... (editing)"
        } else {
            "Enter instance name... (press Ctrl+N to edit)"
        }
    } else {
        &app.new_instance_name
    })
    .style(if app.new_instance_name.is_empty() {
        Style::default().fg(Color::Gray)
    } else if app.is_editing_name {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(if app.is_editing_name {
                " Instance Name (Editing - press Esc to finish) "
            } else {
                " Instance Name (press Ctrl+N to edit) "
            }),
    );
    f.render_widget(name_input, chunks[0]);

    // Download assets checkbox
    let checkbox_text = if app.download_assets {
        "☑ Download assets? (Enables sound/music, but slower)"
    } else {
        "☐ Download assets? (Enables sound/music, but slower)"
    };
    let checkbox = Paragraph::new(checkbox_text)
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Options (press Ctrl+D to toggle) "),
        );
    f.render_widget(checkbox, chunks[1]);

    // Create button and status
    let can_create = !app.new_instance_name.is_empty() && !app.available_versions.is_empty();
    let button_text = if app.is_loading {
        "Creating instance... Please wait"
    } else if app.new_instance_name.is_empty() {
        "Press Enter to name instance"
    } else if can_create {
        "Press Enter to create instance"
    } else {
        "No version selected"
    };

    let button_style = if app.is_loading {
        Style::default().fg(Color::Yellow)
    } else if can_create {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Red)
    };

    let create_button = Paragraph::new(button_text)
        .style(button_style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Create Instance "),
        );
    f.render_widget(create_button, chunks[2]);

    // Version list - enhanced with better styling and live search
    if app.available_versions.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Minecraft Versions ")
            .title_style(Style::default().fg(Color::Cyan).bold());
        let paragraph = Paragraph::new(vec![
            Line::from("Loading versions..."),
            Line::from(""),
            Line::from("Press F5 to refresh"),
            Line::from(""),
            Line::from("Supported versions:"),
            Line::from("  • Release versions"),
            Line::from("  • Snapshot versions"),
            Line::from("  • Beta/Alpha versions"),
        ])
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .alignment(Alignment::Center);
        f.render_widget(paragraph, chunks[3]);
    } else {
        // Always use filtered_versions (respects both filters and search)
        let version_source = &app.filtered_versions;
        let sel_idx = app.selected_filtered_version;

        let items: Vec<ListItem> = version_source
            .iter()
            .enumerate()
            .map(|(i, version)| {
                let is_selected = i == sel_idx;
                let cat = crate::tui::app::App::classify_version(&version.name);
                let (type_label, type_color) = match cat {
                    crate::tui::app::VersionCategory::Snapshot => ("Snapshot", Color::Yellow),
                    crate::tui::app::VersionCategory::Beta => ("Beta", Color::Magenta),
                    crate::tui::app::VersionCategory::Alpha => ("Alpha", Color::LightMagenta),
                    crate::tui::app::VersionCategory::Release => ("Release", Color::Green),
                };

                ListItem::new(vec![Line::from(vec![
                    if is_selected {
                        Span::styled("▶ ", Style::default().fg(Color::Yellow).bold())
                    } else {
                        Span::raw("  ")
                    },
                    Span::styled(&version.name, Style::default().fg(Color::Cyan).bold()),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", type_label), Style::default().fg(type_color)),
                    if version.is_classic_server {
                        Span::styled(" (Server)", Style::default().fg(Color::Blue))
                    } else {
                        Span::raw("")
                    },
                ])])
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(sel_idx));

        let selected_version_name = if app.version_search_active {
            format!(
                " Minecraft Versions - Search: {} ({} matches) ",
                app.version_search_query,
                version_source.len()
            )
        } else if let Some(version) = version_source.get(sel_idx) {
            format!(" Minecraft Versions - Selected: {} ", version.name)
        } else {
            " Minecraft Versions  (Ctrl+S to search) ".to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(selected_version_name)
                    .title_style(Style::default().fg(Color::Cyan).bold())
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        // Draw filter toggles above the list as a small hint/status row
        let filter_line = Paragraph::new(vec![Line::from(vec![
            Span::styled("Filters: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("[R:{}] ", if app.filter_release { "on" } else { "off" }),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("[S:{}] ", if app.filter_snapshot { "on" } else { "off" }),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("[B:{}] ", if app.filter_beta { "on" } else { "off" }),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                format!("[A:{}] ", if app.filter_alpha { "on" } else { "off" }),
                Style::default().fg(Color::LightMagenta),
            ),
            Span::raw("  (F6:R, F7:S, F8:B, F9:A, F10:Reset)"),
        ])])
        .block(Block::default().borders(Borders::ALL).title(" Filters "))
        .style(Style::default().fg(Color::White));

        // Layout: add a small top area for filters
        let list_area = {
            let sub = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(chunks[3]);
            f.render_widget(filter_line, sub[0]);
            sub[1]
        };

        f.render_stateful_widget(list, list_area, &mut list_state);
    }
}
