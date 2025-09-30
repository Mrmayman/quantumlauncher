// QuantumLauncher TUI - UI Rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::tui::app::{App, TabId};
use crate::tui::tabs::accounts::render_accounts_tab;
use crate::tui::tabs::create::render_create_tab;
use crate::tui::tabs::instance_settings::render_instance_settings_tab;
use crate::tui::tabs::instances::render_instances_tab;
use crate::tui::tabs::logs::render_logs_tab;
use crate::tui::tabs::settings::render_settings_tab;

/// Main rendering function
pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs (consistent for all pages)
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer with status
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_main_content(f, chunks[1], app);
    render_footer(f, chunks[2], app);

    if app.is_loading {
        render_loading_popup(f);
    }

    if app.show_help_popup {
        render_help_popup(f, app);
    }

    if app.show_delete_confirm {
        render_delete_confirm_popup(f, app);
    }

    // Rename popup overlay
    if app.is_renaming_instance {
        render_rename_popup(f, app);
    }

    // Memory allocation editor popup
    if app.is_editing_memory {
        render_memory_edit_popup(f, app);
    }

    // Args editor popup
    if app.is_editing_args {
        render_args_edit_popup(f, app);
    }
}

/// Render the header with tabs
fn render_header(f: &mut Frame, area: Rect, app: &App) {
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
        TabId::InstanceSettings => 0,
    };

    let tabs_widget = Tabs::new(tabs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" QuantumLauncher "),
        )
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(selected_tab);
    f.render_widget(tabs_widget, area);
}

/// Render the main content area based on current tab
fn render_main_content(f: &mut Frame, area: Rect, app: &mut App) {
    match app.current_tab {
        TabId::Instances => render_instances_tab(f, area, app),
        TabId::Create => render_create_tab(f, area, app),
        TabId::Settings => render_settings_tab(f, area, app),
        TabId::Accounts => render_accounts_tab(f, area, app),
        TabId::Logs => render_logs_tab(f, area, app),
        TabId::InstanceSettings => render_instance_settings_tab(f, area, app),
    }
}

/// Render the footer with status and help
fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(35),
            Constraint::Length(20),
        ])
        .split(area);

    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[0]);

    let account_info = if let Some(current_account) = &app.current_account {
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

    let help = Paragraph::new("'?' help | 'q' quit")
        .block(Block::default().borders(Borders::ALL).title(" Keys "))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

/// Render loading popup
fn render_loading_popup(f: &mut Frame) {
    let area = centered_rect(30, 7, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("Loading...")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let loading_text = Paragraph::new("Please wait...")
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(loading_text, area);

    let progress_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width - 4,
        height: 1,
    };
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Yellow))
        .ratio(0.5);
    f.render_widget(gauge, progress_area);
}

/// Helper function to center a rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
/// Render confirmation popup for instance deletion
fn render_delete_confirm_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(50, 30, f.area());

    let block = Block::default()
        .title(" Confirm Instance Deletion ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).bold());

    let lines = vec![
        Line::from(""),
        Line::from("Are you sure you want to permanently delete this instance?"),
        Line::from(""),
        Line::from("This action cannot be undone!"),
        Line::from(""),
        Line::from("Press 'Y' to confirm, 'N' or Esc to cancel."),
        Line::from(""),
    ];

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(para, popup_area);
}

/// Render rename instance popup
fn render_rename_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, f.area());
    f.render_widget(Clear, area);

    let title = " Rename Instance ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).bold());

    // Compose simple instructions and current buffer text
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Type new name and press "),
            Span::styled("Enter", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" to rename instance"),
        ]),
        Line::from("Esc to cancel. Invalid characters will be removed."),
        Line::from(""),
        Line::from(vec![
            Span::styled("New name: ", Style::default().fg(Color::Green)),
            Span::raw(app.rename_input.clone()),
        ]),
        Line::from(""),
    ];

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

/// Render memory allocation popup
fn render_memory_edit_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Edit Memory Allocation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta).bold());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Enter RAM in MB or GB (e.g., "),
            Span::styled("2048", Style::default().fg(Color::Yellow)),
            Span::raw(", "),
            Span::styled("2G", Style::default().fg(Color::Yellow)),
            Span::raw(", "),
            Span::styled("4GB", Style::default().fg(Color::Yellow)),
            Span::raw(")"),
        ]),
        Line::from("Recommended: 2-3 GB for vanilla; 4-8 GB for heavy modpacks"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(Color::Green)),
            Span::raw(format!("{} MB", app.memory_edit_mb)),
        ]),
        Line::from(vec![
            Span::styled("New value: ", Style::default().fg(Color::Green)),
            Span::raw(app.memory_edit_input.clone()),
        ]),
        Line::from(""),
        Line::from("Type to edit, Enter to save, Esc to cancel, Backspace to delete"),
    ];

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(para, area);
}

