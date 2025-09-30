// QuantumLauncher TUI - Popup Widgets

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, TabId};

/// Render loading popup
pub fn render_loading_popup(f: &mut Frame) {
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

    // Simple loading indicator
    let progress_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width - 4,
        height: 1,
    };

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Yellow))
        .ratio(0.5); // Static for now, could be animated

    f.render_widget(gauge, progress_area);
}

/// Helper function to center a rect
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

/// Render help popup with contextual controls based on current tab and state
pub fn render_help_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let help_text = get_contextual_help(app);

    let block = Block::default()
        .title("❓ Help & Controls")
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
pub fn get_contextual_help(app: &App) -> Vec<Line> {
    let mut help_text = vec![
        Line::from(vec![
            Span::styled("QuantumLauncher TUI Controls", Style::default().fg(Color::Yellow).bold())
        ]),
        Line::from(""),
    ];

    // Add tab-specific help based on current tab
    match app.current_tab {
        TabId::Instances => help_text.extend(get_instances_help()),
        TabId::Create => help_text.extend(get_create_help(app)),
        TabId::Settings => help_text.extend(get_settings_help()),
        TabId::Accounts => help_text.extend(get_accounts_help(app)),
        TabId::Logs => help_text.extend(get_logs_help()),
    }

    // Add common/global help
    help_text.extend(get_global_help());

    help_text.push(Line::from(""));
    help_text.push(Line::from(vec![
        Span::styled("Press '?' or Esc to close this help", Style::default().fg(Color::Green).italic())
    ]));

    help_text
}

/// Help for Instances tab
pub fn get_instances_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("═══ INSTANCES TAB ═══", Style::default().fg(Color::Cyan).bold())
        ]),
        Line::from("↑/↓ or j/k         Navigate instance list"),
        Line::from("Shift+Enter        Launch selected instance"),
        Line::from("e                  Edit selected instance (coming soon)"),
        Line::from("d                  Delete selected instance (coming soon)"),
        Line::from("F5                 Refresh instance list"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Select an instance and press Enter for launch info (use CLI for actual launching)")
        ]),
        Line::from(""),
    ]
}

/// Help for Create tab
pub fn get_create_help(app: &App) -> Vec<Line> {
    let mut help = vec![
        Line::from(vec![
            Span::styled("═══ CREATE INSTANCE TAB ═══", Style::default().fg(Color::Green).bold())
        ]),
    ];

    if app.is_editing_name {
        help.extend(vec![
            Line::from("Typing text...      Instance name input is active"),
            Line::from("Esc                Finish editing name and focus version list"),
            Line::from("Backspace          Remove character from name"),
        ]);
    } else {
        help.extend(vec![
            Line::from("Ctrl+N             Start editing instance name"),
            Line::from("↑/↓ or j/k         Navigate Minecraft versions"),
            Line::from("Enter              Create instance with selected version"),
        ]);
    }

    help.extend(vec![
        Line::from("F5                 Refresh version list"),
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 Tips:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("• Choose a descriptive name for your instance"),
        Line::from("• Select the Minecraft version you want to play"),
        Line::from("• Press Enter when ready to create the instance"),
        Line::from(""),
    ]);

    help
}

/// Help for Settings tab
pub fn get_settings_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("═══ SETTINGS TAB ═══", Style::default().fg(Color::Magenta).bold())
        ]),
        Line::from("↑/↓ or j/k         Navigate settings list"),
        Line::from("Enter              Edit selected setting"),
        Line::from("c                  Clear Java installs (when selected)"),
        Line::from("Esc                Cancel editing / Exit"),
        Line::from("s                  Save current edit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Available Settings:", Style::default().bold())
        ]),
        Line::from("• Java Arguments   - Global JVM arguments"),
        Line::from("• Pre-launch Prefix - Commands before launch"),
        Line::from("• Window Size      - Default Minecraft window"),
        Line::from("• Clear Java Installs - Reset Java detection"),
        Line::from("• UI Scale         - Interface scaling"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Configure global launcher settings that apply to all instances")
        ]),
    ]
}

/// Help for Accounts tab
pub fn get_accounts_help(app: &App) -> Vec<Line<'static>> {
    let mut help = vec![
        Line::from(vec![
            Span::styled("═══ ACCOUNTS TAB ═══", Style::default().fg(Color::Blue).bold())
        ]),
    ];

    if app.is_add_account_mode {
        help.extend(vec![
            Line::from("↑/↓                Change account type"),
            Line::from("Tab                Switch between fields (ElyBy only)"),
            Line::from("Typing text...     Enter account credentials"),
            Line::from("p                  Toggle password visibility (ElyBy only)"),
            Line::from("Enter              Add account and login"),
            Line::from("Esc                Cancel add account mode"),
        ]);
    } else {
        help.extend(vec![
            Line::from("↑/↓ or j/k         Navigate account list"),
            Line::from("n                  Add new account"),
            Line::from("l                  Logout selected account"),
            Line::from("d                  Set selected account as default"),
            Line::from("Enter              Use account for launching"),
        ]);
    }

    help.extend(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 Account Status:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("🟢 Green          - Currently logged in"),
        Line::from("🔴 Red            - Not logged in"),
        Line::from("🟡 Yellow         - Login in progress"),
        Line::from("★                 - Default account (used for launching)"),
        Line::from("🔵 Blue           - Offline account (ready for launching)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("🌐 ElyBy Support:", Style::default().fg(Color::Cyan)),
        ]),
        Line::from("• Full username/password authentication"),
        Line::from("• Two-factor authentication (OTP) support"),
        Line::from("• Password visibility toggle"),
        Line::from("• Real-time error feedback"),
        Line::from(""),
    ]);

    help
}

/// Help for Logs tab
pub fn get_logs_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("═══ LOGS TAB ═══", Style::default().fg(Color::Red).bold())
        ]),
        Line::from("↑/↓ or j/k         Scroll by one line"),
        Line::from("PageUp/PageDown    Scroll by one page"),
        Line::from("Home/End or g/G    Jump to top/bottom"),
        Line::from("Mouse wheel        Scroll logs"),
        Line::from("c                  Clear logs"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Logs auto-follow at bottom; scroll up to pause follow")
        ]),
        Line::from(""),
    ]
}

/// Global help that applies to all tabs
pub fn get_global_help() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("═══ GLOBAL CONTROLS ═══", Style::default().fg(Color::White).bold())
        ]),
    Line::from("←/→              Switch between tabs"),
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
