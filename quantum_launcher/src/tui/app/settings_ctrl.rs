// Settings tab navigation and license helpers

use crate::tui::app::{App, SettingsFocus};

impl App {
    // Index of the combined "About & Licenses" entry in the left settings menu
    pub const fn licenses_menu_index() -> usize {
        crate::tui::app::licenses::menu_index()
    }

    pub fn licenses() -> &'static [(&'static str, &'static [&'static str])] {
        crate::tui::app::licenses::entries()
    }

    /// Compile-time embedded license text fallbacks (mirrors the Iced UI behavior)
    /// Index corresponds to `Self::licenses()` ordering
    pub fn license_fallback_content(index: usize) -> Option<&'static str> {
        crate::tui::app::licenses::fallback_content(index)
    }

    /// Move selection forward within Settings depending on focused pane
    pub fn settings_next_item(&mut self) {
        // Settings left menu (General, Java, UI/Theme, Launch, About & Licenses = 5 items)
        if self.settings_focus == SettingsFocus::Middle
            && self.about_selected == Self::licenses_menu_index()
        {
            // license_selected uses 0 = About, 1..=N = licenses; allow cycling through About + N licenses
            let count = Self::licenses().len() + 1; // +1 for the About entry
            if count > 0 {
                self.license_selected = (self.license_selected + 1) % count;
            }
        } else {
            let left_count = Self::licenses_menu_index() + 1; // 0..=4 => 5 items
            self.about_selected = if left_count > 0 {
                (self.about_selected + 1) % left_count
            } else {
                0
            };
            self.about_scroll = 0;
        }
    }

    /// Move selection backward within Settings depending on focused pane
    pub fn settings_prev_item(&mut self) {
        if self.settings_focus == SettingsFocus::Middle
            && self.about_selected == Self::licenses_menu_index()
        {
            // Match next_item behavior: include About as an option (0), so total = licenses.len() + 1
            let count = Self::licenses().len() + 1;
            if count > 0 {
                if self.license_selected > 0 {
                    self.license_selected -= 1;
                } else {
                    self.license_selected = count - 1;
                }
            }
        } else {
            let left_last = Self::licenses_menu_index(); // 4
            if self.about_selected > 0 {
                self.about_selected -= 1;
            } else {
                self.about_selected = left_last;
            }
            self.about_scroll = 0;
        }
    }
}
