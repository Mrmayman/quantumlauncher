use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::Path,
    sync::mpsc::{self, Receiver},
};

use iced::Task;
use notify::Watcher;
use ql_core::{
    err, file_utils, read_log::LogLine, GenericProgress, InstanceSelection, IntoIoError,
    IntoStringError, IoError, JsonFileError, LaunchedProcess, ListEntry, ModId, Progress,
    LAUNCHER_DIR, LAUNCHER_VERSION_NAME,
};
use ql_instances::auth::{ms::CLIENT_ID, AccountData, AccountType};
use tokio::process::ChildStdin;

use crate::{
    config::{LauncherConfig, SIDEBAR_WIDTH},
    stylesheet::styles::LauncherTheme,
};

mod images;
mod menu;
mod message;
pub use images::ImageState;
pub use menu::*;
pub use message::*;

pub const OFFLINE_ACCOUNT_NAME: &str = "(Offline)";
pub const NEW_ACCOUNT_NAME: &str = "+ Add Account";

pub const ADD_JAR_NAME: &str = "+ Add JAR";
pub const REMOVE_JAR_NAME: &str = "- Remove Selected";
pub const OPEN_FOLDER_JAR_NAME: &str = "> Open Folder";
pub const NONE_JAR_NAME: &str = "(None)";

type Res<T = ()> = Result<T, String>;

#[cfg(feature = "discord_rpc")]
pub const DISCORD_CLIENT_ID: &str = "1422991467944611871";

pub struct InstanceLog {
    pub log: Vec<String>,
    pub has_crashed: bool,
    pub command: String,
}

pub struct Launcher {
    pub state: State,
    pub selected_instance: Option<InstanceSelection>,
    pub config: LauncherConfig,
    pub theme: LauncherTheme,
    pub images: ImageState,

    pub is_log_open: bool,
    pub log_scroll: isize,
    pub tick_timer: usize,
    pub is_launching_game: bool,

    pub java_recv: Option<ProgressBar<GenericProgress>>,
    pub custom_jar: Option<CustomJarState>,
    pub mod_updates_checked: HashMap<InstanceSelection, Vec<(ModId, String, bool)>>,
    pub autosave: HashSet<AutoSaveKind>,
    #[cfg(feature = "discord_rpc")]
    pub discord: Option<Discord>,

    pub accounts: HashMap<String, AccountData>,
    pub accounts_dropdown: Vec<String>,
    pub accounts_selected: Option<String>,

    pub version_list_cache: VersionListCache,
    pub client_list: Option<Vec<String>>,
    pub server_list: Option<Vec<String>>,

    pub processes: HashMap<InstanceSelection, GameProcess>,
    pub logs: HashMap<InstanceSelection, InstanceLog>,

