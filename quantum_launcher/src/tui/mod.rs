// QuantumLauncher TUI Module

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use tokio::sync::mpsc;

// Default configuration
const DEFAULT_TUI_REFRESH_INTERVAL_MS: u64 = 10000; // Fallback periodic refresh

// Use the refactored app module
#[path = "app/mod.rs"]
mod app;
mod handlers;
pub mod tabs;
mod ui;

pub use app::{App, AppResult};

#[derive(Debug, Clone)]
pub enum AuthEvent {
    LoginStarted,
    LoginSuccess {
        account_data: ql_instances::auth::AccountData,
    },
    LoginNeedsOtp,
    LoginError {
        error: String,
    },
    LaunchStarted(String),
    LaunchSuccess(
        String,
        std::sync::Arc<std::sync::Mutex<tokio::process::Child>>,
    ),
    LaunchError(String, String),
    LaunchEnded(String),
    // New: per-instance log line streamed from the running game's stdout/stderr
    InstanceLogLine {
        instance_name: String,
        line: String,
    },
    InstanceCreateStarted(String),
    InstanceCreateProgress {
        instance_name: String,
        message: String,
    },
    InstanceCreateSuccess {
        instance_name: String,
    },
    InstanceCreateError {
        instance_name: String,
        error: String,
    },
    RefreshStarted,
    RefreshCompleted,
    RefreshData {
        instances: Vec<(String, String, String)>,
    }, // (name, version, loader)
}

/// Entry point for the TUI mode
pub fn run_tui() -> AppResult<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Disable stdio logging so println!/eprintln! don't corrupt the TUI
    ql_core::print::set_stdio_logging_enabled(false);
    // Create async runtime and run the app
    let rt = tokio::runtime::Runtime::new()?;
    let app = App::new();
    let res = rt.block_on(run_app(&mut terminal, app));

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Re-enable stdio logging now that TUI is closed
    ql_core::print::set_stdio_logging_enabled(true);

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

