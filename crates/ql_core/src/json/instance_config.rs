use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{InstanceSelection, IntoIoError, IntoJsonError, JsonFileError, Loader};

/// Configuration for a specific instance.
/// Not to be confused with [`crate::json::VersionDetails`]. That one
/// is launcher agnostic data provided from mojang, this one is
/// Quantum Launcher specific information.
///
/// Stored in:
/// - Client: `QuantumLauncher/instances/<NAME>/config.json`
/// - Server: `QuantumLauncher/servers/<NAME>/config.json`
///
/// See the documentation of each field for more information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstanceConfigJson {
    /// Memory allocation in MB
    // Since: v0.1
    pub ram_in_mb: usize,
    /// **Default: `Vanilla` (meaning, no loader)**
    // Since: v0.1
    pub mod_type: Loader,
    /// More metadata about the mod type
    // Since: v0.5
    pub mod_type_info: Option<ModTypeInfo>,

    /// Use a different **launcher-provided** java version.
    /// Prioritized over [`Self::java_override`]
    // Since: v0.5.1
    pub java_override_version: Option<usize>,
    /// Use a different **user-provided** `java` binary (path)
    // Since: v0.1
    pub java_override: Option<String>,

    /// Show logs in launcher (`true`, default) or raw console output AKA `stdout`/`stderr` (`false`).
    // Since: v0.3
    pub enable_logger: Option<bool>,
    /// Extra Java arguments
    // Since: v0.3
    pub java_args: Option<Vec<String>>,
    /// Extra game arguments
    // Since: v0.3
    pub game_args: Option<Vec<String>>,

    /// Previously used to indicate if a version was downloaded from Omniarchive
    // Since: v0.3.1 - v0.4.1
    #[deprecated(since = "0.4.2", note = "migrated to BetterJSONs, so no longer needed")]
    pub omniarchive: Option<serde_json::Value>,

    /// Classic server mode (default: `false`).
    /// - `false`: client or non-classic server
    /// - `true`: classic server (ZIP download, no `stop` command)
    // Since: v0.3.1
    pub is_classic_server: Option<bool>,
    /// Whether this is a server, not a client
    pub is_server: Option<bool>,

    /// Close launcher after client starts (default: `false`).
    /// Enable to reduce taskbar icons; leaving it open has negligible impact.
    // Since: v0.4
    pub close_on_start: Option<bool>,
    // Since: v0.4.2
    pub global_settings: Option<GlobalSettings>,

    /// Whether global launcher-wide Java arguments will be used
    /// (default: `true`)
    pub global_java_args_enable: Option<bool>,

    /// Controls how this instance's pre-launch prefix commands interact with global pre-launch prefix.
    /// See [`PreLaunchPrefixMode`] documentation for more info.
    ///
    /// **Default: `CombineGlobalLocal`**
    pub pre_launch_prefix_mode: Option<PreLaunchPrefixMode>,
    /// **Client and Server**
    /// Custom jar configuration for using alternative client/server jars.
    /// Replaces default Minecraft jar while using assets from the configured version.
    ///
    /// Useful for:
    /// - Modified client jars (e.g., Cypress, Omniarchive)
    /// - Custom modded or external jars
    /// - Custom server implementations
    ///
    /// **Default: `None`** (uses official Minecraft jar)
    pub custom_jar: Option<CustomJarConfig>,
    /// Information related to the currently-installed
    /// version of the game
    pub version_info: Option<VersionInfo>,
    /// An override for the main class when launching the game.
    /// Mainly only used for debugging purposes.
    pub main_class_override: Option<String>,
}

impl InstanceConfigJson {
    /// Returns a String containing the Java argument to
    /// allocate the configured amount of RAM.
    #[must_use]
    pub fn get_ram_argument(&self) -> String {
        format!("-Xmx{}M", self.ram_in_mb)
    }

