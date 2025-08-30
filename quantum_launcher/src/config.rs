use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};
use ql_core::json::GlobalSettings;
use ql_core::{
    err, IntoIoError, IntoJsonError, JsonFileError, LAUNCHER_DIR, LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

pub const SIDEBAR_WIDTH_DEFAULT: u32 = 190;

/// The global launcher configuration.
///
/// This is stored in the launcher directory
/// (`QuantumLauncher/`) as `config.json`.
///
/// For more info on the launcher directory see
/// <https://mrmayman.github.io/quantumlauncher#files-location>
///
/// # Why `Option`?
///
/// Note: many fields here are `Option`s. This is for
/// backwards-compatibility, as if you upgrade from an older
/// version without these fields, `serde` will safely serialize
/// them as `None`.
///
/// So generally `None` is interpreted as a default value
/// put there when migrating from a version without the feature.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherConfig {
    /// The offline username set by the player when playing Minecraft.
    pub username: String,

    #[deprecated(
        since = "0.2.0",
        note = "removed feature, field left here for backwards compatibility"
    )]
    pub java_installs: Option<Vec<String>>,

    /// The theme (Light/Dark) set by the user.
    // Since: v0.3
    pub theme: Option<String>,
    /// The color scheme set by the user.
    ///
    /// Valid options are:
    /// - Purple
    /// - Brown
    /// - Sky Blue
    /// - Catppuccin
    /// - Teal
    // Since: v0.3
    pub style: Option<String>,

    /// The version that the launcher was last time
    /// you opened it.
    // Since: v0.3
    pub version: Option<String>,

    /// The width of the sidebar in the main menu
    /// (which shows the list of instances). You can
    /// drag it around to resize it.
    // Since: v0.4
    pub sidebar_width: Option<u32>,
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
    /// - `(1.0-*)` A higher number means more zoomed in buttons, text
    ///   and everything else (useful if you are on a high DPI display
    ///   or have bad eyesight),
    /// - `1.0` is the default value.
    /// - `(0.x-1.0)` A lower number means zoomed out UI elements.
    // Since: v0.4
    pub ui_scale: Option<f64>,

    /// Whether to enable antialiasing or not.
    /// Minor improvement in visual quality,
    /// also nudges launcher to use dedicated GPU
    /// for the interface.
    ///
    /// Default: `true`
    // Since: v0.4.2
    pub antialiasing: Option<bool>,
    /// Many launcher window related config options.
    // Since: v0.4.2
    pub window: Option<WindowProperties>,

    /// Settings that apply both on a per-instance basis and with global overrides.
    // Since: v0.4.2
    pub global_settings: Option<GlobalSettings>,
    pub extra_java_args: Option<Vec<String>>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        #[allow(deprecated)]
        Self {
            username: String::new(),
            theme: None,
            style: None,
            version: Some(LAUNCHER_VERSION_NAME.to_owned()),
            sidebar_width: Some(SIDEBAR_WIDTH_DEFAULT),
            accounts: None,
            ui_scale: None,
            java_installs: Some(Vec::new()),
            antialiasing: Some(true),
            account_selected: None,
            window: None,
            global_settings: None,
            extra_java_args: None,
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

        let config = std::fs::read_to_string(&config_path).path(&config_path)?;
        let mut config: Self = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(err) => {
                err!("Invalid launcher config! This may be a sign of corruption! Please report if this happens to you.\nError: {err}");
                return LauncherConfig::create(&config_path);
            }
        };
        if config.antialiasing.is_none() {
            config.antialiasing = Some(true);
        }

        #[allow(deprecated)]
        {
            if config.java_installs.is_none() {
                config.java_installs = Some(Vec::new());
            }
        }

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
        let config = LauncherConfig::default();
        std::fs::write(path, serde_json::to_string(&config).json_to()?.as_bytes()).path(path)?;
        Ok(config)
    }

    pub fn read_window_size(&mut self) -> (f32, f32) {
        let window = self.window.get_or_insert_with(Default::default);
        let scale = self.ui_scale.unwrap_or(1.0) as f32;
        let window_width = window
            .width
            .filter(|_| window.save_window_size)
            .unwrap_or(WINDOW_WIDTH * scale);
        let window_height = window
            .height
            .filter(|_| window.save_window_size)
            .unwrap_or(WINDOW_HEIGHT * scale);
        (window_width, window_height)
    }

    pub fn get_launch_prefix(&mut self) -> &mut Vec<String> {
        self.global_settings
            .get_or_insert_with(GlobalSettings::default)
            .pre_launch_prefix
            .get_or_insert_with(Vec::new)
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
