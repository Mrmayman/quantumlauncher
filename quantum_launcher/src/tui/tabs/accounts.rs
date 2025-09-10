use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{self, App};

pub fn render_accounts_tab(f: &mut Frame, area: Rect, app: &App) {
    if app.is_add_account_mode {
        render_add_account_form(f, area, app);
    } else {
        render_account_list(f, area, app);
    }
}

fn render_account_list(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Account list
            Constraint::Length(8), // Login form or account actions
        ])
        .split(area);

    // Account List
    let account_items: Vec<ListItem> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let status = if account.account_type == "Offline" {
                " (offline - ready to launch)"
            } else if account.is_logged_in {
                " (logged in)"
            } else {
                ""
            };

            // Check if this account is the default
            let is_default = if let Some(ref current_account) = app.current_account {
                current_account == &account.username
            } else {
                false
            };

            let default_indicator = if is_default { " * (default)" } else { "" };
            let content = format!(
                "{} [{}]{}{}",
                account.username, account.account_type, status, default_indicator
            );

            let mut item = ListItem::new(content);
            if i == app.selected_account {
                item = item.style(Style::default().bg(Color::Yellow).fg(Color::Black));
            }
            item
        })
        .collect();

    let accounts_list = List::new(account_items)
        .block(Block::default().borders(Borders::ALL).title("Accounts"))
        .style(Style::default().fg(Color::White));

    f.render_widget(accounts_list, chunks[0]);

    // Bottom panel for login form or account actions
    // Account actions - no longer supporting separate login mode
    let selected_account_info = if let Some(account) = app.get_selected_account() {
        if account.is_logged_in {
            format!(
                "Selected: {} (logged in)\nPress 'l' to logout, 'n' to add new account",
                account.username
            )
        } else {
            format!("Selected: {} (not logged in)\nPress 'n' to add new account (login during creation)", account.username)
        }
    } else {
        format!(
            "Debug: {} accounts, selected index: {}\nPress 'n' to add new account",
            app.accounts.len(),
            app.selected_account
        )
    };

    let account_info = Paragraph::new(selected_account_info)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(account_info, chunks[1]);
}

