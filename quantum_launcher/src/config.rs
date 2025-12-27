use crate::stylesheet::styles::{LauncherTheme, LauncherThemeColor, LauncherThemeLightness};
use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};
use ql_core::json::GlobalSettings;
use ql_core::ListEntryKind;
use ql_core::{
    err, IntoIoError, IntoJsonError, JsonFileError, LAUNCHER_DIR, LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, path::Path};

pub const SIDEBAR_WIDTH: f32 = 0.33;
const OPACITY: f32 = 0.9;

/// Global launcher configuration stored in
/// `QuantumLauncher/config.json`.
///
/// For more info on the launcher directory see
/// <https://mrmayman.github.io/quantumlauncher#files-location>
///
/// # Why `Option`?
///
/// Many fields are `Option`s for backwards compatibility.
/// If upgrading from an older version,
/// `serde` will deserialize missing fields as `None`,
/// which is treated as a default value.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherConfig {
    /// The offline username set by the player when playing Minecraft.
    pub username: String,

    #[deprecated(
        since = "0.2.0",
        note = "removed feature, field left here for backwards compatibility"
    )]
    pub java_installs: Option<Vec<String>>,

    /// UI mode (Light/Dark/Auto) set by the user.
    // Since: v0.3
    #[serde(rename = "theme")]
    pub ui_mode: Option<LauncherThemeLightness>,
    /// UI color theme
    // Since: v0.3
    #[serde(rename = "style")]
    pub ui_theme: Option<LauncherThemeColor>,

    /// The launcher version when you last opened it
    // Since: v0.3
    pub version: Option<String>,

    /// A list of Minecraft accounts logged into the launcher.
    ///
    /// `String (username) : ConfigAccount { uuid: String, skin: None (unimplemented) }`
    ///
    /// Upon opening the launcher,
    /// `read_refresh_token(username)` (in [`ql_instances::auth`])
    /// is called on each account's key value (username)
    /// to get the refresh token (stored securely on disk).
    // Since: v0.4
    pub accounts: Option<HashMap<String, ConfigAccount>>,
    /// Refers to the entry of the `accounts` map
    /// that's selected in the UI when you open the launcher.
    // Since: v0.4.2
    pub account_selected: Option<String>,

    /// The scale of the UI, i.e. how big everything is.
    ///
    /// - above 1.0: More zoomed in buttons/text/etc.
    ///   Useful for high DPI displays or bad eyesight
    /// - 1.0: default
    /// - 0.0-1.0: Zoomed out, smaller UI elements
    // Since: v0.4
    pub ui_scale: Option<f64>,

    /// Whether to enable antialiasing or not.
    /// Minor improvement in visual quality,
    /// also nudges launcher to use dedicated GPU
    /// for the interface.
    ///
    /// Default: `true`
    // Since: v0.4.2
    #[serde(rename = "antialiasing")]
    pub ui_antialiasing: Option<bool>,
    /// Many launcher window related config options.
    // Since: v0.4.2
    pub window: Option<WindowProperties>,

    /// Settings that apply both on a per-instance basis and with global overrides.
    // Since: v0.4.2
    pub global_settings: Option<GlobalSettings>,
    // Since: v0.5.0
    pub extra_java_args: Option<Vec<String>>,
    // Since: v0.5.0
    pub ui: Option<UiSettings>,
    // Since: v0.5.0
    pub persistent: Option<PersistentSettings>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        #[allow(deprecated)]
        Self {
            username: String::new(),
            ui_mode: None,
            ui_theme: None,
            version: Some(LAUNCHER_VERSION_NAME.to_owned()),
            accounts: None,
            ui_scale: None,
            java_installs: Some(Vec::new()),
            ui_antialiasing: Some(true),
            account_selected: None,
            window: None,
            global_settings: None,
            extra_java_args: None,
            ui: None,
            persistent: None,
        }
    }
}

impl LauncherConfig {
    /// Load the launcher configuration.
    ///
    /// # Errors
    /// - if the user doesn't have permission to access launcher directory
    ///
    /// This function is designed to *not* fail fast,
    /// resetting the config if it's nonexistent or corrupted
    /// (with an error log message).
    pub fn load_s() -> Result<Self, JsonFileError> {
        let config_path = LAUNCHER_DIR.join("config.json");
        if !config_path.exists() {
            return LauncherConfig::create(&config_path);
        }

        let mut config = std::fs::read_to_string(&config_path).path(&config_path)?;
        if config.is_empty() {
            for _ in 0..5 {
                config = std::fs::read_to_string(&config_path).path(&config_path)?;
                if !config.is_empty() {
                    break;
                }
            }
        }
        let mut config: Self = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(err) => {
                err!("Invalid launcher config! This may be a sign of corruption! Please report if this happens to you.\nError: {err}");
                let old_path = LAUNCHER_DIR.join("config.json.bak");
                _ = std::fs::copy(&config_path, &old_path);
                return LauncherConfig::create(&config_path);
            }
        };
        config.fix();