/// Main event loop for the TUI
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> AppResult<()> {
    // Create authentication channel
    let (auth_tx, mut auth_rx) = mpsc::unbounded_channel::<AuthEvent>();
    app.set_auth_channel(auth_tx);

    // Track last refresh time for periodic refreshing
    let mut last_refresh = std::time::Instant::now();

    loop {
        // Sync TUI logs with core logger buffer (freeze when user scrolled up)
        {
            let latest = ql_core::print::get_logs_lines(Some(2000));
            let at_bottom = app.logs_offset == 0 && app.logs_auto_follow;
            if at_bottom {
                if latest != app.game_logs {
                    app.game_logs = latest;
                    app.logs_offset = 0; // keep following
                }
            } else {
                // User is viewing history; don't replace buffer to avoid snapping
            }
            // Do not clamp here using raw lines; renderer clamps by wrapped rows.
        }

        // Check for auth events first
        if let Ok(auth_event) = auth_rx.try_recv() {
            handlers::auth::handle_auth_event(&mut app, auth_event);
        }

        // Check if we need a forced refresh (e.g., after launch events that might spam stdout)
        let needs_forced_refresh = app.check_and_reset_forced_refresh();

        // Check if it's time for periodic refresh (use configured value with default fallback)
        let interval_ms = app
            .tui_refresh_interval_ms
            .unwrap_or(DEFAULT_TUI_REFRESH_INTERVAL_MS);
        // Allow disabling periodic refresh by setting interval to 0 ms
        let needs_periodic_refresh =
            interval_ms != 0 && last_refresh.elapsed().as_millis() >= interval_ms as u128;

        if needs_forced_refresh || needs_periodic_refresh {
            // Clear terminal and force a complete redraw to overwrite any stdout spam
            terminal.clear()?;
            if needs_periodic_refresh {
                last_refresh = std::time::Instant::now();
            }
        }

        terminal.draw(|f| ui::render(f, &mut app))?;

        // Handle keyboard input with timeout to allow auth events to be processed
        if let Ok(has_event) = crossterm::event::poll(std::time::Duration::from_millis(50)) {
            if has_event {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        // First, if rename popup is active, capture keys specifically
                        if app.is_renaming_instance {
                            match key.code {
                                KeyCode::Esc => {
                                    app.cancel_rename_instance();
                                }
                                KeyCode::Enter => {
                                    app.apply_rename_instance();
                                }
                                KeyCode::Backspace => {
                                    app.rename_input.pop();
                                }
                                KeyCode::Char(c) => {
                                    app.rename_input.push(c);
                                }
                                _ => {}
                            }
                            continue;
                        }
                        // Memory edit popup handling
                        if app.is_editing_memory {
                            match key.code {
                                KeyCode::Esc => {
                                    app.cancel_memory_edit();
                                }
                                KeyCode::Enter => {
                                    app.apply_memory_edit();
                                }
                                KeyCode::Backspace => {
                                    app.memory_edit_input.pop();
                                }
                                KeyCode::Char(c) => {
                                    app.memory_edit_input.push(c);
                                }
                                _ => {}
                            }
                            continue;
                        }
                        // Args edit popup handling
                        if app.is_editing_args {
                            match key.code {
                                KeyCode::Esc => {
                                    app.cancel_args_edit();
                                }
                                KeyCode::Enter => {
                                    if app.args_edit_kind == app::ArgsEditKind::GlobalJava {
                                        app.apply_global_java_args_edit();
                                    } else if app.args_edit_kind == app::ArgsEditKind::WindowSize {
                                        // Save WIDTH,HEIGHT into instance global_settings
                                        if let Some(idx) = app.instance_settings_instance {
                                            if let Some(inst) = app.instances.get(idx) {
                                                let instance_name = inst.name.clone();
                                                if let Ok(dir) =
                                                    ql_core::file_utils::get_launcher_dir()
                                                {
                                                    let path = dir
                                                        .join("instances")
                                                        .join(&instance_name)
                                                        .join("config.json");
                                                    if let Ok(s) = std::fs::read_to_string(&path) {
                                                        if let Ok(mut cfg) = serde_json::from_str::<
                                                            ql_core::json::InstanceConfigJson,
                                                        >(
                                                            &s
                                                        ) {
                                                            let txt = app.args_edit_input.trim();
                                                            if txt.is_empty() {
                                                                if let Some(gs) =
                                                                    cfg.global_settings.as_mut()
                                                                {
                                                                    gs.window_width = None;
                                                                    gs.window_height = None;
                                                                }
                                                            } else {
                                                                let parts: Vec<&str> = txt
                                                                    .split([',', 'x', 'X'])
                                                                    .collect();
                                                                if parts.len() >= 2 {
                                                                    let w = parts[0]
                                                                        .trim()
                                                                        .parse::<u32>()
                                                                        .ok();
                                                                    let h = parts[1]
                                                                        .trim()
                                                                        .parse::<u32>()
                                                                        .ok();
                                                                    let gs = cfg
                                                                        .global_settings
                                                                        .get_or_insert_with(
                                                                            Default::default,
                                                                        );
                                                                    gs.window_width = w;
                                                                    gs.window_height = h;
                                                                }
                                                            }
                                                            if let Ok(new_s) =
                                                                serde_json::to_string_pretty(&cfg)
                                                            {
                                                                let _ =
                                                                    std::fs::write(&path, new_s);
                                                            }
                                                            app.is_editing_args = false;
                                                            app.status_message =
                                                                "✅ Saved window size".to_string();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if app.args_edit_kind
                                        == app::ArgsEditKind::GlobalWindowSize
                                    {
                                        app.apply_global_window_size_edit();
                                    } else if app.args_edit_kind
                                        == app::ArgsEditKind::GlobalTuiRefreshInterval
                                    {
                                        app.apply_tui_refresh_interval_edit();
                                    } else {
                                        app.apply_args_edit();
                                    }
                                }
                                KeyCode::Backspace => {
                                    app.args_edit_input.pop();
                                }
                                KeyCode::Char(c) => {
                                    app.args_edit_input.push(c);
                                }
                                _ => {}
                            }
                            continue;
                        }
                        // Args edit popup handling
                        if app.is_editing_args {
                            match key.code {
                                KeyCode::Esc => {
                                    app.cancel_args_edit();
                                }
                                KeyCode::Enter => {
                                    app.apply_args_edit();
                                }
                                KeyCode::Backspace => {
                                    app.args_edit_input.pop();
                                }
                                KeyCode::Char(c) => {
                                    app.args_edit_input.push(c);
                                }
                                _ => {}
                            }
                            continue;
                        }
                        // Handle delete confirmation popup
                        if app.show_delete_confirm {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    // Confirm deletion
                                    if let Some(idx) = app.instance_settings_instance {
                                        let name = app.instances[idx].name.clone();
                                        app.delete_instance(&name);
                                    }
                                    app.show_delete_confirm = false;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    // Cancel deletion
                                    app.show_delete_confirm = false;
                                    app.status_message = "Instance deletion cancelled.".to_string();
                                }
                                _ => {}
                            }
                            continue;
                        }
                        // Handle help popup separately
                        if app.show_help_popup {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                                    app.toggle_help_popup();
                                }
                                _ => {}
                            }
                        } else {
                            // Normal key handling when popups are not shown
                            // 1) Logs tab scrolling
                            match key.code {
                                KeyCode::Up if app.current_tab == app::TabId::Logs => {
                                    app.logs_auto_follow = false;
                                    app.logs_offset = app.logs_offset.saturating_add(1);
                                    continue;
                                }
                                KeyCode::Down if app.current_tab == app::TabId::Logs => {
                                    app.logs_offset = app.logs_offset.saturating_sub(1);
                                    if app.logs_offset == 0 {
                                        app.logs_auto_follow = true;
                                    }
                                    continue;
                                }
                                KeyCode::PageUp if app.current_tab == app::TabId::Logs => {
                                    app.logs_auto_follow = false;
                                    app.logs_offset =
                                        app.logs_offset.saturating_add(app.logs_visible_lines);
                                    continue;
                                }
                                KeyCode::PageDown if app.current_tab == app::TabId::Logs => {
                                    app.logs_offset =
                                        app.logs_offset.saturating_sub(app.logs_visible_lines);
                                    if app.logs_offset == 0 {
                                        app.logs_auto_follow = true;
                                    }
                                    continue;
                                }
                                KeyCode::Home if app.current_tab == app::TabId::Logs => {
                                    app.logs_auto_follow = false;
                                    app.logs_offset = app.game_logs.len();
                                    continue;
                                }
                                KeyCode::End if app.current_tab == app::TabId::Logs => {
                                    app.logs_offset = 0;
                                    app.logs_auto_follow = true;
                                    continue;
                                }
                                KeyCode::Char('c') if app.current_tab == app::TabId::Logs => {
                                    app.clear_logs();
                                    app.logs_offset = 0;
                                    app.logs_auto_follow = true;
                                    continue;
                                }
                                KeyCode::Char('j') if app.current_tab == app::TabId::Logs => {
                                    app.logs_offset = app.logs_offset.saturating_sub(1);
                                    if app.logs_offset == 0 {
                                        app.logs_auto_follow = true;
                                    }
                                    continue;
                                }
                                KeyCode::Char('k') if app.current_tab == app::TabId::Logs => {
                                    app.logs_auto_follow = false;
                                    app.logs_offset = app.logs_offset.saturating_add(1);
                                    continue;
                                }
                                KeyCode::Char('g') if app.current_tab == app::TabId::Logs => {
                                    app.logs_auto_follow = false;
                                    app.logs_offset = app.game_logs.len();
                                    continue;
                                }
                                KeyCode::Char('G') if app.current_tab == app::TabId::Logs => {
                                    app.logs_offset = 0;
                                    app.logs_auto_follow = true;
                                    continue;
                                }
                                _ => {}
                            }

                            // 2) Instance-specific logs scrolling (Instance Settings -> Logs)
                            match key.code {
                                KeyCode::Up
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_auto_follow = false;
                                    app.instance_logs_offset =
                                        app.instance_logs_offset.saturating_add(1);
                                    continue;
                                }
                                KeyCode::Down
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_offset =
                                        app.instance_logs_offset.saturating_sub(1);
                                    if app.instance_logs_offset == 0 {
                                        app.instance_logs_auto_follow = true;
                                    }
                                    continue;
                                }
                                KeyCode::PageUp
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_auto_follow = false;
                                    app.instance_logs_offset = app
                                        .instance_logs_offset
                                        .saturating_add(app.instance_logs_visible_lines);
                                    continue;
                                }
                                KeyCode::PageDown
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_offset = app
                                        .instance_logs_offset
                                        .saturating_sub(app.instance_logs_visible_lines);
                                    if app.instance_logs_offset == 0 {
                                        app.instance_logs_auto_follow = true;
                                    }
                                    continue;
                                }
                                KeyCode::Home
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_auto_follow = false;
                                    app.instance_logs_offset =
                                        app.instance_logs_offset.saturating_add(usize::MAX / 2);
                                    continue;
                                }
                                KeyCode::End
                                    if app.current_tab == app::TabId::InstanceSettings
                                        && app.instance_settings_tab
                                            == app::InstanceSettingsTab::Logs =>
                                {
                                    app.instance_logs_offset = 0;
                                    app.instance_logs_auto_follow = true;
                                    continue;
                                }
                                _ => {}
                            }

                            // 3) Settings-specific page scrolling
                            match key.code {
                                KeyCode::PageUp if app.current_tab == app::TabId::Settings => {
                                    app.about_scroll = app.about_scroll.saturating_sub(8);
                                    continue;
                                }
                                KeyCode::PageDown if app.current_tab == app::TabId::Settings => {
                                    app.about_scroll = app.about_scroll.saturating_add(8);
                                    continue;
                                }
                                _ => {}
                            }

                            // 4) Instance Settings navigation & Settings focus overrides
                            match key.code {
                                // F5: refresh instances list asynchronously
                                KeyCode::F(5) => {
                                    app.start_refresh();
                                    continue;
                                }
                                // Esc in instance settings: back from subpage or to Instances
                                KeyCode::Esc if app.current_tab == app::TabId::InstanceSettings => {
                                    if app.instance_settings_tab
                                        == app::InstanceSettingsTab::Setting
                                        && app.instance_settings_page
                                            != app::InstanceSettingsPage::List
                                    {
                                        app.instance_settings_page =
                                            app::InstanceSettingsPage::List;
                                        app.status_message = "Back to Settings list".to_string();
                                        continue;
                                    }
                                    app.current_tab = app::TabId::Instances;
                                    app.status_message = "Returned to instances list".to_string();
                                    continue;
                                }
                                // Navigation within Instance Settings
                                KeyCode::Up | KeyCode::Char('k')
                                    if app.current_tab == app::TabId::InstanceSettings =>
                                {
                                    app.navigate_instance_settings(-1);
                                    continue;
                                }
                                KeyCode::Down | KeyCode::Char('j')
                                    if app.current_tab == app::TabId::InstanceSettings =>
                                {
                                    app.navigate_instance_settings(1);
                                    continue;
                                }
                                KeyCode::Left | KeyCode::Char('h')
                                    if app.current_tab == app::TabId::InstanceSettings =>
                                {
                                    app.prev_instance_settings_tab();
                                    continue;
                                }
                                KeyCode::Right
                                    if app.current_tab == app::TabId::InstanceSettings =>
                                {
                                    app.next_instance_settings_tab();
                                    continue;
                                }
                                KeyCode::Enter
                                    if app.current_tab == app::TabId::InstanceSettings =>
                                {
                                    if app.instance_settings_tab
                                        == app::InstanceSettingsTab::Setting
                                        && app.instance_settings_page
                                            == app::InstanceSettingsPage::Java
                                    {
                                        app.select_in_java_page();
                                        continue;
                                    }
                                    if app.instance_settings_tab
                                        == app::InstanceSettingsTab::Setting
                                        && app.instance_settings_page
                                            == app::InstanceSettingsPage::Launch
                                    {
                                        app.select_in_launch_page();
                                        continue;
                                    }
                                    app.select_instance_settings_item();
                                    continue;
                                }
                                // Settings: Licenses focus management
                                KeyCode::Left | KeyCode::Char('h')
                                    if app.current_tab == app::TabId::Settings
                                        && app.about_selected
                                            == app::App::licenses_menu_index() =>
                                {
                                    app.settings_focus = app::SettingsFocus::Left;
                                    continue;
                                }
                                KeyCode::Right
                                    if app.current_tab == app::TabId::Settings
                                        && app.about_selected
                                            == app::App::licenses_menu_index() =>
                                {
                                    app.settings_focus = app::SettingsFocus::Middle;
                                    continue;
                                }
                                KeyCode::Enter
                                    if app.current_tab == app::TabId::Settings
                                        && app.about_selected
                                            == app::App::licenses_menu_index() =>
                                {
                                    app.settings_focus = app::SettingsFocus::Middle;
                                    continue;
                                }
                                // Settings General/Java: allow focusing middle pane to show submenu and activate editing
                                KeyCode::Right
                                    if app.current_tab == app::TabId::Settings
                                        && (app.about_selected == 0 || app.about_selected == 1) =>
                                {
                                    app.settings_focus = app::SettingsFocus::Middle;
                                    continue;
                                }
                                KeyCode::Left | KeyCode::Char('h')
                                    if app.current_tab == app::TabId::Settings
                                        && (app.about_selected == 0 || app.about_selected == 1) =>
                                {
                                    app.settings_focus = app::SettingsFocus::Left;
                                    continue;
                                }
                                // Settings -> Java/General: Enter behavior depends on focus
                                KeyCode::Enter
                                    if app.current_tab == app::TabId::Settings
                                        && (app.about_selected == 1 || app.about_selected == 0) =>
                                {
                                    if app.settings_focus == app::SettingsFocus::Middle {
                                        if app.about_selected == 1 {
                                            app.open_global_java_args_edit();
                                        } else {
                                            // General submenu has multiple items; open based on selection
                                            match app.general_selected {
                                                0 => app.open_global_window_size_edit(),
                                                1 => app.open_tui_refresh_interval_edit(),
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        // From left pane, move focus to middle instead of opening popup
                                        app.settings_focus = app::SettingsFocus::Middle;
                                    }
                                    continue;
                                }
                                // Settings -> General submenu navigation (Up/Down)
                                KeyCode::Up
                                    if app.current_tab == app::TabId::Settings
                                        && app.about_selected == 0
                                        && app.settings_focus == app::SettingsFocus::Middle =>
                                {
                                    app.general_selected = app.general_selected.saturating_sub(1);
                                    continue;
                                }
                                KeyCode::Down
                                    if app.current_tab == app::TabId::Settings
                                        && app.about_selected == 0
                                        && app.settings_focus == app::SettingsFocus::Middle =>
                                {
                                    app.general_selected = (app.general_selected + 1).min(1);
                                    continue;
                                }
                                // F12: force redraw
                                KeyCode::F(12) => {
                                    terminal.clear()?;
                                    app.status_message = "🔄 Terminal refreshed".to_string();
                                    continue;
                                }
                                _ => {}
                            }

                            // 5) Delegate remaining keys to the shared input handler
                            let quit = handlers::input::handle_key_event(&mut app, key);
                            if quit {
                                return Ok(());
                            }
                        }
                    }
                    Event::Mouse(me) => match me.kind {
                        MouseEventKind::ScrollUp => {
                            if app.current_tab == app::TabId::Settings {
                                app.about_scroll = app.about_scroll.saturating_sub(3);
                            }
                            if app.current_tab == app::TabId::Logs {
                                app.logs_auto_follow = false;
                                app.logs_offset = app.logs_offset.saturating_add(3);
                            }
                            if app.current_tab == app::TabId::InstanceSettings
                                && app.instance_settings_tab == app::InstanceSettingsTab::Logs
                            {
                                app.instance_logs_auto_follow = false;
                                app.instance_logs_offset =
                                    app.instance_logs_offset.saturating_add(3);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.current_tab == app::TabId::Settings {
                                app.about_scroll = app.about_scroll.saturating_add(3);
                            }
                            if app.current_tab == app::TabId::Logs {
                                app.logs_offset = app.logs_offset.saturating_sub(3);
                                if app.logs_offset == 0 {
                                    app.logs_auto_follow = true;
                                }
                            }
                            if app.current_tab == app::TabId::InstanceSettings
                                && app.instance_settings_tab == app::InstanceSettingsTab::Logs
                            {
                                app.instance_logs_offset =
                                    app.instance_logs_offset.saturating_sub(3);
                                if app.instance_logs_offset == 0 {
                                    app.instance_logs_auto_follow = true;
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }
    Ok(())
}