    /// Loads the launcher-specific instance configuration from disk,
    /// based on a path to the root of the instance directory.
    ///
    /// # Errors
    /// - `dir`/`config.json` doesn't exist or isn't a file
    /// - `config.json` file couldn't be loaded
    /// - `config.json` couldn't be parsed into valid JSON
    pub async fn read_from_dir(dir: &Path) -> Result<Self, JsonFileError> {
        let config_json_path = dir.join("config.json");
        let config_json = tokio::fs::read_to_string(&config_json_path)
            .await
            .path(config_json_path)?;
        Ok(serde_json::from_str(&config_json).json(config_json)?)
    }

    /// Loads the launcher-specific instance configuration from disk,
    /// based on a specific `InstanceSelection`
    ///
    /// # Errors
    /// - `config.json` file couldn't be loaded
    /// - `config.json` couldn't be parsed into valid JSON
    pub async fn read(instance: &InstanceSelection) -> Result<Self, JsonFileError> {
        Self::read_from_dir(&instance.get_instance_path()).await
    }

    /// Saves the launcher-specific instance configuration to disk,
    /// based on a path to the root of the instance directory.
    ///
    /// # Errors
    /// - `config.json` file couldn't be written to
    pub async fn save_to_dir(&self, dir: &Path) -> Result<(), JsonFileError> {
        let config_json_path = dir.join("config.json");
        let config_json = serde_json::to_string_pretty(self).json_to()?;
        tokio::fs::write(&config_json_path, config_json)
            .await
            .path(config_json_path)?;
        Ok(())
    }

    /// Saves the launcher-specific instance configuration to disk,
    /// based on a specific `InstanceSelection`
    ///
    /// # Errors
    /// - `config.json` file couldn't be written to
    /// - `self` couldn't be serialized into valid JSON
    pub async fn save(&self, instance: &InstanceSelection) -> Result<(), JsonFileError> {
        self.save_to_dir(&instance.get_instance_path()).await
    }

    #[must_use]
    pub fn get_window_size(&self, global: Option<&GlobalSettings>) -> (Option<u32>, Option<u32>) {
        let local = self.global_settings.as_ref();
        (
            local
                .and_then(|n| n.window_width)
                .or(global.and_then(|n| n.window_width)),
            local
                .and_then(|n| n.window_height)
                .or(global.and_then(|n| n.window_height)),
        )
    }

    /// Gets Java arguments (combining them with global args based on configuration)
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Won't panic
    pub fn get_java_args(&self, global_args: &[String]) -> Vec<String> {
        let use_global_args = self.global_java_args_enable.unwrap_or(true);
        let mut instance_args = self.java_args.clone().unwrap_or_default();

        if use_global_args {
            instance_args.extend(global_args.iter().filter(|n| !n.trim().is_empty()).cloned());
        }
        instance_args.retain(|n| !n.trim().is_empty());
        instance_args
    }

    /// Gets pre-launch prefix commands, (empty if none).
    ///
    /// Whether to combine with global prefixes, and how,
    /// depends on the instance's [`PreLaunchPrefixMode`].
    #[must_use]
    pub fn build_launch_prefix(&mut self, global_prefix: &[String]) -> Vec<String> {
        let mode = self.pre_launch_prefix_mode.unwrap_or_default();

        let mut instance_prefix: Vec<String> = self
            .c_global_settings()
            .pre_launch_prefix
            .iter_mut()
            .flatten()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .collect();

        let mut global_prefix: Vec<String> = global_prefix
            .iter()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .collect();

        match mode {
            PreLaunchPrefixMode::Disable => instance_prefix,
            PreLaunchPrefixMode::CombineGlobalLocal => {
                global_prefix.extend(instance_prefix);
                global_prefix
            }
            PreLaunchPrefixMode::CombineLocalGlobal => {
                instance_prefix.extend(global_prefix);
                instance_prefix
            }
        }
    }

