use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{InstanceSelection, IntoIoError, IntoJsonError, JsonFileError};

/// Configuration for using a custom Minecraft JAR file
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomJarConfig {
    pub name: String,
    pub autoset_main_class: bool,
}

/// Defines how instance Java arguments should interact with global Java arguments
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JavaArgsMode {
    /// Use global arguments only if instance arguments are empty,
    /// as a *fallback*.
    #[serde(rename = "fallback")]
    Fallback,
    /// Disable global arguments entirely,
    /// **only** use instance arguments
    #[serde(rename = "disable")]
    Disable,
    /// Combine global arguments with instance arguments,
    /// using both together.
    #[serde(rename = "combine")]
    #[default]
    Combine,
}

impl JavaArgsMode {
    pub const ALL: &[Self] = &[Self::Combine, Self::Disable, Self::Fallback];

    pub fn get_description(self) -> &'static str {
        match self {
            JavaArgsMode::Fallback => "Use global arguments only when instance has no arguments",
            JavaArgsMode::Disable => "No global arguments are applied",
            JavaArgsMode::Combine => {
                "Global arguments are combined with instance arguments (default)"
            }
        }
    }
}

impl std::fmt::Display for JavaArgsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaArgsMode::Fallback => write!(f, "Fallback"),
            JavaArgsMode::Disable => write!(f, "Disable"),
            JavaArgsMode::Combine => write!(f, "Combine (default)"),
        }
    }
}

/// Defines how instance pre-launch prefix commands should interact with global pre-launch prefix commands
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreLaunchPrefixMode {
    /// Use global prefix only if instance prefix is empty
    #[serde(rename = "fallback")]
    Fallback,
    /// Use instance prefix only, ignoring global prefix
    #[serde(rename = "disable")]
    Disable,
    /// Combine global prefix + instance prefix
    /// (in order)
    #[serde(rename = "combine_global_local")]
    #[default]
    CombineGlobalLocal,
    /// Combine instance prefix + global prefix
    /// (in order)
    #[serde(rename = "combine_local_global")]
    CombineLocalGlobal,
}

impl PreLaunchPrefixMode {
    pub const ALL: &[Self] = &[
        Self::CombineGlobalLocal,
        Self::CombineLocalGlobal,
        Self::Disable,
        Self::Fallback,
    ];

    pub fn get_description(self) -> &'static str {
        match self {
            PreLaunchPrefixMode::Fallback => "Use global prefix only when instance has no prefix",
            PreLaunchPrefixMode::Disable => "Use only instance prefix, ignore global prefix",
            PreLaunchPrefixMode::CombineGlobalLocal => {
                "Combine global + instance prefix (global first, then instance)"
            }
            PreLaunchPrefixMode::CombineLocalGlobal => {
                "Combine instance + global prefix (instance first, then global)"
            }
        }
    }
}

impl std::fmt::Display for PreLaunchPrefixMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreLaunchPrefixMode::Fallback => write!(f, "Fallback"),
            PreLaunchPrefixMode::Disable => write!(f, "Disable"),
            PreLaunchPrefixMode::CombineGlobalLocal => write!(f, "Combine Global+Local (default)"),
            PreLaunchPrefixMode::CombineLocalGlobal => write!(f, "Combine Local+Global"),
        }
    }
}

