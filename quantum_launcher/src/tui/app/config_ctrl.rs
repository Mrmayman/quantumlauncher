// Config helpers: saving config synchronously for TUI flows

use crate::config::LauncherConfig;
use crate::tui::app::App;

impl App {
    /// Save config synchronously (helper for TUI)
    pub(crate) fn save_config_sync(&self, config: &LauncherConfig) -> Result<(), String> {
        let config_path = match ql_core::file_utils::get_launcher_dir() {
            Ok(dir) => dir.join("config.json"),
            Err(e) => return Err(format!("Failed to get launcher directory: {}", e)),
        };
        let config_str = match serde_json::to_string_pretty(config) {
            Ok(str) => str,
            Err(e) => return Err(format!("Failed to serialize config: {}", e)),
        };

        match std::fs::write(&config_path, config_str) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to write config: {}", e)),
        }
    }

    /// Open popup to edit global TUI refresh interval (ms)
    pub fn open_tui_refresh_interval_edit(&mut self) {
        use crate::tui::app::ArgsEditKind;
        let cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        let val = cfg
            .global_settings
            .as_ref()
            .and_then(|gs| gs.tui_refresh_interval_ms)
            .map(|n| n.to_string())
            .unwrap_or_default();
        self.is_editing_args = true;
        self.args_edit_input = val;
        self.args_edit_kind = ArgsEditKind::GlobalTuiRefreshInterval;
        self.status_message = "Editing TUI refresh interval (ms; empty for default)".to_string();
    }

    /// Save popup buffer into global TUI refresh interval
    pub fn apply_tui_refresh_interval_edit(&mut self) {
        use crate::tui::app::ArgsEditKind;
        if !self.is_editing_args {
            return;
        }
        if self.args_edit_kind != ArgsEditKind::GlobalTuiRefreshInterval {
            return;
        }
        let mut cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        let txt = self.args_edit_input.trim();
        if txt.is_empty() {
            if let Some(gs) = cfg.global_settings.as_mut() {
                gs.tui_refresh_interval_ms = None;
            }
            self.tui_refresh_interval_ms = None;
        } else {
            match txt.parse::<u64>() {
                Ok(n) => {
                    // 0 disables periodic refresh; otherwise clamp to sensible minimum
                    let n = if n == 0 { 0 } else { n.max(50) };
                    let gs = cfg.global_settings.get_or_insert_with(Default::default);
                    gs.tui_refresh_interval_ms = Some(n);
                    self.tui_refresh_interval_ms = Some(n);
                }
                Err(_) => {
                    self.status_message =
                        "❌ Invalid number; please enter an integer in milliseconds.".to_string();
                    return;
                }
            }
        }
        match self.save_config_sync(&cfg) {
            Ok(_) => {
                self.is_editing_args = false;
                self.status_message = "✅ Saved TUI refresh interval".to_string();
            }
            Err(e) => {
                self.status_message = format!("❌ Failed to save global config: {}", e);
            }
        }
    }

    /// Open popup to edit global Java arguments (LauncherConfig.extra_java_args)
    pub fn open_global_java_args_edit(&mut self) {
        use crate::tui::app::ArgsEditKind;
        let cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        let vec = cfg.extra_java_args.unwrap_or_default();
        let text = vec
            .iter()
            .map(|a| {
                if a.is_empty() {
                    "\"\"".to_string()
                } else if a.chars().any(|c| c.is_whitespace() || c == ',') {
                    format!("\"{}\"", a.replace('"', "\\\""))
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",");

        self.is_editing_args = true;
        self.args_edit_input = text;
        self.args_edit_kind = ArgsEditKind::GlobalJava;
        self.status_message = "Editing global Java arguments".to_string();
    }

    /// Save args popup buffer into global LauncherConfig.extra_java_args
    pub fn apply_global_java_args_edit(&mut self) {
        if !self.is_editing_args {
            return;
        }
        if self.args_edit_kind != crate::tui::app::ArgsEditKind::GlobalJava {
            return;
        }
        let parsed = super::instances_ctrl::parse_shell_like_args(self.args_edit_input.trim());
        let mut cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        cfg.extra_java_args = Some(parsed);
        match self.save_config_sync(&cfg) {
            Ok(_) => {
                self.is_editing_args = false;
                self.status_message = "✅ Saved global Java arguments".to_string();
            }
            Err(e) => {
                self.status_message = format!("❌ Failed to save global config: {}", e);
            }
        }
    }

    /// Open popup to edit global window size (WIDTH,HEIGHT) in LauncherConfig.global_settings
    pub fn open_global_window_size_edit(&mut self) {
        use crate::tui::app::ArgsEditKind;
        let cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        let (w, h) = cfg
            .global_settings
            .as_ref()
            .map(|gs| (gs.window_width, gs.window_height))
            .unwrap_or((None, None));
        let text = match (w, h) {
            (Some(w), Some(h)) => format!("{},{}", w, h),
            _ => String::new(),
        };
        self.is_editing_args = true;
        self.args_edit_input = text;
        self.args_edit_kind = ArgsEditKind::GlobalWindowSize;
        self.status_message =
            "Editing global window size (WIDTH,HEIGHT; empty to clear)".to_string();
    }

    /// Save popup buffer into global window size in LauncherConfig.global_settings
    pub fn apply_global_window_size_edit(&mut self) {
        use crate::tui::app::ArgsEditKind;
        if self.args_edit_kind != ArgsEditKind::GlobalWindowSize {
            return;
        }
        let mut cfg = match LauncherConfig::load_s() {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("❌ Failed to load global config: {}", e);
                return;
            }
        };
        let txt = self.args_edit_input.trim();
        if txt.is_empty() {
            if let Some(gs) = cfg.global_settings.as_mut() {
                gs.window_width = None;
                gs.window_height = None;
            }
        } else {
            let parts: Vec<&str> = txt.split([',', 'x', 'X']).collect();
            if parts.len() >= 2 {
                let w = parts[0].trim().parse::<u32>().ok();
                let h = parts[1].trim().parse::<u32>().ok();
                let gs = cfg.global_settings.get_or_insert_with(Default::default);
                gs.window_width = w;
                gs.window_height = h;
            }
        }
        match self.save_config_sync(&cfg) {
            Ok(_) => {
                self.is_editing_args = false;
                self.status_message = "✅ Saved global window size".to_string();
            }
            Err(e) => {
                self.status_message = format!("❌ Failed to save global config: {}", e);
            }
        }
    }
}
