// QuantumLauncher TUI - Settings Tab (advanced, includes About & Licenses)

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, SettingsFocus};

/// Render the settings tab
pub fn render_settings_tab(f: &mut Frame, area: Rect, app: &mut App) {
    // Triple-pane layout: Left (categories), Middle (submenu for Licenses), Right (details)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32),
            Constraint::Length(42),
            Constraint::Min(0),
        ])
        .split(area);

    // Left categories with combined About & Licenses
    let left_items = vec![
        ListItem::new("General"),
        ListItem::new("Java"),
        ListItem::new("UI / Theme"),
        ListItem::new("Launch"),
        ListItem::new("About & Licenses"),
    ];
    let mut left_state = ListState::default();
    left_state.select(Some(app.about_selected));
    let left_block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings ")
        .border_style(Style::default().fg(Color::Cyan));
    let left_list = List::new(left_items)
        .block(left_block)
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black).bold())
        .highlight_symbol("▸ ");
    f.render_stateful_widget(left_list, chunks[0], &mut left_state);

    // Middle pane: Licenses submenu when selected
    let is_licenses = app.about_selected == App::licenses_menu_index();
    let middle_block = Block::default()
        .borders(Borders::ALL)
        .title(" Submenu ")
        .border_style(if app.settings_focus == SettingsFocus::Middle {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    if is_licenses {
        let mut items: Vec<ListItem> = Vec::new();
        // First entry: About
        let mut about_entry = String::from("  About");
        if app.license_selected == 0 {
            about_entry.push_str("  ←");
        }
        items.push(ListItem::new(about_entry));
        // Then license entries
        for (i, (name, _)) in App::licenses().iter().enumerate() {
            let mut s = String::new();
            s.push_str("  ");
            s.push_str(name);
            if i + 1 == app.license_selected {
                s.push_str("  ←");
            }
            items.push(ListItem::new(s));
        }
        let mut mid_state = ListState::default();
        mid_state.select(Some(app.license_selected));
        let submenu = List::new(items)
            .block(middle_block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(submenu, chunks[1], &mut mid_state);
    } else {
        // Show contextual submenu entries for certain sections
        if app.about_selected == 0 {
            // General: submenu items for Global Game Window Size and TUI Refresh Interval
            let items = vec![
                ListItem::new(vec![
                    Line::from(Span::styled(
                        "  Global Game Window Size",
                        Style::default().fg(Color::White).bold(),
                    )),
                    Line::from(Span::raw(
                        "    Press Enter to edit as WIDTH,HEIGHT (empty to clear)",
                    )),
                ]),
                ListItem::new(vec![
                    Line::from(Span::styled(
                        "  TUI Refresh Interval (ms)",
                        Style::default().fg(Color::White).bold(),
                    )),
                    Line::from(Span::raw(
                        "    Press Enter to edit number of milliseconds (e.g., 500)",
                    )),
                ]),
            ];
            let mut mid_state = ListState::default();
            mid_state.select(Some(app.general_selected));
            let list = List::new(items)
                .block(middle_block)
                .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(list, chunks[1], &mut mid_state);
        } else if app.about_selected == 1 {
            // Java: show a single entry for Global Java Arguments
            let items = vec![ListItem::new(vec![
                Line::from(Span::styled(
                    "  Global Java arguments",
                    Style::default().fg(Color::White).bold(),
                )),
                Line::from(Span::raw(
                    "    Press Enter to edit as text (comma-separated)",
                )),
            ])
            .style(if app.settings_focus == SettingsFocus::Middle {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            })];
            let list = List::new(items).block(middle_block).highlight_symbol("▶ ");
            f.render_widget(list, chunks[1]);
        } else {
            let para = Paragraph::new("Select an option on the left. Licenses will appear here.")
                .block(middle_block)
                .wrap(Wrap { trim: true });
            f.render_widget(para, chunks[1]);
        }
    }

    // Right content based on selected section
    let right = match app.about_selected {
        0 => {
            // General right pane: default overview if Left-focused; detailed help per selected item when Middle-focused
            let lines: Vec<Line> = if app.settings_focus == SettingsFocus::Middle {
                match app.general_selected {
                    0 => vec![
                        Line::from(Span::styled("Global Game Window Size", Style::default().fg(Color::White).bold())),
                        Line::from("Set a default WIDTH,HEIGHT for game window. Instances with no local size will use this."),
                        Line::from("Press Enter to edit as WIDTH,HEIGHT (empty to clear)."),
                        Line::from(""),
                        Line::from("Examples: 854,480  |  1366,768  |  1920,1080"),
                    ],
                    1 => vec![
                        Line::from(Span::styled("TUI Refresh Interval (ms)", Style::default().fg(Color::White).bold())),
                        Line::from("Controls how often the TUI forces a redraw to prevent stdout spam artifacts."),
                        Line::from("0 disables periodic refresh; lower = snappier updates, higher = lower CPU."),
                        Line::from("Default: 500 ms. Press Enter to edit; empty resets to default."),
                    ],
                    _ => vec![Line::from("Select an option in the middle pane.")],
                }
            } else {
                vec![
                    Line::from(Span::styled("General settings for TUI", Style::default().fg(Color::Cyan).bold())),
                    Line::from(""),
                    Line::from("Use Right to focus the Submenu, Up/Down to choose an option, Enter to edit."),
                    Line::from("Press '?' for help."),
                ]
            };
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" General "))
                .wrap(Wrap { trim: true })
        }
        1 => {
            // Global Java settings: allow editing global extra_java_args
            let lines = vec![
                Line::from(Span::styled("Global Java Settings", Style::default().fg(Color::Cyan).bold())),
                Line::from(""),
                Line::from("Edit global Java arguments used by all instances (combined depending on instance mode)."),
                Line::from("Press Enter to edit as comma-separated list. Quotes preserve spaces/commas."),
                Line::from(""),
                Line::from(Span::styled("Tip:", Style::default().fg(Color::Yellow))),
                Line::from(" - Mode per-instance controls how these global args interact (Combine/Disable/Fallback)."),
            ];
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Java "))
                .wrap(Wrap { trim: true })
        }
        2 => Paragraph::new(vec![
            Line::from("UI / Theme settings coming soon!"),
            Line::from(""),
        ])
        .block(Block::default().borders(Borders::ALL).title(" UI / Theme "))
        .wrap(Wrap { trim: true }),
        3 => Paragraph::new(vec![
            Line::from("Launch options coming soon!"),
            Line::from(""),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Launch "))
        .wrap(Wrap { trim: true }),
        4 => {
            if app.settings_focus == SettingsFocus::Left {
                let overview = vec![
                    Line::from(Span::styled(
                        "About & Licenses",
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from("Select 'About' or one of the licenses from the middle pane."),
                    Line::from(""),
                    Line::from("Project License: GNU GPL v3 (see LICENSE)"),
                ];
                Paragraph::new(overview)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" About & Licenses "),
                    )
                    .wrap(Wrap { trim: true })
            } else if app.license_selected == 0 {
                let about_lines: Vec<Line> = vec![
                    Line::from(Span::styled("QuantumLauncher", Style::default().fg(Color::Cyan).bold())),
                    Line::from("A simple, powerful Minecraft launcher."),
                    Line::from("") ,
                    Line::from("Upstream project: Mrmayman & contributors."),
                    Line::from("TUI subsystem made by Sreehari425."),
                    Line::from("Built with ratatui (https://ratatui.rs)."),
                    Line::from("") ,
                    Line::from("QuantumLauncher is free and open source software under the GNU GPLv3 License."),
                    Line::from("No warranty is provided for this software."),
                    Line::from("You're free to share, modify, and redistribute it under the same license."),
                    Line::from("If you like this launcher, consider sharing it with your friends."),
                    Line::from("Every new user motivates me to keep working on this :)"),
                    Line::from("") ,
                    Line::from("Source : https://github.com/Mrmayman/QuantumLauncher"),
                    Line::from("") ,
                ];

                let overview = Paragraph::new(about_lines)
                    .block(Block::default().borders(Borders::ALL).title(" About "))
                    .wrap(Wrap { trim: true });
                f.render_widget(overview, chunks[2]);
                Paragraph::new("").block(Block::default())
            } else {
                let idx0 = app.license_selected - 1;
                let idx = idx0.min(App::licenses().len().saturating_sub(1));
                let (name, candidates) = App::licenses()[idx];
                let mut content = String::new();
                let mut loaded = false;
                for p in candidates {
                    if let Ok(s) = std::fs::read_to_string(p) {
                        content = s;
                        loaded = true;
                        break;
                    }
                }
                if !loaded {
                    if let Some(fallback) = App::license_fallback_content(idx) {
                        content = fallback.to_string();
                        loaded = true;
                    }
                }
                if !loaded {
                    let mut not_found_msg = format!(
                        "{} file not found. Please ensure it is distributed with the program.",
                        name
                    );
                    not_found_msg.push_str("\n\nTried the following paths:\n");
                    for p in candidates.iter() {
                        not_found_msg.push_str(&format!(" - {}\n", p));
                    }
                    content = not_found_msg;
                }

                let mut lines = vec![
                    Line::from(Span::styled(
                        name.to_string(),
                        Style::default().fg(Color::Green).bold(),
                    )),
                    Line::from(""),
                ];
                for line in content.lines() {
                    lines.push(Line::from(line.to_string()));
                }
                Paragraph::new(lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" {} ", name)),
                    )
                    .wrap(Wrap { trim: true })
                    .scroll((app.about_scroll, 0))
            }
        }
        _ => Paragraph::new("").block(Block::default()),
    };

    let drew_about_stacked = app.about_selected == 4
        && app.settings_focus == SettingsFocus::Middle
        && app.license_selected == 0;
    if !drew_about_stacked {
        f.render_widget(right, chunks[2]);
    }
}