/// Render Java/Game arguments editor popup
fn render_args_edit_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 45, f.area());
    f.render_widget(Clear, area);

    let title = match app.args_edit_kind {
        crate::tui::app::ArgsEditKind::Java => " Edit Java Arguments ",
        crate::tui::app::ArgsEditKind::Game => " Edit Game Arguments ",
        crate::tui::app::ArgsEditKind::GlobalJava => " Edit Global Java Arguments ",
        crate::tui::app::ArgsEditKind::WindowSize => " Edit Window Size ",
        crate::tui::app::ArgsEditKind::GlobalWindowSize => " Edit Global Window Size ",
        crate::tui::app::ArgsEditKind::GlobalTuiRefreshInterval => {
            " Edit TUI Refresh Interval (ms) "
        }
        // Pre-launch editors kept for enum completeness but not exposed in TUI
        crate::tui::app::ArgsEditKind::PreLaunchPrefixInstance => " Edit Arguments ",
        crate::tui::app::ArgsEditKind::PreLaunchPrefixGlobal => " Edit Arguments ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue).bold());

    let mut lines = vec![Line::from("")];
    match app.args_edit_kind {
        crate::tui::app::ArgsEditKind::WindowSize => {
            lines.push(Line::from(
                "Enter window size as WIDTH,HEIGHT (both integers).",
            ));
            lines.push(Line::from("Examples: 854,480  or  1920,1080"));
            lines.push(Line::from("Leave empty to reset to default (auto size)."));
        }
        crate::tui::app::ArgsEditKind::GlobalWindowSize => {
            lines.push(Line::from(
                "Enter GLOBAL window size as WIDTH,HEIGHT (both integers).",
            ));
            lines.push(Line::from("Instances without local size will use this."));
            lines.push(Line::from("Examples: 854,480  or  1920,1080"));
            lines.push(Line::from("Leave empty to clear (use Minecraft default)."));
        }
        crate::tui::app::ArgsEditKind::GlobalTuiRefreshInterval => {
            lines.push(Line::from(
                "Enter number of milliseconds between forced TUI redraws.",
            ));
            lines.push(Line::from(
                "0 disables periodic refresh; lower = snappier, higher = lower CPU.",
            ));
            lines.push(Line::from("Examples: 0  |  250  |  500  |  1000"));
            lines.push(Line::from("Leave empty to reset to default (500)."));
        }
        _ => {
            lines.push(Line::from("Enter arguments as a single line, comma-separated. Use quotes for spaces or commas."));
            lines.push(Line::from(
                "Examples: --demo,--width 854,--height 480  or  -Xms512M,-Xmx2G",
            ));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Args: ", Style::default().fg(Color::Green)),
        Span::raw(app.args_edit_input.clone()),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Type to edit, Enter to save, Esc to cancel, Backspace to delete",
    ));

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(para, area);
}

/// Render help popup with contextual controls based on current tab and state
fn render_help_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let help_text = get_contextual_help(app);

    let block = Block::default()
        .title("Help & Controls")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let help_paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, area);
}

/// Generate contextual help content based on current tab and state
fn get_contextual_help(app: &App) -> Vec<Line<'_>> {
    let mut help_text = vec![
        Line::from(vec![Span::styled(
            "QuantumLauncher TUI Controls",
            Style::default().fg(Color::Yellow).bold(),
        )]),
        Line::from(""),
    ];

    // Add tab-specific help based on current tab
    match app.current_tab {
        TabId::Instances => {
            help_text.extend(get_instances_help(app));
        }
        TabId::Create => {
            help_text.extend(get_create_help(app));
        }
        TabId::Settings => {
            help_text.extend(get_settings_help());
        }
        TabId::Accounts => {
            help_text.extend(get_accounts_help(app));
        }
        TabId::Logs => {
            help_text.extend(get_logs_help());
        }
        TabId::InstanceSettings => {
            help_text.extend(get_instance_settings_help(app));
        }
    }

    // Add common/global help
    help_text.extend(get_global_help());

    help_text.push(Line::from(""));
    help_text.push(Line::from(vec![Span::styled(
        "Press '?' or Esc to close this help",
        Style::default().fg(Color::Green).italic(),
    )]));

    help_text
}

