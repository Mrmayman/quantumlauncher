// QuantumLauncher TUI - Instance Settings Tab renderers

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::tui::app::{App, Instance, InstanceSettingsPage, InstanceSettingsTab};
use crate::tui::tabs::logs::render_instance_logs;

pub fn render_instance_settings_tab(f: &mut Frame, area: Rect, app: &mut App) {
    if let Some(instance_idx) = app.instance_settings_instance {
        if let Some(instance) = app.instances.get(instance_idx) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(area);

            render_instance_info_card(f, chunks[0], instance);

            let sub_tabs = vec![
                Line::from("Overview"),
                Line::from("Mods"),
                Line::from("Settings"),
                Line::from("Logs"),
            ];
            let selected_sub_tab = match app.instance_settings_tab {
                InstanceSettingsTab::Overview => 0,
                InstanceSettingsTab::Mod => 1,
                InstanceSettingsTab::Setting => 2,
                InstanceSettingsTab::Logs => 3,
            };
            let sub_tabs_widget = Tabs::new(sub_tabs)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Instance Management ")
                        .title_style(Style::default().fg(Color::Cyan).bold()),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .bold()
                        .bg(Color::DarkGray),
                )
                .select(selected_sub_tab);
            f.render_widget(sub_tabs_widget, chunks[1]);

            match app.instance_settings_tab {
                InstanceSettingsTab::Overview => {
                    render_instance_overview(f, chunks[2], app.instance_settings_selected, instance)
                }
                InstanceSettingsTab::Mod => render_instance_mods(f, chunks[2], &instance.name),
                InstanceSettingsTab::Setting => match app.instance_settings_page {
                    InstanceSettingsPage::List => render_instance_settings(
                        f,
                        chunks[2],
                        app.instance_settings_selected,
                        instance,
                    ),
                    InstanceSettingsPage::Java => {
                        render_instance_java_settings(f, chunks[2], app, instance)
                    }
                    InstanceSettingsPage::Launch => {
                        render_instance_launch_settings(f, chunks[2], app, instance)
                    }
                },
                InstanceSettingsTab::Logs => {
                    let instance_name = instance.name.clone();
                    render_instance_logs(f, chunks[2], app, &instance_name)
                }
            }
        }
    }
}

fn render_instance_info_card(f: &mut Frame, area: Rect, instance: &Instance) {
    let status_color = if instance.is_running {
        Color::Green
    } else {
        Color::Gray
    };
    let status_text = if instance.is_running {
        "● RUNNING"
    } else {
        "○ STOPPED"
    };

    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Instance Details ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .border_style(Style::default().fg(Color::Blue));

    let name_text = Paragraph::new(Line::from(vec![
        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
        Span::styled(&instance.name, Style::default().fg(Color::Cyan).bold()),
    ]))
    .alignment(ratatui::layout::Alignment::Left);

    let version_text = Paragraph::new(Line::from(vec![
        Span::raw("  Minecraft Version: "),
        Span::styled(&instance.version, Style::default().fg(Color::Green).bold()),
    ]))
    .alignment(ratatui::layout::Alignment::Left);

    let loader_text = Paragraph::new(Line::from(vec![
        Span::raw("  Mod Loader: "),
        Span::styled(&instance.loader, Style::default().fg(Color::Yellow).bold()),
    ]))
    .alignment(ratatui::layout::Alignment::Left);

    let status_text_widget = Paragraph::new(Line::from(vec![
        Span::raw("  Status: "),
        Span::styled(status_text, Style::default().fg(status_color).bold()),
    ]))
    .alignment(ratatui::layout::Alignment::Left);

    let help_text = Paragraph::new(Line::from(vec![
        Span::styled("  ← → ", Style::default().fg(Color::Gray)),
        Span::raw("Switch tabs   "),
        Span::styled("↑ ↓ ", Style::default().fg(Color::Gray)),
        Span::raw("Navigate   "),
        Span::styled("Enter ", Style::default().fg(Color::Gray)),
        Span::raw("Select   "),
        Span::styled("Esc ", Style::default().fg(Color::Gray)),
        Span::raw("Back"),
    ]))
    .alignment(ratatui::layout::Alignment::Left);

    f.render_widget(block, area);
    f.render_widget(name_text, card_chunks[0]);
    f.render_widget(version_text, card_chunks[2]);
    f.render_widget(loader_text, card_chunks[3]);
    f.render_widget(status_text_widget, card_chunks[4]);
    f.render_widget(help_text, card_chunks[5]);
}