        Ok(config)
    }

    pub async fn save(&self) -> Result<(), JsonFileError> {
        let config_path = LAUNCHER_DIR.join("config.json");
        let config = serde_json::to_string(&self).json_to()?;

        tokio::fs::write(&config_path, config.as_bytes())
            .await
            .path(config_path)?;
        Ok(())
    }

    fn create(path: &Path) -> Result<Self, JsonFileError> {
        let mut config = LauncherConfig::default();
        config.fix();
        std::fs::write(path, serde_json::to_string(&config).json_to()?.as_bytes()).path(path)?;
        Ok(config)
    }

    fn fix(&mut self) {
        if self.ui_antialiasing.is_none() {
            self.ui_antialiasing = Some(true);
        }
        if let (Some(accounts), Some(selected)) = (&self.accounts, &self.account_selected) {
            if !accounts.contains_key(selected) {
                self.account_selected = None;
            }
        }

        #[allow(deprecated)]
        {
            if self.java_installs.is_none() {
                self.java_installs = Some(Vec::new());
            }
        }
    }

    pub fn c_window_size(&self) -> (f32, f32) {
        let window = self.window.clone().unwrap_or_default();
        let scale = self.ui_scale.unwrap_or(1.0) as f32;
        let window_width = window
            .width
            .filter(|_| window.save_window_size)
            .unwrap_or(WINDOW_WIDTH * scale);
        let window_height = window.height.filter(|_| window.save_window_size).unwrap_or(
            (WINDOW_HEIGHT
                + if self.uses_system_decorations() {
                    0.0
                } else {
                    30.0
                })
                * scale,
        );
        (window_width, window_height)
    }

    pub fn c_ui_opacity(&self) -> f32 {
        self.ui.as_ref().map_or(OPACITY, |n| n.window_opacity)
    }

    pub fn uses_system_decorations(&self) -> bool {
        self.ui
            .as_ref()
            .map(|n| matches!(n.window_decorations, UiWindowDecorations::System))
            .unwrap_or(true) // change this to false when enabling the experimental decorations
    }

    pub fn c_theme(&self) -> LauncherTheme {
        LauncherTheme {
            lightness: self.ui_mode.unwrap_or_default(),
            color: self.ui_theme.unwrap_or_default(),
            alpha: self.c_ui_opacity(),
            system_dark_mode: dark_light::detect()
                .map(|n| n == dark_light::Mode::Dark)
                .unwrap_or_default(),
        }
    }

    pub fn c_window(&mut self) -> &mut WindowProperties {
        self.window.get_or_insert_with(WindowProperties::default)
    }

    pub fn c_global(&mut self) -> &mut GlobalSettings {
        self.global_settings
            .get_or_insert_with(GlobalSettings::default)
    }

    pub fn c_persistent(&mut self) -> &mut PersistentSettings {
        self.persistent
            .get_or_insert_with(PersistentSettings::default)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigAccount {
    /// UUID of the Minecraft account. Stored as a string without dashes.
    ///
    /// Example: `2553495fc9094d40a82646cfc92cd7a5`
    ///
    /// A UUID is like an alternate username that can be used to identify
    /// an account. Unlike a username it can't be changed, so it's useful for
    /// dealing with account data in a stable manner.
    ///
    /// You can find someone's UUID through many online services where you
    /// input their username.
    pub uuid: String,
    /// Currently unimplemented, does nothing.
    pub skin: Option<String>, // TODO: Add skin visualization?

    /// Type of account:
    ///
    /// - `"Microsoft"`
    /// - `"ElyBy"`
    /// - `"LittleSkin"`
    pub account_type: Option<String>,

    /// The original login identifier used for keyring operations.
    /// This is the email address or username that was used during login.
    /// For email/password logins, this will be the email.
    /// For username/password logins, this will be the username.
    pub keyring_identifier: Option<String>,

    /// A game-readable "nice" username.
    ///
    /// This will be identical to the regular
    /// username of the account in most cases
    /// except for the case where the user
    /// has an `ely.by` account with an email.
    /// In that case, this will be the actual
    /// username while the regular "username"
    /// would be an email.
    pub username_nice: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowProperties {
    /// Whether to retain window size in the first place.
    // Since: v0.4.2
    pub save_window_size: bool,

    /// The width of the window when the launcher was last closed.
    /// Used to restore the window size between launches.
    // Since: v0.4.2
    pub width: Option<f32>,
    /// The height of the window when the launcher was last closed.
    /// Used to restore the window size between launches.
    // Since: v0.4.2
    pub height: Option<f32>,
}

impl Default for WindowProperties {
    fn default() -> Self {
        Self {
            save_window_size: true,
            width: None,
            height: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct UiSettings {
    // Since: v0.5.0
    pub window_decorations: UiWindowDecorations,
    // Since: v0.5.0
    pub window_opacity: f32,
    // Since: v0.5.0
    pub idle_fps: Option<u64>,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            window_decorations: UiWindowDecorations::default(),
            window_opacity: OPACITY,
            idle_fps: None,
        }
    }
}

impl UiSettings {
    pub fn get_idle_fps(&self) -> u64 {
        self.idle_fps.unwrap_or(6)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub enum UiWindowDecorations {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

/*impl Default for UiWindowDecorations {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        return Self::Left;
        #[cfg(not(target_os = "macos"))]
        Self::Right
    }
}*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistentSettings {
    pub selected_instance: Option<String>,
    pub selected_server: Option<String>,
    pub selected_remembered: bool,

    /// Remembers version filters (eg: snapshot, release, etc) in Create Instance
    pub create_instance_filters: Option<HashSet<ListEntryKind>>,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            selected_instance: None,
            selected_server: None,
            selected_remembered: true,
            create_instance_filters: None,
        }
    }
}

impl PersistentSettings {
    #[must_use]
    pub fn get_create_instance_filters(&self) -> std::collections::HashSet<ListEntryKind> {
        self.create_instance_filters
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(ListEntryKind::default_selected)
    }
}