// Help for About tab
// About help removed; integrated into Settings help

/// Help for Instances tab
fn get_instances_help(_app: &App) -> Vec<Line<'_>> {
    vec![
        Line::from(vec![Span::styled(
            "═══ INSTANCES TAB ═══",
            Style::default().fg(Color::Cyan).bold(),
        )]),
        Line::from("↑/↓ or j/k         Navigate instance list"),
        Line::from("Shift+Enter        Launch selected instance"),
        Line::from("e                  Edit selected instance (coming soon)"),
        Line::from("d                  Delete selected instance (coming soon)"),
        Line::from("F5                 Refresh instance list"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw(
                "Select an instance and press Enter for launch info (use CLI for actual launching)",
            ),
        ]),
        Line::from(""),
    ]
}

/// Help for Create tab
fn get_create_help(app: &App) -> Vec<Line<'_>> {
    let mut help = vec![Line::from(vec![Span::styled(
        "═══ CREATE INSTANCE TAB ═══",
        Style::default().fg(Color::Green).bold(),
    )])];

    if app.is_editing_name {
        help.extend(vec![
            Line::from(vec![Span::styled(
                "Currently editing instance name",
                Style::default().fg(Color::Yellow).italic(),
            )]),
            Line::from("Type               Enter instance name"),
            Line::from("Backspace          Delete character"),
            Line::from("Esc                Finish editing name"),
            Line::from(""),
        ]);
    } else {
        help.extend(vec![
            Line::from("↑/↓ or j/k         Navigate version list"),
            Line::from("Ctrl+N             Edit instance name"),
            Line::from("Ctrl+D             Toggle download assets"),
            Line::from(""),
        ]);
    }

    // Version search controls
    help.extend(vec![
        Line::from(vec![Span::styled(
            "Version search:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("Ctrl+S             Toggle search mode for versions"),
        Line::from("Type               Filter versions while search is active"),
        Line::from("Backspace          Delete character (search mode)"),
        Line::from("Esc                Exit search mode"),
        Line::from("Enter              Create using the selected match"),
        Line::from(""),
    ]);

    // Version type filters
    help.extend(vec![
        Line::from(vec![Span::styled(
            "Version filters:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("F6                 Toggle Release"),
        Line::from("F7                 Toggle Snapshot"),
        Line::from("F8                 Toggle Beta"),
        Line::from("F9                 Toggle Alpha"),
        Line::from("F10                Reset all filters"),
        Line::from(""),
    ]);

    help.extend(vec![
        Line::from(vec![Span::styled(
            "Create Instance Guide:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("1. Press Ctrl+N to edit instance name"),
        Line::from("2. Select Minecraft version (↑/↓)"),
        Line::from("3. Toggle asset download (Ctrl+D)"),
        Line::from("   ☑ Enabled: Slower download, with sound/music"),
        Line::from("   ☐ Disabled: Faster download, no sound/music"),
        Line::from("4. Press Enter to create"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Filters:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("Ctrl+R             Toggle Release"),
        Line::from("Ctrl+P             Toggle Snapshot"),
        Line::from("Ctrl+B             Toggle Beta"),
        Line::from("Ctrl+A             Toggle Alpha"),
        Line::from("Ctrl+0             Reset all filters"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(Color::Cyan)),
            Span::raw("Simple controls: Ctrl+N to edit/create, Ctrl+D to toggle assets"),
        ]),
        Line::from(""),
    ]);

    help
}

/// Help for Settings tab
fn get_settings_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "═══ SETTINGS TAB ═══",
            Style::default().fg(Color::Magenta).bold(),
        )]),
        Line::from("↑/↓ or j/k         Navigate left menu / license list"),
        Line::from("→ / Enter          Focus middle pane when on 'Licenses'"),
        Line::from("← / Esc            Return focus to left pane"),
        Line::from("PageUp/PageDown    Scroll license text (right pane)"),
        Line::from("Mouse wheel        Scroll license text (right pane)"),
        Line::from("r                  Reset to defaults (coming soon)"),
        Line::from("s                  Save settings (coming soon)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw("Licenses page: left = category, middle = license, right = text"),
        ]),
        Line::from(""),
    ]
}

/// Help for Accounts tab
fn get_accounts_help(app: &App) -> Vec<Line<'static>> {
    let mut help = vec![Line::from(vec![Span::styled(
        "═══ ACCOUNTS TAB ═══",
        Style::default().fg(Color::Blue).bold(),
    )])];

    if app.is_add_account_mode {
        help.extend(vec![
            Line::from(vec![Span::styled(
                "Currently adding new account",
                Style::default().fg(Color::Yellow).italic(),
            )]),
            Line::from("↑/↓ or j/k         Select account type"),
            Line::from("Tab                Switch between fields"),
            Line::from("Type               Enter credentials"),
            Line::from("Enter              Create account"),
            Line::from("Esc                Cancel adding account"),
            Line::from(""),
            Line::from("Account Types:"),
            Line::from("• Microsoft        - Official Minecraft account"),
            Line::from("• ElyBy           - Alternative auth service"),
            Line::from("• LittleSkin      - Alternative auth service"),
            Line::from("• Offline         - Play without authentication (specify username)"),
            Line::from(""),
        ]);
    } else {
        help.extend(vec![
            Line::from("↑/↓ or j/k         Navigate account list"),
            Line::from("l                  Logout selected account (if logged in)"),
            Line::from("n                  Add new account (with login for ElyBy/LittleSkin)"),
            Line::from("d                  Set selected account as default"),
            Line::from("r                  Refresh account status"),
            Line::from(""),
        ]);
    }

    help.extend(vec![
        Line::from(vec![Span::styled(
            "Account Status:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("Green          - Currently logged in"),
        Line::from("Red            - Not logged in"),
        Line::from("Yellow         - Login in progress"),
        Line::from("*              - Default account (used for launching)"),
        Line::from("Blue           - Offline account (ready for launching)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "ElyBy Support:",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from("• Full username/password authentication"),
        Line::from("• Two-factor authentication (OTP) support"),
        Line::from("• Password visibility toggle"),
        Line::from("• Real-time error feedback"),
        Line::from(""),
    ]);

    help
}

/// Global help that applies to all tabs
fn get_global_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "═══ GLOBAL CONTROLS ═══",
            Style::default().fg(Color::White).bold(),
        )]),
        Line::from("←/→                Switch between tabs"),
        Line::from("Tab                Next tab"),
        Line::from("Shift+Tab          Previous tab"),
        Line::from("?                  Show/hide this help popup"),
        Line::from("q                  Quit application"),
        Line::from("F5                 Refresh current data"),
        Line::from("F12                Force terminal refresh (if display corrupted)"),
        Line::from("Esc                Go back / Cancel current action"),
        Line::from(""),
    ]
}

