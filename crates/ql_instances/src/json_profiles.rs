use std::collections::BTreeMap;

use serde::Serialize;

/// Represents the `launcher_profiles.json` file.
///
/// It's not needed for the game to run, but some
/// loader installers depend on it so it's included.
#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct ProfileJson {
    profiles: BTreeMap<String, Profiles>,
    clientToken: Option<String>,
    // Map<UUID, AuthenticationDatabase>
    authenticationDatabase: Option<BTreeMap<String, AuthenticationDatabase>>,
    launcherVersion: Option<LauncherVersion>,
    settings: Settings,
    analyticsToken: Option<String>,
    analyticsFailcount: Option<i32>,
    selectedUser: Option<SelectedUser>,
    version: Option<i32>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct Profiles {
    name: String,
    r#type: Option<String>,
    created: Option<String>,
    lastUsed: Option<String>,
    icon: Option<String>,
    lastVersionId: String,
    gameDir: Option<String>,
    javaDir: Option<String>,
    javaArgs: Option<String>,
    logConfig: Option<String>,
    logConfigIsXML: Option<bool>,
    resolution: Option<Resolution>,
}

#[derive(Serialize)]
pub struct Resolution {
    height: i32,
    width: i32,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct AuthenticationDatabase {
    accessToken: String,
    username: String,
    // Map<UUID, Name>
    profiles: BTreeMap<String, String>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct LauncherVersion {
    name: String,
    format: i32,
    profilesFormat: i32,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    enableSnapshots: bool,
    enableAdvanced: bool,
    keepLauncherOpen: bool,
    showGameLog: bool,
    locale: Option<String>,
    showMenu: bool,
    enableHistorical: bool,
    profileSorting: String,
    crashAssistance: bool,
    enableAnalytics: bool,
    soundOn: Option<bool>,
}

#[derive(Serialize)]
pub struct SelectedUser {
    account: String,
    profile: String,
}

impl Default for ProfileJson {
    fn default() -> Self {
        Self {
            profiles: [].into(),
            clientToken: None,
            authenticationDatabase: None,
            launcherVersion: None,
            settings: Settings {
                enableSnapshots: true,
                enableAdvanced: true,
                keepLauncherOpen: true,
                showGameLog: true,
                locale: None,
                showMenu: true,
                enableHistorical: true,
                profileSorting: "ByLastPlayed".to_owned(),
                crashAssistance: false,
                enableAnalytics: false,
                soundOn: Some(false),
            },
            analyticsToken: None,
            analyticsFailcount: None,
            selectedUser: None,
            version: None,
        }
    }
}
