use serde::{Deserialize, Serialize};

/// Defines how instance Java arguments should interact with global Java arguments
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JavaArgsMode {
    /// Use global args only if instance args are empty, as a *fallback*.
    #[serde(rename = "fallback")]
    Fallback,
    /// **Only** use instance arguments
    #[serde(rename = "disable")]
    Disable,
    /// Combine both
    #[serde(rename = "combine")]
    #[default]
    Combine,
}

impl JavaArgsMode {
    pub const ALL: &'static [Self] = &[Self::Combine, Self::Disable, Self::Fallback];

    #[must_use]
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
    /// Only use instance prefix
    #[serde(rename = "disable")]
    Disable,
    /// Combine global prefix + instance prefix (in order)
    #[serde(rename = "combine_global_local")]
    #[default]
    CombineGlobalLocal,
    /// Combine instance prefix + global prefix (in order)
    #[serde(rename = "combine_local_global")]
    CombineLocalGlobal,
}

impl PreLaunchPrefixMode {
    pub const ALL: &'static [Self] = &[
        Self::CombineGlobalLocal,
        Self::CombineLocalGlobal,
        Self::Disable,
        Self::Fallback,
    ];

    #[must_use]
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
