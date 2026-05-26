use std::{fmt::Display, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    LAUNCHER_DIR, err,
    json::{manifest::Version, version::JavaVersionJson},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Instance {
    pub name: Arc<str>,
    pub kind: InstanceKind,
}

impl Instance {
    #[must_use]
    pub fn new(name: &str, kind: InstanceKind) -> Self {
        Self {
            name: Arc::from(name),
            kind,
        }
    }

    #[must_use]
    pub fn client(name: &str) -> Self {
        Self::new(name, InstanceKind::Client)
    }

    #[must_use]
    pub fn server(name: &str) -> Self {
        Self::new(name, InstanceKind::Server)
    }

    /// Gets the path where launcher-specific things are stored.
    ///
    /// - Instances: `QuantumLauncher/instances/<NAME>/`
    /// - Servers: `QuantumLauncher/servers/<NAME>/`
    #[must_use]
    pub fn get_instance_path(&self) -> PathBuf {
        let name = &*self.name;
        self.kind.get_root_directory().join(name)
    }

    /// Gets the path where files used by the game itself are stored.
    ///
    /// - Clients: `QuantumLauncher/instances/<NAME>/.minecraft/`
    /// - Servers: `QuantumLauncher/servers/<NAME>/data/`
    ///
    /// It can vary in the future,
    /// the only requirement is that it must be equal to, or a subdirectory of,
    /// the instance path ([`Instance::get_instance_path`]).
    #[must_use]
    pub fn get_dot_minecraft_path(&self) -> PathBuf {
        self.kind
            .get_root_directory()
            .join(&*self.name)
            .join(match self.kind {
                InstanceKind::Client => ".minecraft",
                InstanceKind::Server => "data",
            })
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn is_server(&self) -> bool {
        self.kind.is_server()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InstanceKind {
    Server,
    #[serde(other)]
    Client,
}

impl InstanceKind {
    #[must_use]
    pub const fn is_server(self) -> bool {
        matches!(self, Self::Server)
    }

    pub fn get_root_directory(&self) -> PathBuf {
        let name = match self {
            InstanceKind::Client => "instances",
            InstanceKind::Server => "servers",
        };
        LAUNCHER_DIR.join(name)
    }
}

/// A struct representing information about a Minecraft version
#[derive(Debug, Clone, PartialEq)]
pub struct ListEntry {
    pub name: String,
    pub supports_server: bool,
    /// For UI display purposes only
    pub kind: ListEntryKind,
}

impl ListEntry {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            kind: ListEntryKind::guess(&name),
            supports_server: Version::guess_if_supports_server(&name),
            name,
        }
    }

    #[must_use]
    pub fn with_kind(name: String, ty: &str) -> Self {
        Self {
            kind: ListEntryKind::calculate(&name, ty),
            supports_server: Version::guess_if_supports_server(&name),
            name,
        }
    }
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ListEntryKind {
    Release,
    Snapshot,
    Preclassic,
    Classic,
    Indev,
    Infdev,
    Alpha,
    Beta,
    AprilFools,
    Special,
}

impl Display for ListEntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListEntryKind::Release => write!(f, "Release"),
            ListEntryKind::Snapshot => write!(f, "Snapshot"),
            ListEntryKind::Preclassic => write!(f, "Pre-classic"),
            ListEntryKind::Classic => write!(f, "Classic"),
            ListEntryKind::Indev => write!(f, "Indev"),
            ListEntryKind::Infdev => write!(f, "Infdev"),
            ListEntryKind::Alpha => write!(f, "Alpha"),
            ListEntryKind::Beta => write!(f, "Beta"),
            ListEntryKind::AprilFools => write!(f, "April Fools"),
            ListEntryKind::Special => write!(f, "Special"),
        }
    }
}

impl ListEntryKind {
    pub const ALL: &'static [ListEntryKind] = &[
        ListEntryKind::Release,
        ListEntryKind::Snapshot,
        ListEntryKind::Beta,
        ListEntryKind::Alpha,
        ListEntryKind::Infdev,
        ListEntryKind::Indev,
        ListEntryKind::Classic,
        ListEntryKind::Preclassic,
        ListEntryKind::AprilFools,
        ListEntryKind::Special,
    ];

    /// Returns the default selected categories
    #[must_use]
    pub fn default_selected() -> std::collections::HashSet<ListEntryKind> {
        let mut set = std::collections::HashSet::new();
        set.extend(Self::ALL);
        set.remove(&Self::Snapshot);
        set.remove(&Self::Special);
        set
    }
}

impl ListEntryKind {
    fn guess(id: &str) -> Self {
        if id.starts_with("b1.") {
            ListEntryKind::Beta
        } else if id.starts_with("a1.") {
            ListEntryKind::Alpha
        } else if id.starts_with("inf-") {
            ListEntryKind::Infdev
        } else if id.starts_with("in-") {
            ListEntryKind::Indev
        } else if id.starts_with("pc-") {
            ListEntryKind::Preclassic
        } else if id.starts_with("c0.") {
            ListEntryKind::Classic
        } else if id.contains('w') {
            ListEntryKind::Snapshot
        } else {
            ListEntryKind::Release
        }
    }

