// QuantumLauncher TUI - Application State

use crate::state::ClientProcess;
use ql_core::ListEntry;
use std::{collections::HashMap, error::Error, fmt};
use tokio::sync::mpsc;

pub type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    Microsoft,
    ElyBy,
    LittleSkin,
    Offline,
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            AccountType::Microsoft => "Microsoft (Coming Soon)",
            AccountType::ElyBy => "ElyBy",
            AccountType::LittleSkin => "LittleSkin (Coming Soon)",
            AccountType::Offline => "Offline",
        };
        write!(f, "{}", name)
    }
}

impl AccountType {
    pub fn all() -> Vec<AccountType> {
        vec![
            AccountType::Microsoft,
            AccountType::ElyBy,
            AccountType::LittleSkin,
            AccountType::Offline,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TabId {
    Instances,
    Create,
    Settings,
    Accounts,
    Logs,
    InstanceSettings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AddAccountFieldFocus {
    Username,
    Password,
    Otp,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub username: String,
    pub account_type: String,
    #[allow(dead_code)]
    pub uuid: String,
    pub is_logged_in: bool,
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TabId::Instances => write!(f, "Instances"),
            TabId::Create => write!(f, "Create"),
            TabId::Settings => write!(f, "Settings"),
            TabId::Accounts => write!(f, "Accounts"),
            TabId::Logs => write!(f, "Logs"),
            TabId::InstanceSettings => write!(f, "Instance Settings"),
        }
    }
}

#[derive(Debug)]
pub struct Instance {
    pub name: String,
    pub version: String,
    pub loader: String,
    pub is_running: bool,
}

#[derive(Debug)]
pub struct App {
    pub current_tab: TabId,
    pub instances: Vec<Instance>,
    pub selected_instance: usize,
    pub available_versions: Vec<ListEntry>,
    // Version search/filtering (Create tab)
    pub version_search_active: bool,
    pub version_search_query: String,
    pub filtered_versions: Vec<ListEntry>,
    pub selected_filtered_version: usize,
    // Version type filters
    pub filter_release: bool,
    pub filter_snapshot: bool,
    pub filter_beta: bool,
    pub filter_alpha: bool,
    pub new_instance_name: String,
    pub is_editing_name: bool,
    pub download_assets: bool,
    pub show_help_popup: bool,
    pub accounts: Vec<Account>,
    pub selected_account: usize,
    pub current_account: Option<String>,
    pub status_message: String,
    pub should_quit: bool,
    pub is_loading: bool,
    // New account creation fields
    pub is_add_account_mode: bool,
    pub new_account_type: AccountType,
    pub new_account_username: String,
    pub new_account_password: String,
    pub selected_account_type: usize,
    // ElyBy specific fields for account creation
    pub new_account_otp: Option<String>,
    pub needs_otp: bool,
    pub show_password: bool,
    pub login_error: Option<String>,
    pub add_account_field_focus: AddAccountFieldFocus, // Track which field is being edited during account creation
    // Auth channel for async authentication
    pub auth_sender: Option<mpsc::UnboundedSender<crate::tui::AuthEvent>>,
    // Game logs storage
    pub game_logs: Vec<String>,
    // Logs view state
    pub logs_offset: usize, // number of lines scrolled up from the bottom (0 = bottom)
    pub logs_visible_lines: usize, // lines that fit in viewport (updated by renderer)
    pub logs_auto_follow: bool, // auto follow new logs when at bottom
    // Terminal refresh tracking
    pub needs_forced_refresh: bool,
    // Instance settings fields
    pub instance_settings_instance: Option<usize>, // Index of the instance being viewed in settings
    pub instance_settings_tab: InstanceSettingsTab, // Current tab in instance settings
    pub instance_settings_selected: usize,         // Selected item in instance settings
    pub instance_settings_page: InstanceSettingsPage, // Which subpage inside Settings tab
    // Process tracking for kill functionality
    pub client_processes: HashMap<String, ClientProcess>, // Track running instances
    pub show_delete_confirm: bool, // Show confirmation popup for instance deletion
    // About page state
    pub about_selected: usize, // Left menu selection index
    pub about_scroll: u16,     // Right content scroll offset
    // Settings → Licenses submenu state
    pub settings_focus: SettingsFocus, // Which pane is focused in Settings
    pub license_selected: usize,       // Selected license in the Licenses submenu
    // Settings → General submenu selection (e.g., which item in middle pane)
    pub general_selected: usize,
    // Per-instance logs (instance name -> recent lines)
    pub instance_logs: HashMap<String, Vec<String>>,
    // Per-instance logs view state
    pub instance_logs_offset: usize, // number of lines scrolled up from bottom (0 = bottom)
    pub instance_logs_visible_lines: usize, // lines that fit in viewport (set by renderer)
    pub instance_logs_auto_follow: bool, // follow new lines when at bottom
    // Rename instance popup state
    pub is_renaming_instance: bool, // whether rename popup is active
    pub rename_input: String,       // buffer for new name while typing
    // Memory allocation edit popup state
    pub is_editing_memory: bool,   // whether memory edit popup is active
    pub memory_edit_mb: usize,     // working value for RAM in MB
    pub memory_edit_input: String, // text buffer for manual entry
    // Java/Game args edit popup state
    pub is_editing_args: bool,        // whether args edit popup is active
    pub args_edit_input: String,      // text buffer for args (space-separated, supports quotes)
    pub args_edit_kind: ArgsEditKind, // which args are being edited
    // Current Java args mode (for display and quick changes)
    pub java_args_mode_current: ql_core::json::JavaArgsMode,
    // Configurable TUI refresh interval (ms); None => use default
    pub tui_refresh_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceSettingsTab {
    Overview,
    Mod,
    Setting,
    Logs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceSettingsPage {
    List,
    Java,
    Launch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ArgsEditKind {
    Java,
    Game,
    GlobalJava,
    WindowSize,
    GlobalWindowSize,
    GlobalTuiRefreshInterval,
    PreLaunchPrefixInstance,
    PreLaunchPrefixGlobal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    Left,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCategory {
    Release,
    Snapshot,
    Beta,
    Alpha,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            current_tab: TabId::Instances,
            instances: Vec::new(),
            selected_instance: 0,
            available_versions: Vec::new(),
            version_search_active: false,
            version_search_query: String::new(),
            filtered_versions: Vec::new(),
            selected_filtered_version: 0,
            filter_release: true,
            filter_snapshot: true,
            filter_beta: true,
            filter_alpha: true,
            new_instance_name: String::new(),
            is_editing_name: false,
            download_assets: true, // Default to true like in Iced UI
            show_help_popup: false,
            accounts: Vec::new(),
            selected_account: 0,
            current_account: None,
            status_message: "Welcome to QuantumLauncher TUI! Press '?' for help, 'q' to quit."
                .to_string(),
            should_quit: false,
            is_loading: false,
            // Initialize new account creation fields
            is_add_account_mode: false,
            new_account_type: AccountType::ElyBy, // Default to ElyBy as the primary working option
            new_account_username: String::new(),
            new_account_password: String::new(),
            selected_account_type: 0,
            // Initialize ElyBy specific fields for account creation
            new_account_otp: None,
            needs_otp: false,
            show_password: false,
            login_error: None,
            add_account_field_focus: AddAccountFieldFocus::Username,
            auth_sender: None,
            game_logs: Vec::new(),
            logs_offset: 0,
            logs_visible_lines: 20,
            logs_auto_follow: true,
            needs_forced_refresh: false,
            // Initialize instance settings fields
            instance_settings_instance: None,
            instance_settings_tab: InstanceSettingsTab::Overview,
            instance_settings_selected: 0,
            instance_settings_page: InstanceSettingsPage::List,
            // Initialize client process tracking
            client_processes: HashMap::new(),
            show_delete_confirm: false,
            // About
            about_selected: 0,
            about_scroll: 0,
            settings_focus: SettingsFocus::Left,
            license_selected: 0,
            general_selected: 0,
            instance_logs: HashMap::new(),
            instance_logs_offset: 0,
            instance_logs_visible_lines: 20,
            instance_logs_auto_follow: true,
            // Rename popup
            is_renaming_instance: false,
            rename_input: String::new(),
            // Memory popup
            is_editing_memory: false,
            memory_edit_mb: 2048,
            memory_edit_input: String::new(),
            // Args popup
            is_editing_args: false,
            args_edit_input: String::new(),
            args_edit_kind: ArgsEditKind::Java,
            java_args_mode_current: ql_core::json::JavaArgsMode::Combine,
            tui_refresh_interval_ms: None,
        };

        // Load instances and accounts on startup
        app.refresh();
        app.load_accounts();
        // Load refresh interval from global config if present
        if let Ok(cfg) = crate::config::LauncherConfig::load_s() {
            app.tui_refresh_interval_ms = cfg
                .global_settings
                .as_ref()
                .and_then(|gs| gs.tui_refresh_interval_ms);
        }
        app
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    #[allow(dead_code)]
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabId::Instances => TabId::Create,
            TabId::Create => TabId::Settings,
            TabId::Settings => TabId::Accounts,
            TabId::Accounts => TabId::Logs,
            TabId::Logs => TabId::Instances,
            TabId::InstanceSettings => TabId::Instances,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabId::Instances => TabId::Logs,
            TabId::Create => TabId::Instances,
            TabId::Settings => TabId::Create,
            TabId::Accounts => TabId::Settings,
            TabId::Logs => TabId::Accounts,
            TabId::InstanceSettings => TabId::Instances,
        };
    }

    // set_tab removed: tabs are changed via next/previous or specific actions

    pub fn next_item(&mut self) {
        match self.current_tab {
            TabId::Instances => {
                if !self.instances.is_empty() {
                    self.selected_instance = (self.selected_instance + 1) % self.instances.len();
                }
            }
            TabId::Create => {
                if !self.filtered_versions.is_empty() {
                    self.selected_filtered_version =
                        (self.selected_filtered_version + 1) % self.filtered_versions.len();
                }
            }
            TabId::Accounts => {
                if !self.accounts.is_empty() {
                    self.selected_account = (self.selected_account + 1) % self.accounts.len();
                }
            }
            TabId::Settings => {
                self.settings_next_item();
            }
            _ => {}
        }
    }

    pub fn prev_item(&mut self) {
        match self.current_tab {
            TabId::Instances => {
                if !self.instances.is_empty() && self.selected_instance > 0 {
                    self.selected_instance -= 1;
                } else if !self.instances.is_empty() {
                    self.selected_instance = self.instances.len() - 1;
                }
            }
            TabId::Create => {
                if !self.filtered_versions.is_empty() && self.selected_filtered_version > 0 {
                    self.selected_filtered_version -= 1;
                } else if !self.filtered_versions.is_empty() {
                    self.selected_filtered_version = self.filtered_versions.len() - 1;
                }
            }
            TabId::Accounts => {
                if !self.accounts.is_empty() && self.selected_account > 0 {
                    self.selected_account -= 1;
                } else if !self.accounts.is_empty() {
                    self.selected_account = self.accounts.len() - 1;
                }
            }
            TabId::Settings => {
                self.settings_prev_item();
            }
            _ => {}
        }
    }

    // Settings: helpers moved to tui/app/settings_ctrl.rs

    pub fn select_item(&mut self) {
        match self.current_tab {
            TabId::Instances => {
                // Open instance settings page instead of launching directly
                self.instance_settings_instance = Some(self.selected_instance);
                self.instance_settings_tab = InstanceSettingsTab::Overview;
                self.instance_settings_selected = 0;
                self.current_tab = TabId::InstanceSettings;
                self.status_message = format!(
                    "Opened settings for instance: {}",
                    self.instances[self.selected_instance].name
                );
            }
            TabId::Create => {
                // Create tab should not use select_item - use create_instance instead
                self.status_message =
                    "Use Enter to create instance or Ctrl+N to edit name".to_string();
            }
            _ => {}
        }
    }

    // start_refresh and refresh moved to tui/app/instances_ctrl.rs

    pub fn toggle_help_popup(&mut self) {
        self.show_help_popup = !self.show_help_popup;
    }

    pub fn set_auth_channel(&mut self, sender: mpsc::UnboundedSender<crate::tui::AuthEvent>) {
        self.auth_sender = Some(sender);
    }

    /// Check if forced refresh is needed and reset the flag
    pub fn check_and_reset_forced_refresh(&mut self) -> bool {
        let needs_refresh = self.needs_forced_refresh;
        self.needs_forced_refresh = false;
        needs_refresh
    }
}