/// Configuration for a specific instance.
/// Not to be confused with [`crate::json::VersionDetails`]. That one
/// is launcher agnostic data provided from mojang, this one is
/// Quantum Launcher specific information.
///
/// Stored in:
/// - Client: `QuantumLauncher/instances/<instance_name>/config.json`
/// - Server: `QuantumLauncher/servers/<instance_name>/config.json`
///
/// See the documentation of each field for more information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstanceConfigJson {
    /// **Default: `"Vanilla"`**
    ///
    /// Can be one of:
    /// - `"Vanilla"` (unmodded)
    /// - `"Fabric"`
    /// - `"Forge"`
    /// - `"OptiFine"`
    /// - `"Quilt"`
    /// - `"NeoForge"`
    pub mod_type: String,
    /// If you want to use your own Java installation
    /// instead of the auto-installed one, specify
    /// the path to the `java` executable here.
    pub java_override: Option<String>,
    /// The amount of RAM in megabytes the instance should have.
    pub ram_in_mb: usize,
    /// **Default: `true`**
    ///
    /// - `true` (default): Show log output in launcher.
    ///   May not show all log output, especially during a crash.
    /// - `false`: Print raw, unformatted log output to the console (stdout).
    ///   This is useful for debugging, but may be hard to read.
    pub enable_logger: Option<bool>,
    /// This is an optional list of additional
    /// arguments to pass to Java.
    pub java_args: Option<Vec<String>>,
    /// This is an optional list of additional
    /// arguments to pass to the game.
    pub game_args: Option<Vec<String>>,

    /// DEPRECATED in v0.4.2
    ///
    /// This used to indicate whether a version
    /// was downloaded from Omniarchive instead
    /// of Mojang, in Quantum Launcher
    /// v0.3.1 - v0.4.1
    #[deprecated(since = "0.4.2", note = "migrated to BetterJSONs, so no longer needed")]
    pub omniarchive: Option<serde_json::Value>,
    /// **Default: `false`**
    ///
    /// - `true`: the instance is a classic server.
    /// - `false` (default): the instance is a client
    ///   or a non-classic server (alpha, beta, release).
    ///
    /// This is stored because classic servers:
    /// - Are downloaded differently (zip file to extract)
    /// - Cannot be stopped by sending a `stop` command.
    ///   (need to kill the process)
    pub is_classic_server: Option<bool>,
    /// **`false` for client instances, `true` for server installations**
    ///
    /// Added in v0.4.2
    pub is_server: Option<bool>,
    /// **Client Only**
    ///
    /// If true, then the Java Garbage Collector
    /// will be modified through launch arguments,
    /// for *different* performance.
    ///
    /// **Default: `false`**
    ///
    /// This doesn't specifically improve performance,
    /// in fact from my testing it worsens them?:
    ///
    /// - Without these args I got 110-115 FPS average on vanilla
    ///   Minecraft 1.20 in a new world.
    ///
    /// - With these args I got 105-110 FPS. So... yeah they aren't
    ///   doing the job for me.
    ///
    /// But in different workloads this might improve performance.
    ///
    /// # Arguments
    ///
    /// The G1 garbage collector will be used.
    /// Here are the specific arguments.
    ///
    /// - `-XX:+UnlockExperimentalVMOptions`
    /// - `-XX:+UseG1GC`
    /// - `-XX:G1NewSizePercent=20`
    /// - `-XX:G1ReservePercent=20`
    /// - `-XX:MaxGCPauseMillis=50`
    /// - `-XX:G1HeapRegionSize=32M`
    pub do_gc_tuning: Option<bool>,
    /// **Client Only**
    ///
    /// Whether to close the launcher upon
    /// starting the game.
    ///
    /// **Default: `false`**
    ///
    /// This keeps *just the game* running
    /// after you open it. However:
    /// - The impact of keeping the launcher open
    ///   is downright **negligible**. Quantum Launcher
    ///   is **very** lightweight. You won't feel any
    ///   difference even on slow computers
    /// - By doing this you lose access to easy log viewing
    ///   and the ability to easily kill the game process if stuck
    ///
    /// Ultimately if you want one less icon in your taskbar then go ahead.
    pub close_on_start: Option<bool>,

    pub global_settings: Option<GlobalSettings>,

    /// Controls how this instance's Java arguments interact with global Java arguments.
    /// See [`JavaArgsMode`] documentation for more info.
    ///
    /// **Default: `JavaArgsMode::Combine`**
    pub java_args_mode: Option<JavaArgsMode>,

    /// Controls how this instance's pre-launch prefix commands interact with global pre-launch prefix.
    /// See [`PreLaunchPrefixMode`] documentation for more info.
    ///
    /// **Default: `PreLaunchPrefixMode::CombineGlobalLocal`**
    pub pre_launch_prefix_mode: Option<PreLaunchPrefixMode>,
    /// **Client and Server**
    ///
    /// Custom jar configuration for using alternative client/server jars.
    /// When set, the launcher will use the specified custom jar instead of the default
    /// Minecraft jar, but will use assets from the instance's configured version.
    ///
    /// This is useful for:
    /// - Modified client jars (e.g., Cypress, Omniarchive special versions)
    /// - Custom modded jars not available through official channels
    /// - Client/server jars from external sources
    /// - Custom server implementations
    ///
    /// **Default: `None`** (use official Minecraft jar)
    pub custom_jar: Option<CustomJarConfig>,
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

    /// Gets Java arguments with global fallback/combination support.
    ///
    /// The behavior depends on the instance's `java_args_mode`.
    /// See [`JavaArgsMode`] documentation for more info.
    ///
    /// Returns an empty vector if no arguments should be used.
    #[must_use]
    pub fn get_java_args(&self, global_args: &[String]) -> Vec<String> {
        let mode = self
            .java_args_mode
            .as_ref()
            .unwrap_or(&JavaArgsMode::Combine);
        let instance_args = self.java_args.as_ref();

        let has_meaningful_instance_args =
            instance_args.is_some_and(|args| args.iter().any(|arg| !arg.trim().is_empty()));

        match mode {
            // Use instance if meaningful, otherwise global
            JavaArgsMode::Fallback => {
                if has_meaningful_instance_args {
                    instance_args.unwrap().clone()
                } else {
                    global_args.to_owned()
                }
            }
            // Use instance args only, ignore global completely
            JavaArgsMode::Disable => {
                if has_meaningful_instance_args {
                    instance_args.unwrap().clone()
                } else {
                    Vec::new()
                }
            }
            // Combine both instance and global args
            JavaArgsMode::Combine => {
                let mut combined = Vec::new();
                combined.extend(global_args.iter().filter(|n| !n.trim().is_empty()).cloned());
                if has_meaningful_instance_args {
                    combined.extend(instance_args.unwrap().iter().cloned());
                }

                combined
            }
        }
    }

    pub fn get_launch_prefix(&mut self) -> &mut Vec<String> {
        self.global_settings
            .get_or_insert_with(GlobalSettings::default)
            .pre_launch_prefix
            .get_or_insert_with(Vec::new)
    }

    /// Gets pre-launch prefix commands with global fallback/combination support.
    ///
    /// The behavior depends on the instance's `pre_launch_prefix_mode`:
    /// - `Fallback`: Returns instance prefix if meaningful, otherwise global prefix
    /// - `Override`: Returns instance prefix only (ignores global even if instance is empty)
    /// - `CombineGlobalLocal`: Returns global prefix + instance prefix (global first, then instance)
    /// - `CombineLocalGlobal`: Returns instance prefix + global prefix (instance first, then global)
    ///
    /// Returns an empty vector if no prefixes should be used.
    #[must_use]
    pub fn setup_launch_prefix(&mut self, global_prefix: &[String]) -> Vec<String> {
        let mode = self.pre_launch_prefix_mode.unwrap_or_default();

        let mut instance_prefix: Vec<String> = self
            .get_launch_prefix()
            .iter_mut()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .collect();

        let mut global_prefix: Vec<String> = global_prefix
            .iter()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .collect();

        match mode {
            PreLaunchPrefixMode::Fallback => {
                if instance_prefix.is_empty() {
                    global_prefix.to_owned()
                } else {
                    instance_prefix
                }
            }
            PreLaunchPrefixMode::Disable => {
                if instance_prefix.is_empty() {
                    Vec::new()
                } else {
                    instance_prefix
                }
            }
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
}

/// Settings that can both be set on a per-instance basis
/// and also have a global default.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GlobalSettings {
    /// **Client Only**
    ///
    /// Custom window **width** for Minecraft (in windowed mode).
    ///
    /// When set, this will launch Minecraft with a specific window width
    /// using the `--width` command line argument.
    pub window_width: Option<u32>,
    /// **Client Only**
    ///
    /// Custom window **height** for Minecraft (in windowed mode).
    ///
    /// When set, this will launch Minecraft with a specific window height
    /// using the `--height` command line argument.
    pub window_height: Option<u32>,
    /// This is an optional list of commands to prepend
    /// to the launch command (e.g., "prime-run" for NVIDIA GPU usage on Linux).
    pub pre_launch_prefix: Option<Vec<String>>,
}