    #[must_use]
    pub fn calculate(id: &str, ty: &str) -> Self {
        if ty == "special" {
            ListEntryKind::Special
        } else if ty == "april-fools" {
            ListEntryKind::AprilFools
        } else if id.starts_with("b1.") {
            ListEntryKind::Beta
        } else if id.starts_with("a1.") {
            ListEntryKind::Alpha
        } else if id.starts_with("inf-") {
            ListEntryKind::Infdev
        } else if id.starts_with("in-") {
            ListEntryKind::Indev
        } else if id.starts_with("pc-") {
            ListEntryKind::Preclassic
        } else if id.starts_with("c0.") {
            ListEntryKind::Classic
        } else if ty == "snapshot" {
            ListEntryKind::Snapshot
        } else {
            ListEntryKind::Release
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loader {
    #[serde(rename = "Fabric")]
    Fabric,
    #[serde(rename = "Quilt")]
    Quilt,
    #[serde(rename = "Forge")]
    Forge,
    NeoForge,

    // The launcher supports these, but modrinth doesn't
    // (so no Mod Store):
    #[serde(rename = "OptiFine")]
    OptiFine,
    #[serde(rename = "Paper")]
    Paper,

    // The launcher doesn't currently support these:
    #[serde(rename = "LiteLoader")]
    Liteloader,
    #[serde(rename = "ModLoader")]
    Modloader,
    #[serde(rename = "Rift")]
    Rift,

    #[serde(rename = "Vanilla")]
    #[default]
    #[serde(other)]
    Vanilla,
}

impl Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = serde_json::to_string(self)
            .ok()
            .and_then(|n| n.strip_prefix("\"").map(str::to_owned))
            .and_then(|n| n.strip_suffix("\"").map(str::to_owned))
        {
            write!(f, "{s}")
        } else {
            write!(f, "{self:?}")
        }
    }
}

impl Loader {
    pub const ALL: &[Self] = &[
        Self::Vanilla,
        Self::Fabric,
        Self::Quilt,
        Self::Forge,
        Self::NeoForge,
        Self::OptiFine,
        Self::Paper,
        Self::Liteloader,
        Self::Modloader,
        Self::Rift,
    ];

    #[must_use]
    pub fn not_vanilla(self) -> Option<Self> {
        (!self.is_vanilla()).then_some(self)
    }

    #[must_use]
    pub fn is_vanilla(self) -> bool {
        matches!(self, Loader::Vanilla)
    }

    #[must_use]
    pub fn to_modrinth_str(self) -> &'static str {
        match self {
            Loader::Forge => "forge",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Liteloader => "liteloader",
            Loader::Modloader => "modloader",
            Loader::Rift => "rift",
            Loader::NeoForge => "neoforge",
            Loader::OptiFine => "optifine",
            Loader::Paper => "paper",
            Loader::Vanilla => " ",
        }
    }

    #[must_use]
    pub fn to_curseforge_num(&self) -> &'static str {
        match self {
            Loader::Forge => "1",
            Loader::Fabric => "4",
            Loader::Quilt => "5",
            Loader::NeoForge => "6",
            Loader::Liteloader => "3",
            Loader::Rift
            | Loader::Paper
            | Loader::Modloader
            | Loader::OptiFine
            | Loader::Vanilla => {
                err!("Unsupported loader for curseforge: {self:?}");
                "0"
            } // Not supported
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JavaVersion {
    Java8 = 8,
    Java16 = 16,
    Java17 = 17,
    Java21 = 21,
    Java25 = 25,
}

impl JavaVersion {
    pub const ALL: &[Self] = &[
        Self::Java8,
        Self::Java16,
        Self::Java17,
        Self::Java21,
        Self::Java25,
    ];

    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Java8 => Some(Self::Java16),
            Self::Java16 => Some(Self::Java17),
            Self::Java17 => Some(Self::Java21),
            Self::Java21 => Some(Self::Java25),
            Self::Java25 => None,
        }
    }
}

impl Display for JavaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Java8 => "java_8",
            Self::Java16 => "java_16",
            Self::Java17 => "java_17",
            Self::Java21 => "java_21",
            Self::Java25 => "java_25",
        })
    }
}

impl From<JavaVersionJson> for JavaVersion {
    fn from(version: JavaVersionJson) -> Self {
        match version.majorVersion {
            8 => Self::Java8,
            16 => Self::Java16,
            17 => Self::Java17,
            21 => Self::Java21,
            _ => Self::Java25,
        }
    }
}

impl From<usize> for JavaVersion {
    fn from(value: usize) -> Self {
        match value {
            8 => Self::Java8,
            16 => Self::Java16,
            17 => Self::Java17,
            21 => Self::Java21,
            _ => Self::Java25,
        }
    }
}
