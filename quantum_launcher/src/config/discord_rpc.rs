use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const BASIC_DETAILS: &str = "Opened Launcher";
const GAMEOPEN_DETAILS: &str = "Minecraft v${version}";
const GAMEOPEN_STATE: &str = "Instance name: ${instance}";
const GAMEEXIT_DETAILS: &str = "Just quit game";
const GAMEEXIT_STATE: &str = "Minecraft v${version}";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcConfig {
    /// Enable Discord Rich Presence support
    // Since: TBD
    pub enable: bool,
    /// Custom rich presence activity name
    // Since: TBD
    pub name: Option<String>,
    /// Details for the basic/initial rich presence activity
    // Since: TBD
    pub basic: RpcText,
    /// The default status display type to use.
    // Since: TBD
    #[serde(default)]
    pub status_display_type: PresenceStatusDisplayType,
    /// Whether to change rich presence with instance open/exit events.
    // Since: TBD
    pub update_on_game_open: bool,
    /// Activity on opening the game
    // Since: TBD
    pub on_gameopen: RpcText,
    /// Activity on closing the game
    // Since: TBD
    pub on_gameexit: RpcText,
    /// Whether to display "Competing on ..." in the rich presence activity
    // Since: TBD
    #[serde(default = "competing_default")]
    pub competing: bool,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl RpcConfig {
    pub fn fix(&mut self) {
        self.basic.fix();
        self.on_gameopen.fix();
        self.on_gameexit.fix();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcText {
    pub top_text: Option<String>,
    pub top_text_url: Option<String>,
    pub bottom_text: Option<String>,
    pub bottom_text_url: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl RpcText {
    fn fix(&mut self) {
        if self.top_text.as_ref().is_some_and(|n| n.is_empty()) {
            self.top_text = None;
        }
        if self.bottom_text.as_ref().is_some_and(|n| n.is_empty()) {
            self.bottom_text = None;
        }

        if self
            .top_text_url
            .as_ref()
            .is_some_and(|n| n.trim().is_empty())
        {
            self.top_text_url = None;
        }
        if self
            .bottom_text_url
            .as_ref()
            .is_some_and(|n| n.trim().is_empty())
        {
            self.bottom_text_url = None;
        }
    }
}

const fn competing_default() -> bool {
    false
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            enable: false,
            name: None,
            basic: RpcText {
                top_text: Some(BASIC_DETAILS.to_owned()),
                top_text_url: None,
                bottom_text: None,
                bottom_text_url: None,
                _extra: HashMap::new(),
            },
            update_on_game_open: true,
            on_gameopen: RpcText {
                top_text: Some(GAMEOPEN_DETAILS.to_owned()),
                top_text_url: None,
                bottom_text: Some(GAMEOPEN_STATE.to_owned()),
                bottom_text_url: None,
                _extra: HashMap::new(),
            },
            on_gameexit: RpcText {
                top_text: Some(GAMEEXIT_DETAILS.to_owned()),
                top_text_url: None,
                bottom_text: Some(GAMEEXIT_STATE.to_owned()),
                bottom_text_url: None,
                _extra: HashMap::new(),
            },
            _extra: HashMap::new(),
            competing: competing_default(),
            status_display_type: PresenceStatusDisplayType::Name,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum PresenceStatusDisplayType {
    Details,
    State,
    #[default]
    #[serde(other)]
    Name,
}

impl PresenceStatusDisplayType {
    pub const ALL: &'static [Self] = &[
        PresenceStatusDisplayType::Name,
        PresenceStatusDisplayType::Details,
        PresenceStatusDisplayType::State,
    ];
}

impl std::fmt::Display for PresenceStatusDisplayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PresenceStatusDisplayType::Name => "App Name",
            PresenceStatusDisplayType::Details => "Top Text",
            PresenceStatusDisplayType::State => "Bottom Text",
        })
    }
}