    pub window_state: WindowState,
    pub keys_pressed: HashSet<iced::keyboard::Key>,
    pub modifiers_pressed: iced::keyboard::Modifiers,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum AutoSaveKind {
    LauncherConfig,
    Jarmods,
}

pub struct WindowState {
    pub size: (f32, f32),
    pub mouse_pos: (f32, f32),
    pub is_maximized: bool,
}

pub struct CustomJarState {
    pub choices: Vec<String>,
    pub recv: Receiver<notify::Event>,
    pub _watcher: notify::RecommendedWatcher,
}

impl CustomJarState {
    pub fn load() -> Task<Message> {
        Task::perform(load_custom_jars(), |n| {
            Message::EditInstance(EditInstanceMessage::CustomJarLoaded(n.strerr()))
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct VersionListCache {
    pub list: Option<Vec<ListEntry>>,
    pub latest_stable: Option<String>,
}

pub struct GameProcess {
    pub child: LaunchedProcess,
    pub receiver: Option<Receiver<LogLine>>,
    pub server_input: Option<(ChildStdin, bool)>,
}

impl Launcher {
    pub fn load_new(
        message: Option<String>,
        is_new_user: bool,
        config: Result<LauncherConfig, JsonFileError>,
    ) -> Result<Self, JsonFileError> {
        if let Err(err) = file_utils::get_launcher_dir() {
            err!("Could not get launcher dir (This is a bug):");
            return Ok(Self::with_error(format!(
                "Could not get launcher dir: {err}"
            )));
        }

        let mut config = config?;
        let theme = config.c_theme();
        let (window_width, window_height) = config.c_window_size();

        let mut launch = if let Some(message) = message {
            MenuLaunch::with_message(message)
        } else {
            MenuLaunch::default()
        };

        launch.resize_sidebar(SIDEBAR_WIDTH);

        let launch = State::Launch(launch);

        // The version field was added in 0.3
        let version = config.version.as_deref().unwrap_or("0.3.0");

        let state = if is_new_user {
            State::Welcome(MenuWelcome::P1InitialScreen)
        } else if version == LAUNCHER_VERSION_NAME {
            launch
        } else {
            config.version = Some(LAUNCHER_VERSION_NAME.to_owned());
            State::ChangeLog
        };

        let (accounts, accounts_dropdown, selected_account) = load_accounts(&mut config);

        Ok(Self {
            state,
            config,
            theme,
            accounts,
            accounts_dropdown,

            window_state: WindowState {
                size: (window_width, window_height),
                mouse_pos: (0.0, 0.0),
                is_maximized: false,
            },
            accounts_selected: Some(selected_account),

            client_list: None,
            server_list: None,
            java_recv: None,
            version_list_cache: VersionListCache::default(),
            selected_instance: None,
            custom_jar: None,
            #[cfg(feature = "discord_rpc")]
            discord: None,

            logs: HashMap::new(),
            processes: HashMap::new(),

            keys_pressed: HashSet::new(),
            mod_updates_checked: HashMap::new(),

            is_log_open: false,
            is_launching_game: false,

            log_scroll: 0,
            tick_timer: 0,

            autosave: HashSet::new(),
            images: ImageState::default(),
            modifiers_pressed: iced::keyboard::Modifiers::empty(),
        })
    }

    pub fn with_error(error: impl Display) -> Self {
        let error = error.to_string();
        let launcher_dir = if error.contains("Could not get launcher dir") {
            None
        } else {
            Some(LAUNCHER_DIR.clone())
        };

        let (config, theme) = launcher_dir
            .as_ref()
            .and_then(|_| {
                match LauncherConfig::load_s().map(|n| {
                    let theme = n.c_theme();
                    (n, theme)
                }) {
                    Ok(n) => Some(n),
                    Err(err) => {
                        err!("Error loading config: {err}");
                        None
                    }
                }
            })
            .unwrap_or((LauncherConfig::default(), LauncherTheme::default()));

        let (window_width, window_height) = config.c_window_size();

        Self {
            config,
            theme,

            state: State::Error { error },

            java_recv: None,
            client_list: None,
            server_list: None,
            selected_instance: None,
            custom_jar: None,
            #[cfg(feature = "discord_rpc")]
            discord: None,

            is_log_open: false,
            is_launching_game: false,

            log_scroll: 0,
            tick_timer: 0,

            logs: HashMap::new(),
            processes: HashMap::new(),
            accounts: HashMap::new(),
            keys_pressed: HashSet::new(),
            mod_updates_checked: HashMap::new(),

            images: ImageState::default(),
            window_state: WindowState {
                size: (window_width, window_height),
                mouse_pos: (0.0, 0.0),
                is_maximized: false,
            },
            autosave: HashSet::new(),
            version_list_cache: VersionListCache::default(),
            accounts_dropdown: vec![OFFLINE_ACCOUNT_NAME.to_owned(), NEW_ACCOUNT_NAME.to_owned()],
            accounts_selected: Some(OFFLINE_ACCOUNT_NAME.to_owned()),
            modifiers_pressed: iced::keyboard::Modifiers::empty(),
        }
    }

    pub fn instance(&self) -> &InstanceSelection {
        self.selected_instance.as_ref().unwrap()
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn set_error(&mut self, error: impl ToString) {
        let error = error.to_string().replace(CLIENT_ID, "[CLIENT ID]");
        err!("{error}");
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen<T: Display>(&mut self, message: Option<T>) -> Task<Message> {
        let mut menu_launch = match message {
            Some(message) => MenuLaunch::with_message(message.to_string()),
            None => MenuLaunch::default(),
        };
        menu_launch.resize_sidebar(SIDEBAR_WIDTH);
        self.state = State::Launch(menu_launch);
        Task::perform(get_entries(false), Message::CoreListLoaded)
    }
}

fn load_accounts(
    config: &mut LauncherConfig,
) -> (HashMap<String, AccountData>, Vec<String>, String) {
    let mut accounts = HashMap::new();

    let mut accounts_dropdown = vec![OFFLINE_ACCOUNT_NAME.to_owned(), NEW_ACCOUNT_NAME.to_owned()];

    if let Some(config_accounts) = config.accounts.as_mut() {
        let mut accounts_to_remove = Vec::new();

        for (username, account) in config_accounts.iter_mut() {
            load_account(
                &mut accounts,
                &mut accounts_dropdown,
                &mut accounts_to_remove,
                username,
                account,
            );
        }

        for rem in accounts_to_remove {
            config_accounts.remove(&rem);
        }
    }

    let selected_account = config.account_selected.clone().unwrap_or(
        accounts_dropdown
            .first()
            .cloned()
            .unwrap_or_else(|| OFFLINE_ACCOUNT_NAME.to_owned()),
    );
    (accounts, accounts_dropdown, selected_account)
}

fn load_account(
    accounts: &mut HashMap<String, AccountData>,
    accounts_dropdown: &mut Vec<String>,
    accounts_to_remove: &mut Vec<String>,
    username: &str,
    account: &mut crate::config::ConfigAccount,
) {
    fn get_refresh_token_for_account_type(
        account_type: AccountType,
        username: &str,
        keyring_identifier: Option<&str>,
    ) -> Result<String, String> {
        let keyring_username = if let Some(keyring_id) = keyring_identifier {
            keyring_id
        } else {
            // Fallback to old behavior for backwards compatibility
            match account_type {
                AccountType::ElyBy => username.strip_suffix(" (elyby)").unwrap_or(username),
                AccountType::LittleSkin => {
                    username.strip_suffix(" (littleskin)").unwrap_or(username)
                }
                AccountType::Microsoft => username,
            }
        };
        ql_instances::auth::read_refresh_token(keyring_username, account_type).strerr()
    }

    let (account_type, refresh_token) =
        if account.account_type.as_deref() == Some("ElyBy") || username.ends_with(" (elyby)") {
            (
                AccountType::ElyBy,
                get_refresh_token_for_account_type(
                    AccountType::ElyBy,
                    username,
                    account.keyring_identifier.as_deref(),
                ),
            )
        } else if account.account_type.as_deref() == Some("LittleSkin")
            || username.ends_with(" (littleskin)")
        {
            (
                AccountType::LittleSkin,
                get_refresh_token_for_account_type(
                    AccountType::LittleSkin,
                    username,
                    account.keyring_identifier.as_deref(),
                ),
            )
        } else {
            (
                AccountType::Microsoft,
                get_refresh_token_for_account_type(
                    AccountType::Microsoft,
                    username,
                    account.keyring_identifier.as_deref(),
                ),
            )
        };

    let keyring_username = if let Some(keyring_id) = &account.keyring_identifier {
        keyring_id.clone()
    } else {
        // Fallback to old behavior for backwards compatibility
        match account_type {
            AccountType::ElyBy => username
                .strip_suffix(" (elyby)")
                .unwrap_or(username)
                .to_owned(),
            AccountType::LittleSkin => username
                .strip_suffix(" (littleskin)")
                .unwrap_or(username)
                .to_owned(),
            AccountType::Microsoft => username.to_owned(),
        }
    };

    match refresh_token {
        Ok(refresh_token) => {
            accounts_dropdown.insert(0, username.to_owned());
            accounts.insert(
                username.to_owned(),
                AccountData {
                    access_token: None,
                    uuid: account.uuid.clone(),
                    refresh_token,
                    needs_refresh: true,
                    account_type,

                    username: keyring_username.clone(),
                    nice_username: account
                        .username_nice
                        .clone()
                        .unwrap_or(keyring_username.clone()),
                },
            );
        }
        Err(err) => {
            err!(
                "Could not load account: {err}\nUsername: {keyring_username}, Account Type: {}",
                account_type.to_string()
            );
            accounts_to_remove.push(username.to_owned());
        }
    }
}

pub async fn get_entries(is_server: bool) -> Res<(Vec<String>, bool)> {
    let dir_path = file_utils::get_launcher_dir().strerr()?.join(if is_server {
        "servers"
    } else {
        "instances"
    });
    if !dir_path.exists() {
        tokio::fs::create_dir_all(&dir_path)
            .await
            .path(&dir_path)
            .strerr()?;
        return Ok((Vec::new(), is_server));
    }

    Ok((
        file_utils::read_filenames_from_dir(&dir_path)
            .await
            .strerr()?
            .into_iter()
            .filter(|n| !n.is_file)
            .map(|n| n.name)
            .collect(),
        is_server,
    ))
}

pub struct ProgressBar<T: Progress> {
    pub num: f32,
    pub message: Option<String>,
    pub receiver: Receiver<T>,
    pub progress: T,
}

impl<T: Default + Progress> ProgressBar<T> {
    pub fn with_recv(receiver: Receiver<T>) -> Self {
        Self {
            num: 0.0,
            message: None,
            receiver,
            progress: T::default(),
        }
    }

    pub fn with_recv_and_msg(receiver: Receiver<T>, msg: String) -> Self {
        Self {
            num: 0.0,
            message: Some(msg),
            receiver,
            progress: T::default(),
        }
    }
}

impl<T: Progress> ProgressBar<T> {
    pub fn tick(&mut self) -> bool {
        let mut has_ticked = false;
        while let Ok(progress) = self.receiver.try_recv() {
            self.num = progress.get_num();
            self.message = progress.get_message();
            self.progress = progress;
            has_ticked = true;
        }
        has_ticked
    }
}

pub async fn load_custom_jars() -> Result<Vec<String>, IoError> {
    let names = file_utils::read_filenames_from_dir(LAUNCHER_DIR.join("custom_jars")).await?;
    let mut list: Vec<String> = names
        .into_iter()
        .filter(|n| n.is_file)
        .map(|n| n.name)
        .collect();

    list.insert(0, NONE_JAR_NAME.to_owned());
    list.push(ADD_JAR_NAME.to_owned());
    list.push(REMOVE_JAR_NAME.to_owned());
    list.push(OPEN_FOLDER_JAR_NAME.to_owned());

    Ok(list)
}

pub fn dir_watch<P: AsRef<Path>>(
    path: P,
) -> notify::Result<(Receiver<notify::Event>, notify::RecommendedWatcher)> {
    let (tx, rx) = mpsc::channel();

    // `notify` runs callbacks in its own thread.
    let mut watcher: notify::RecommendedWatcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            _ = tx.send(event);
        }
    })?;
    watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

    Ok((rx, watcher))
}