fn render_instance_overview(f: &mut Frame, area: Rect, selected_index: usize, instance: &Instance) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_instance_actions(f, main_chunks[0], selected_index, instance);
    render_instance_info_panel(f, main_chunks[1]);
}

fn render_instance_actions(f: &mut Frame, area: Rect, selected_index: usize, instance: &Instance) {
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .title(" Quick Actions ")
        .title_style(Style::default().fg(Color::Green).bold())
        .border_style(Style::default().fg(Color::Green));

    let actions_items = vec![
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(
                    "  ▶ ",
                    if instance.is_running {
                        Style::default().fg(Color::DarkGray).bold()
                    } else {
                        Style::default().fg(Color::Green).bold()
                    },
                ),
                Span::styled(
                    "Launch Instance",
                    if instance.is_running {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White).bold()
                    },
                ),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    if instance.is_running {
                        "Instance is already running"
                    } else {
                        "Start playing this instance"
                    },
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ])
        .style(if selected_index == 0 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(
                    "  ⏹ ",
                    if instance.is_running {
                        Style::default().fg(Color::Red).bold()
                    } else {
                        Style::default().fg(Color::DarkGray).bold()
                    },
                ),
                Span::styled(
                    "Force Stop",
                    if instance.is_running {
                        Style::default().fg(Color::White).bold()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    if instance.is_running {
                        "Kill running instance process"
                    } else {
                        "Instance not running"
                    },
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ])
        .style(if selected_index == 1 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  Folder ", Style::default().fg(Color::Blue).bold()),
                Span::styled("Open Folder", Style::default().fg(Color::White).bold()),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    "Browse instance directory",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ])
        .style(if selected_index == 2 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
    ];

    let actions_list = List::new(actions_items)
        .block(actions_block)
        .highlight_symbol("  ");
    f.render_widget(actions_list, area);
}

fn render_instance_info_panel(f: &mut Frame, area: Rect) {
    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let stats_block = Block::default()
        .borders(Borders::ALL)
        .title(" Instance Statistics ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .border_style(Style::default().fg(Color::Cyan));

    let stats_content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Stats: Total Playtime: ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("2h 45m", Style::default().fg(Color::White).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Last:  Last Played: ", Style::default().fg(Color::Yellow)),
            Span::styled("Never", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Size: Instance Size: ", Style::default().fg(Color::Yellow)),
            Span::styled("~847 MB", Style::default().fg(Color::White).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Created:  Created: ", Style::default().fg(Color::Yellow)),
            Span::styled("Today", Style::default().fg(Color::Green)),
        ]),
    ])
    .block(stats_block)
    .wrap(Wrap { trim: true });

    f.render_widget(stats_content, info_chunks[0]);

    let activity_block = Block::default()
        .borders(Borders::ALL)
        .title(" Quick Info ")
        .title_style(Style::default().fg(Color::Magenta).bold())
        .border_style(Style::default().fg(Color::Magenta));

    let activity_content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Performance: Performance: ",
                Style::default().fg(Color::Green),
            ),
            Span::styled("Optimized", Style::default().fg(Color::Green).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Java: Java Version: ", Style::default().fg(Color::Blue)),
            Span::styled("Auto-detected", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Mode: Game Mode: ", Style::default().fg(Color::Yellow)),
            Span::styled("Survival", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Network: Multiplayer: ", Style::default().fg(Color::Cyan)),
            Span::styled("Ready", Style::default().fg(Color::Green)),
        ]),
    ])
    .block(activity_block)
    .wrap(Wrap { trim: true });

    f.render_widget(activity_content, info_chunks[1]);
}

fn render_instance_mods(f: &mut Frame, area: Rect, _instance_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let mods_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Installed Mods ({}) ", 0))
        .title_style(Style::default().fg(Color::Green).bold())
        .border_style(Style::default().fg(Color::Green));

    let mods_content = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Java: Mod Management Coming Soon!",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(""),
    ])
    .block(mods_block)
    .wrap(Wrap { trim: true });

    f.render_widget(mods_content, chunks[0]);

    let mod_info_block = Block::default()
        .borders(Borders::ALL)
        .title(" Mod Actions / Info ")
        .title_style(Style::default().fg(Color::Blue).bold())
        .border_style(Style::default().fg(Color::Blue));

    let mod_info_content = Paragraph::new(vec![
        Line::from(""),
        Line::from("Install/Remove mods (coming soon)"),
        Line::from(""),
    ])
    .block(mod_info_block)
    .wrap(Wrap { trim: true });

    f.render_widget(mod_info_content, chunks[1]);
}