// accounts rendering moved to tabs::accounts

// Render the logs tab
// moved to tabs::logs::render_logs_tab

/// Help for Logs tab
fn get_logs_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "═══ LOGS TAB ═══",
            Style::default().fg(Color::Magenta).bold(),
        )]),
        Line::from("↑/↓ or j/k         Scroll by one line"),
        Line::from("PageUp/PageDown    Scroll by one page"),
        Line::from("Home/End or g/G    Jump to top/bottom"),
        Line::from("Mouse wheel        Scroll"),
        Line::from("c                  Clear logs"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw("Logs auto-follow at bottom; scroll up to pause follow"),
        ]),
        Line::from(""),
    ]
}

// Instance settings renderers moved to tabs::instance_settings

// Render instance logs tab
// moved to tabs::logs::render_instance_logs

/// Get help text for instance settings
fn get_instance_settings_help(_app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "═══ INSTANCE SETTINGS ═══",
            Style::default().fg(Color::Cyan).bold(),
        )]),
        Line::from("Left/Right or h/l   Switch between sub-tabs"),
        Line::from("Up/Down or j/k      Navigate items/actions"),
        Line::from("Enter               Select action/item"),
        Line::from("Esc                 Return to instances"),
        Line::from(""),
        Line::from("Overview Tab:"),
        Line::from("  ↑/↓               Navigate between Play/Kill/Folder"),
        Line::from("  Enter on Play     Launch the instance"),
        Line::from("  Enter on Kill     Stop running instance"),
        Line::from("  Enter on Folder   Open instance directory"),
        Line::from(""),
        Line::from("Other tabs show planned features."),
        Line::from(""),
        Line::from("Rename popup:"),
        Line::from("  Type/Backspace     Edit name"),
        Line::from("  Enter              Apply"),
        Line::from("  Esc                Cancel"),
        Line::from(""),
    ]
}
