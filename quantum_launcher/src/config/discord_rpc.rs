use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const BASIC_DETAILS: &str = "Opened Launcher";
const BASIC_STATE: &str = "";
const GAMEOPEN_DETAILS: &str = "Minecraft v${version}";
const GAMEOPEN_STATE: &str = "Instance name: ${instance}";
const GAMEEXIT_DETAILS: &str = "Just quit game";
const GAMEEXIT_STATE: &str = "Minecraft v${version}";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcConfig {
    /// Enable Discord Rich Presence support
    // Since: TBD
    pub enable: bool,
    /// Details for the basic/initial rich presence activity
    // Since: TBD
    pub basic: RpcText,
    /// Whether to change rich presence with instance open/exit events.
    // Since: TBD
    pub update_on_game_open: bool,
    /// Activity on opening the game
    // Since: TBD
    pub on_gameopen: RpcText,
    /// Activity on closing the game
    // Since: TBD
    pub on_gameexit: RpcText,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcText {
    pub top_text: Option<String>,
    pub bottom_text: String,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            enable: false,
            basic: RpcText {
                top_text: Some(BASIC_DETAILS.to_owned()),
                bottom_text: BASIC_STATE.to_owned(),
                _extra: HashMap::new(),
            },
            update_on_game_open: true,
            on_gameopen: RpcText {
                top_text: Some(GAMEOPEN_DETAILS.to_owned()),
                bottom_text: GAMEOPEN_STATE.to_owned(),
                _extra: HashMap::new(),
            },
            on_gameexit: RpcText {
                top_text: Some(GAMEEXIT_DETAILS.to_owned()),
                bottom_text: GAMEEXIT_STATE.to_owned(),
                _extra: HashMap::new(),
            },
            _extra: HashMap::new(),
        }
    }
}