fn render_instance_settings(f: &mut Frame, area: Rect, selected_index: usize, instance: &Instance) {
    let settings_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Settings for {} ", instance.name))
        .title_style(Style::default().fg(Color::Magenta).bold())
        .border_style(Style::default().fg(Color::Magenta));

    let settings_items = vec![
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  [R] ", Style::default().fg(Color::Green).bold()),
                Span::styled("Rename Instance", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![Span::styled(
                "    Change the name of this instance",
                Style::default().fg(Color::Gray),
            )]),
        ])
        .style(if selected_index == 0 {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        }),
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  [J] ", Style::default().fg(Color::Blue).bold()),
                Span::styled("Java Settings", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![Span::styled(
                "    Configure JVM and memory",
                Style::default().fg(Color::Gray),
            )]),
        ])
        .style(if selected_index == 1 {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        }),
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  [L] ", Style::default().fg(Color::Cyan).bold()),
                Span::styled("Launch Options", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![Span::styled(
                "    Game arguments and JVM flags",
                Style::default().fg(Color::Gray),
            )]),
        ])
        .style(if selected_index == 2 {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        }),
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  🗑 ", Style::default().fg(Color::Red).bold()),
                Span::styled("Delete Instance", Style::default().fg(Color::Red).bold()),
            ]),
            Line::from(vec![Span::styled(
                "    Permanently remove this instance",
                Style::default().fg(Color::Gray),
            )]),
        ])
        .style(if selected_index == 3 {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        }),
    ];

    let settings_list = List::new(settings_items)
        .block(settings_block)
        .highlight_symbol("  ");
    f.render_widget(settings_list, area);
}

fn render_instance_java_settings(f: &mut Frame, area: Rect, app: &App, instance: &Instance) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Java Settings — {} ", instance.name))
        .title_style(Style::default().fg(Color::White).bold())
        .border_style(Style::default().fg(Color::Blue));

    let items = vec![
        // 0: Java executable override
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Custom Java executable (full path)",
                Style::default().fg(Color::Gray).bold(),
            )),
            Line::from(Span::styled(
                "    (placeholder)",
                Style::default().fg(Color::Gray),
            )),
        ])
        .style(if app.instance_settings_selected == 0 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        // 1: Java args interaction mode
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Interaction with global Java arguments (mode)",
                Style::default().fg(Color::Green).bold(),
            )),
            Line::from(Span::styled(
                format!("    {}", app.java_args_mode_current),
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                format!("    {}", app.java_args_mode_current.get_description()),
                Style::default().fg(Color::Green),
            )),
        ])
        .style(if app.instance_settings_selected == 1 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        // 2: Java arguments list
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Java arguments",
                Style::default().fg(Color::Green).bold(),
            )),
            Line::from(Span::styled(
                "    Press Enter to edit as text",
                Style::default().fg(Color::Green),
            )),
        ])
        .style(if app.instance_settings_selected == 2 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        // 3: Memory allocation
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Memory allocation (Xmx)",
                Style::default().fg(Color::Green).bold(),
            )),
            Line::from(Span::styled(
                format!("    Current: {} MB", app.memory_edit_mb),
                Style::default().fg(Color::Green),
            )),
        ])
        .style(if app.instance_settings_selected == 3 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
    ];

    let list = List::new(items).block(block).highlight_symbol("▶ ");
    f.render_widget(list, area);
}

fn render_instance_launch_settings(f: &mut Frame, area: Rect, app: &App, instance: &Instance) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Launch Options — {} ", instance.name))
        .title_style(Style::default().fg(Color::White).bold())
        .border_style(Style::default().fg(Color::Blue));

    let items = vec![
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Game arguments",
                Style::default().fg(Color::Green).bold(),
            )),
            Line::from(Span::styled(
                "    Press Enter to edit as text",
                Style::default().fg(Color::Green),
            )),
        ])
        .style(if app.instance_settings_selected == 0 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
        // 1: Custom Game Window Size (px)
        ListItem::new(vec![
            Line::from(Span::styled(
                "  Custom Game Window Size (px)",
                Style::default().fg(Color::Green).bold(),
            )),
            Line::from(Span::styled(
                "    Press Enter to set as WIDTH,HEIGHT (empty to reset)",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                "    Common: 854x480, 1366x768, 1920x1080, 2560x1440, 3840x2160",
                Style::default().fg(Color::Green),
            )),
        ])
        .style(if app.instance_settings_selected == 1 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        }),
    ];

    let list = List::new(items).block(block).highlight_symbol("▶ ");
    f.render_widget(list, area);
}
