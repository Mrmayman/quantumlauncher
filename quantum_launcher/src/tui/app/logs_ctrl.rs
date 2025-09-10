// Game logs helpers for TUI: write-through to buffer and core logger

use crate::tui::app::App;

impl App {
    /// Add a log line to game logs
    pub fn add_log(&mut self, log_line: String) {
        // Push to local buffer for immediate rendering and cap size
        crate::tui::app::logs::push_capped(&mut self.game_logs, log_line.clone(), 1000);
        // Mirror into core in-memory logger so external readers (and future sessions) see it
        crate::tui::app::logs::mirror_to_core(log_line);
    }

    /// Clear game logs
    pub fn clear_logs(&mut self) {
        self.game_logs.clear();
    }
}