fn render_add_account_form(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Account type selection - much bigger to show all 4 options (4 items + 2 borders + 4 padding)
            Constraint::Length(3),  // Username input - normal size
            Constraint::Length(3),  // Password input or info - normal size
            Constraint::Length(3),  // OTP input (if needed) or empty - normal size
            Constraint::Min(3),     // Instructions/status - minimum space
        ])
        .split(area);

    // Account type selection
    let account_types = app::AccountType::all();
    let type_items: Vec<ListItem> = account_types
        .iter()
        .enumerate()
        .map(|(i, account_type)| {
            let content = account_type.to_string();
            let mut item = ListItem::new(content);
            if i == app.selected_account_type && app.selected_account_type < account_types.len() {
                item = item.style(Style::default().bg(Color::Yellow).fg(Color::Black));
            }
            item
        })
        .collect();

    // Create a ListState to control scrolling
    let mut list_state = ListState::default();
    if app.selected_account_type < account_types.len() {
        list_state.select(Some(app.selected_account_type));
    }

    let type_list = List::new(type_items)
        .block(Block::default().borders(Borders::ALL).title("Account Type"))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    f.render_stateful_widget(type_list, chunks[0], &mut list_state);

    // Username input with focus indication for ElyBy and LittleSkin
    let username_title = if (app.new_account_type == app::AccountType::ElyBy
        || app.new_account_type == app::AccountType::LittleSkin)
        && app.add_account_field_focus == app::AddAccountFieldFocus::Username
    {
        "Username/Email [CURRENTLY TYPING HERE]"
    } else {
        "Username/Email"
    };

    let username_style = if (app.new_account_type == app::AccountType::ElyBy
        || app.new_account_type == app::AccountType::LittleSkin)
        && app.add_account_field_focus == app::AddAccountFieldFocus::Username
    {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let username_input = Paragraph::new(app.new_account_username.as_str())
        .block(Block::default().borders(Borders::ALL).title(username_title))
        .style(username_style);
    f.render_widget(username_input, chunks[1]);

    // Password input (for ElyBy, LittleSkin, and Microsoft accounts)
    if app.new_account_type != app::AccountType::Offline {
        let password_title = if (app.new_account_type == app::AccountType::ElyBy
            || app.new_account_type == app::AccountType::LittleSkin)
            && app.add_account_field_focus == app::AddAccountFieldFocus::Password
        {
            "Password [CURRENTLY TYPING HERE]"
        } else {
            "Password"
        };

        let password_style = if (app.new_account_type == app::AccountType::ElyBy
            || app.new_account_type == app::AccountType::LittleSkin)
            && app.add_account_field_focus == app::AddAccountFieldFocus::Password
        {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let password_display = if app.show_password {
            app.new_account_password.clone()
        } else {
            "*".repeat(app.new_account_password.len())
        };

        let password_input = Paragraph::new(password_display.as_str())
            .block(Block::default().borders(Borders::ALL).title(password_title))
            .style(password_style);
        f.render_widget(password_input, chunks[2]);
    } else if app.new_account_type == app::AccountType::Offline {
        let info_text = "Offline accounts use the specified username for playing";
        let info_paragraph = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Info"))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(info_paragraph, chunks[2]);
    } else {
        let info_text = "Microsoft accounts use OAuth2 authentication";
        let info_paragraph = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Info"))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(info_paragraph, chunks[2]);
    }

    // OTP input (only if needed for ElyBy)
    if app.needs_otp {
        let otp_title = if app.add_account_field_focus == app::AddAccountFieldFocus::Otp {
            "OTP Code [CURRENTLY TYPING HERE]"
        } else {
            "OTP Code"
        };

        let otp_style = if app.add_account_field_focus == app::AddAccountFieldFocus::Otp {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let empty_string = String::new();
        let otp_display = app.new_account_otp.as_ref().unwrap_or(&empty_string);
        let otp_input = Paragraph::new(otp_display.as_str())
            .block(Block::default().borders(Borders::ALL).title(otp_title))
            .style(otp_style);
        f.render_widget(otp_input, chunks[3]);
    } else {
        // Empty placeholder
        let empty_paragraph = Paragraph::new("").style(Style::default().fg(Color::Gray));
        f.render_widget(empty_paragraph, chunks[3]);
    }

    // Instructions and error messages
    let mut instruction_lines = vec![];

    match app.new_account_type {
        app::AccountType::Microsoft => {
            instruction_lines
                .push("↑/↓: Select account type | Enter: Add account | Esc: Cancel".to_string());
            instruction_lines.push(
                "Warning: Microsoft accounts are not implemented yet - coming soon!".to_string(),
            );
        }
        app::AccountType::Offline => {
            instruction_lines.push(
                "↑/↓: Select account type | Enter: Add offline account | Esc: Cancel".to_string(),
            );
            instruction_lines
                .push("Offline accounts use the specified username for playing".to_string());
        }
        app::AccountType::ElyBy => {
            instruction_lines.push(
                "↑/↓: Select account type | Tab: Next field | p: Toggle password visibility"
                    .to_string(),
            );
            instruction_lines.push("Enter: Create account | Esc: Cancel".to_string());

            if app.needs_otp {
                instruction_lines
                    .push("Two-factor authentication required - enter OTP code".to_string());
            }
        }
        app::AccountType::LittleSkin => {
            instruction_lines.push(
                "↑/↓: Select account type | Tab: Next field | Enter: Add account | Esc: Cancel"
                    .to_string(),
            );
            instruction_lines.push(
                "Warning: LittleSkin accounts are not implemented yet - coming soon!".to_string(),
            );
        }
    }

    // Add error message if present
    if let Some(ref error) = app.login_error {
        instruction_lines.push("".to_string());
        instruction_lines.push(format!("Error: {}", error));
    }

    let instructions_text = instruction_lines.join("\n");
    let instructions_paragraph = Paragraph::new(instructions_text)
        .block(Block::default().borders(Borders::ALL).title("Instructions"))
        .style(Style::default().fg(Color::Green))
        .wrap(Wrap { trim: true });
    f.render_widget(instructions_paragraph, chunks[4]);
}
