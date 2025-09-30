// QuantumLauncher TUI - Input Event Handler

use crate::tui::app::{AccountType, App, TabId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Handle keyboard input events
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }

    // Handle help popup separately
    if app.show_help_popup {
        handle_help_popup_input(app, key.code);
        return false;
    }

    // Handle context-specific input with more specific conditions first
    match key.code {
        // Create tab: Ctrl+N begins editing instance name
        KeyCode::Char('n')
            if app.current_tab == TabId::Create
                && !app.is_editing_name
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.is_editing_name = true;
            app.status_message = "Editing instance name. Press Esc to finish editing.".to_string();
            false
        }
        // Start add-account flow on Accounts tab
        KeyCode::Char('n') | KeyCode::Char('N')
            if app.current_tab == TabId::Accounts && !app.is_add_account_mode =>
        {
            app.toggle_add_account_mode();
            false
        }
        // Account tab specific keys - must come first to override general patterns
        KeyCode::Esc if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.toggle_add_account_mode();
            false
        }
        KeyCode::Enter if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.add_new_account();
            false
        }
        KeyCode::Up if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.prev_account_type();
            false
        }
        KeyCode::Down if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.next_account_type();
            false
        }
        KeyCode::Backspace if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.remove_char_from_add_account_field();
            false
        }
        KeyCode::Tab if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            // Switch between username/password/OTP fields for ElyBy accounts
            if app.new_account_type == AccountType::ElyBy {
                app.next_add_account_field();
            }
            false
        }
        KeyCode::Char('p') if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            // Toggle password visibility for ElyBy accounts
            if app.new_account_type == AccountType::ElyBy {
                app.toggle_password_visibility();
            } else {
                app.add_char_to_add_account_field('p');
            }
            false
        }
        KeyCode::Char(c) if app.current_tab == TabId::Accounts && app.is_add_account_mode => {
            app.add_char_to_add_account_field(c);
            false
        }
        // General key handling - but NOT when editing instance name
        // If Esc pressed during version search, exit search instead of quitting
        KeyCode::Char('q') | KeyCode::Esc
            if !(app.current_tab == TabId::Create && app.is_editing_name) =>
        {
            if app.current_tab == TabId::Create && app.version_search_active {
                app.exit_version_search();
                false
            } else {
                true
            }
        }
        KeyCode::Up | KeyCode::Char('k')
            if !(app.current_tab == TabId::Create && app.is_editing_name) =>
        {
            app.prev_item();
            false
        }
        KeyCode::Down | KeyCode::Char('j')
            if !(app.current_tab == TabId::Create && app.is_editing_name) =>
        {
            app.next_item();
            false
        }
        // Tab navigation: Left/Right arrows
        KeyCode::Left if !(app.current_tab == TabId::Create && app.is_editing_name) => {
            app.previous_tab();
            false
        }
        KeyCode::Right if !(app.current_tab == TabId::Create && app.is_editing_name) => {
            app.next_tab();
            false
        }
        // Tab navigation: Tab = next, Shift+Tab (BackTab) = previous
        KeyCode::Tab if !(app.current_tab == TabId::Create && app.is_editing_name) => {
            app.next_tab();
            false
        }
        KeyCode::BackTab if !(app.current_tab == TabId::Create && app.is_editing_name) => {
            app.previous_tab();
            false
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            // Shift+Enter: Launch the selected instance
            app.launch_selected_instance();
            false
        }
        // Enter on Instances: open instance settings
        KeyCode::Enter if app.current_tab == TabId::Instances => {
            app.select_item();
            false
        }
        // Logs tab specific keys - must come before general 'c' key
        KeyCode::Char('c') if app.current_tab == TabId::Logs => {
            app.clear_logs();
            app.status_message = "✅ Logs cleared".to_string();
            false
        }
        // Removed old letter/number shortcuts for tab switching; use arrows and Tab/Shift+Tab instead
        KeyCode::Char('?') if !(app.current_tab == TabId::Create && app.is_editing_name) => {
            app.toggle_help_popup();
            false
        }
        // F5 is handled in the main TUI loop for async refresh
        // Create tab editing mode - handle character input first when editing
        KeyCode::Backspace if app.current_tab == TabId::Create && app.is_editing_name => {
            app.new_instance_name.pop();
            false
        }
        KeyCode::Char(c) if app.current_tab == TabId::Create && app.is_editing_name => {
            app.new_instance_name.push(c);
            false
        }
        KeyCode::Esc if app.current_tab == TabId::Create && app.is_editing_name => {
            app.is_editing_name = false;
            app.status_message = "Finished editing instance name.".to_string();
            false
        }
        // Create tab commands (only when NOT editing)
        KeyCode::Char('d')
            if app.current_tab == TabId::Create
                && !app.is_editing_name
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.download_assets = !app.download_assets;
            app.status_message = if app.download_assets {
                "✅ Download assets enabled (sound/music will be available)".to_string()
            } else {
                "⚠️  Download assets disabled (faster creation, no sound/music)".to_string()
            };
            false
        }
        // Removed: Tab toggle for assets (conflicts with Tab for tab navigation)
        KeyCode::Enter if app.current_tab == TabId::Create && !app.is_editing_name => {
            if app.version_search_active {
                app.create_instance();
                return false;
            }
            if app.new_instance_name.is_empty() {
                // If no name entered, start editing the name
                app.is_editing_name = true;
                app.status_message =
                    "Editing instance name. Press Esc to finish editing.".to_string();
            } else if !app.available_versions.is_empty() && !app.is_loading {
                // If name is entered and versions available, create instance
                app.create_instance();
            } else if app.available_versions.is_empty() {
                app.status_message = "❌ No versions available. Press F5 to refresh.".to_string();
            }
            false
        }
        // Live search in Create tab
        KeyCode::Backspace if app.current_tab == TabId::Create && !app.is_editing_name => {
            if app.version_search_active {
                app.remove_char_from_version_search();
            }
            false
        }
        // Only accept characters when search is active
        KeyCode::Char(c)
            if app.current_tab == TabId::Create
                && !app.is_editing_name
                && app.version_search_active =>
        {
            app.add_char_to_version_search(c);
            false
        }
        // Ctrl+S toggles search mode
        KeyCode::Char('s')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            if app.version_search_active {
                app.exit_version_search();
            } else {
                app.start_version_search();
            }
            false
        }
        // Version type filters (Create tab)
        KeyCode::F(6) if app.current_tab == TabId::Create => {
            app.toggle_filter_release();
            false
        }
        KeyCode::F(7) if app.current_tab == TabId::Create => {
            app.toggle_filter_snapshot();
            false
        }
        KeyCode::F(8) if app.current_tab == TabId::Create => {
            app.toggle_filter_beta();
            false
        }
        KeyCode::F(9) if app.current_tab == TabId::Create => {
            app.toggle_filter_alpha();
            false
        }
        KeyCode::F(10) if app.current_tab == TabId::Create => {
            app.reset_all_filters();
            false
        }
        // Filter toggles in Create tab
        KeyCode::Char('r')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.toggle_filter_release();
            false
        }
        KeyCode::Char('b')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.toggle_filter_beta();
            false
        }
        KeyCode::Char('a')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.toggle_filter_alpha();
            false
        }
        KeyCode::Char('p')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.toggle_filter_snapshot();
            false
        }
        KeyCode::Char('0')
            if app.current_tab == TabId::Create
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.reset_all_filters();
            false
        }
        // Account tab specific keys
        KeyCode::Char('l') if app.current_tab == TabId::Accounts && !app.is_add_account_mode => {
            if !app.accounts.is_empty() {
                // Always allow logout attempt - the logout function will handle validation
                app.logout_account();
            } else {
                app.status_message = "No accounts to logout".to_string();
            }
            false
        }
        KeyCode::Char('d') if app.current_tab == TabId::Accounts && !app.is_add_account_mode => {
            app.set_default_account();
            false
        }
        _ => false,
    }
}

/// Handle input when help popup is shown
fn handle_help_popup_input(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
            app.toggle_help_popup();
        }
        _ => {}
    }
}