    pub fn c_global_settings(&mut self) -> &mut GlobalSettings {
        self.global_settings
            .get_or_insert_with(GlobalSettings::default)
    }

    #[must_use]
    pub fn get_main_class_mode(&self) -> Option<MainClassMode> {
        self.custom_jar
            .as_ref()
            .is_some_and(|t| t.autoset_main_class)
            .then_some(MainClassMode::SafeFallback)
            .or(self
                .main_class_override
                .as_ref()
                .is_some_and(|n| !n.is_empty())
                .then_some(MainClassMode::Custom))
    }

    #[must_use]
    pub fn get_java_override(&self) -> Option<PathBuf> {
        fn inner(path: &str) -> Option<PathBuf> {
            if path.is_empty() {
                return None;
            }
            if path.starts_with("~/") || path.starts_with("~\\") {
                if let Some(home_dir) = dirs::home_dir() {
                    let without_tilde = &path[2..];
                    let full_path = home_dir.join(without_tilde);
                    return Some(full_path);
                }
            }
            Some(PathBuf::from(path))
        }

        if self.java_override_version.is_some() {
            // Use that instead
            return None;
        }

        let java_override = self.java_override.as_ref()?.trim();
        let path = inner(java_override)?;

        if !path.exists() {
            return None;
        }

        Some(path)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ModTypeInfo {
    pub version: Option<String>,
    /// If an unofficial implementation of the loader
    /// was used, which one (eg: Legacy Fabric).
    pub backend_implementation: Option<String>,
    pub optifine_jar: Option<String>,
}

/// Settings that can both be set on a per-instance basis
/// and also have a global default.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GlobalSettings {
    /// Custom window **width** for Minecraft in windowed mode
    /// (**Client Only**)
    // Since: v0.4.2
    pub window_width: Option<u32>,
    /// Custom window **height** for Minecraft in windowed mode
    /// (**Client Only**)
    // Since: v0.4.2
    pub window_height: Option<u32>,
    /// This is an optional list of commands to prepend
    /// to the launch command (e.g., "prime-run" for NVIDIA GPU usage on Linux).
    // Since: v0.5.0
    pub pre_launch_prefix: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub is_special_lwjgl3: bool,
}

/// Defines how instance pre-launch prefix commands should interact with global pre-launch prefix commands
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreLaunchPrefixMode {
    /// Only use instance prefix
    #[serde(rename = "disable")]
    Disable,
    /// Combine instance prefix + global prefix (in order)
    #[serde(rename = "combine_local_global")]
    CombineLocalGlobal,
    /// Combine global prefix + instance prefix (in order)
    #[serde(rename = "combine_global_local")]
    #[default]
    #[serde(other)]
    CombineGlobalLocal,
}

impl PreLaunchPrefixMode {
    pub const ALL: &'static [Self] = &[
        Self::CombineGlobalLocal,
        Self::CombineLocalGlobal,
        Self::Disable,
    ];

    #[must_use]
    pub const fn get_description(self) -> &'static str {
        match self {
            PreLaunchPrefixMode::Disable => "Only use instance prefix",
            PreLaunchPrefixMode::CombineGlobalLocal => "Global + instance",
            PreLaunchPrefixMode::CombineLocalGlobal => "Instance + global",
        }
    }

    #[must_use]
    pub const fn is_disabled(self) -> bool {
        matches!(self, PreLaunchPrefixMode::Disable)
    }
}

impl std::fmt::Display for PreLaunchPrefixMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreLaunchPrefixMode::Disable => write!(f, "Disable"),
            PreLaunchPrefixMode::CombineGlobalLocal => write!(f, "Combine Global+Local (default)"),
            PreLaunchPrefixMode::CombineLocalGlobal => write!(f, "Combine Local+Global"),
        }
    }
}

/// Configuration for using a custom Minecraft JAR file
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomJarConfig {
    pub name: String,
    pub autoset_main_class: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainClassMode {
    SafeFallback,
    Custom,
}
