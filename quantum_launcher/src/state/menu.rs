use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use frostmark::MarkState;
use iced::{
    widget::{self, scrollable::AbsoluteOffset},
    Task,
};
use ql_core::{
    file_utils::DirItem,
    jarmod::JarMods,
    json::{InstanceConfigJson, VersionDetails},
    DownloadProgress, GenericProgress, InstanceSelection, ListEntry, ModId, OptifineUniqueVersion,
    SelectedMod, StoreBackendType,
};
use ql_mod_manager::{
    loaders::{forge::ForgeInstallProgress, optifine::OptifineInstallProgress},
    store::{CurseforgeNotAllowed, ModConfig, ModIndex, QueryType, RecommendedMod, SearchResult},
};

use crate::{
    config::SIDEBAR_WIDTH_DEFAULT, message_handler::get_locally_installed_mods, state::ImageState,
    WINDOW_WIDTH,
};

use super::{ManageModsMessage, Message, ProgressBar};

#[derive(Clone, PartialEq, Eq, Debug, Default, Copy)]
pub enum LaunchTabId {
    #[default]
    Buttons,
    Log,
    Edit,
}

impl std::fmt::Display for LaunchTabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LaunchTabId::Buttons => "Play",
                LaunchTabId::Log => "Log",
                LaunchTabId::Edit => "Edit",
            }
        )
    }
}

/// The home screen of the launcher.
pub struct MenuLaunch {
    pub message: String,
    pub login_progress: Option<ProgressBar<GenericProgress>>,
    pub tab: LaunchTabId,
    pub edit_instance: Option<MenuEditInstance>,

    sidebar_width: u64,
    pub sidebar_scrolled: f32,
    pub sidebar_grid_state: widget::pane_grid::State<bool>,
    sidebar_split: Option<widget::pane_grid::Split>,

    pub is_viewing_server: bool,
    pub is_uploading_mclogs: bool,
    pub log_scroll: isize,
}

impl Default for MenuLaunch {
    fn default() -> Self {
        Self::with_message(String::new())
    }
}

impl MenuLaunch {
    pub fn with_message(message: String) -> Self {
        let (mut sidebar_grid_state, pane) = widget::pane_grid::State::new(true);
        let sidebar_split = if let Some((_, split)) =
            sidebar_grid_state.split(widget::pane_grid::Axis::Vertical, pane, false)
        {
            sidebar_grid_state.resize(split, SIDEBAR_WIDTH_DEFAULT as f32 / WINDOW_WIDTH);
            Some(split)
        } else {
            None
        };
        Self {
            message,
            tab: LaunchTabId::default(),
            edit_instance: None,
            login_progress: None,
            sidebar_width: SIDEBAR_WIDTH_DEFAULT as u64,
            sidebar_scrolled: 100.0,
            is_viewing_server: false,
            sidebar_grid_state,
            log_scroll: 0,
            is_uploading_mclogs: false,
            sidebar_split,
        }
    }

    pub fn resize_sidebar(&mut self, width: f32, window_width: f32) {
        if let Some(split) = self.sidebar_split {
            self.sidebar_width = width as u64;
            self.sidebar_grid_state.resize(split, width / window_width);
        }
    }

    pub fn get_sidebar_width(&self) -> f32 {
        self.sidebar_width as f32
    }
}

/// The screen where you can edit an instance/server.
pub struct MenuEditInstance {
    pub config: InstanceConfigJson,
    pub is_editing_name: bool,
    pub instance_name: String,
    pub old_instance_name: String,
    pub slider_value: f32,
    pub slider_text: String,
}

pub enum SelectedState {
    All,
    Some,
    None,
}

#[derive(Debug, Clone)]
pub enum ModListEntry {
    Downloaded { id: ModId, config: Box<ModConfig> },
    Local { file_name: String },
}

impl ModListEntry {
    pub fn is_manually_installed(&self) -> bool {
        match self {
            ModListEntry::Local { .. } => true,
            ModListEntry::Downloaded { config, .. } => config.manually_installed,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ModListEntry::Local { file_name } => file_name,
            ModListEntry::Downloaded { config, .. } => &config.name,
        }
    }
}

impl From<ModListEntry> for SelectedMod {
    fn from(value: ModListEntry) -> Self {
        match value {
            ModListEntry::Local { file_name } => SelectedMod::Local {
                file_name: file_name.clone(),
            },
            ModListEntry::Downloaded { id, config } => SelectedMod::Downloaded {
                name: config.name.clone(),
                id: id.clone(),
            },
        }
    }
}

impl PartialEq<ModListEntry> for SelectedMod {
    fn eq(&self, other: &ModListEntry) -> bool {
        match (self, other) {
            (
                SelectedMod::Downloaded { name, id },
                ModListEntry::Downloaded { id: id2, config },
            ) => id == id2 && *name == config.name,
            (SelectedMod::Local { file_name }, ModListEntry::Local { file_name: name2 }) => {
                file_name == name2
            }
            _ => false,
        }
    }
}

pub struct MenuEditMods {
    pub mod_update_progress: Option<ProgressBar<GenericProgress>>,

    pub config: InstanceConfigJson,
    pub mods: ModIndex,
    // TODO: Use this for dynamically adjusting installable loader buttons
    pub version_json: Box<VersionDetails>,

    pub locally_installed_mods: HashSet<String>,
    pub sorted_mods_list: Vec<ModListEntry>,

    pub selected_mods: HashSet<SelectedMod>,
    pub shift_selected_mods: HashSet<SelectedMod>,
    pub selected_state: SelectedState,

    pub update_check_handle: Option<iced::task::Handle>,
    pub available_updates: Vec<(ModId, String, bool)>,

    /// Index of the item selected before pressing shift
    pub list_shift_index: Option<usize>,
    pub drag_and_drop_hovered: bool,
    pub submenu1_shown: bool,

    pub width_name: f32,
}

impl MenuEditMods {
    pub fn update_locally_installed_mods(
        idx: &ModIndex,
        selected_instance: &InstanceSelection,
    ) -> Task<Message> {
        let mut blacklist = Vec::new();
        for mod_info in idx.mods.values() {
            for file in &mod_info.files {
                blacklist.push(file.filename.clone());
                blacklist.push(format!("{}.disabled", file.filename));
            }
        }
        Task::perform(
            get_locally_installed_mods(selected_instance.get_dot_minecraft_path(), blacklist),
            |n| Message::ManageMods(ManageModsMessage::LocalIndexLoaded(n)),
        )
    }

    /// Returns two `Vec`s that are:
    /// - The IDs of downloaded mods
    /// - The filenames of local mods
    ///
    /// ...respectively, from the mods selected in the mod menu.
    pub fn get_kinds_of_ids(&self) -> (Vec<String>, Vec<String>) {
        let ids_downloaded = self
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    Some(id.get_index_str())
                } else {
                    None
                }
            })
            .collect();

        let ids_local: Vec<String> = self
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Local { file_name } = s_mod {
                    Some(file_name.clone())
                } else {
                    None
                }
            })
            .collect();
        (ids_downloaded, ids_local)
    }

    pub fn update_selected_state(&mut self) {
        self.selected_state = if self.selected_mods.is_empty() {
            SelectedState::None
        } else if self.selected_mods.len() == self.sorted_mods_list.len() {
            SelectedState::All
        } else {
            SelectedState::Some
        };
    }
}

pub struct MenuExportMods {
    pub selected_mods: HashSet<SelectedMod>,
}

pub struct MenuEditJarMods {
    pub jarmods: Option<JarMods>,
    pub selected_state: SelectedState,
    pub selected_mods: HashSet<String>,
    pub drag_and_drop_hovered: bool,
    pub free_for_autosave: bool,
}

pub enum MenuCreateInstance {
    LoadingList {
        _handle: iced::task::Handle,
    },
    Choosing {
        instance_name: String,
        selected_version: Option<ListEntry>,
        download_assets: bool,
        combo_state: Box<iced::widget::combo_box::State<ListEntry>>,
    },
    DownloadingInstance(ProgressBar<DownloadProgress>),
    ImportingInstance(ProgressBar<GenericProgress>),
}

pub enum MenuInstallFabric {
    Loading {
        is_quilt: bool,
        _loading_handle: iced::task::Handle,
    },
    Loaded {
        is_quilt: bool,
        fabric_version: String,
        fabric_versions: Vec<String>,
        progress: Option<ProgressBar<GenericProgress>>,
    },
    Unsupported(bool),
}

impl MenuInstallFabric {
    pub fn is_quilt(&self) -> bool {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. }
            | MenuInstallFabric::Loaded { is_quilt, .. }
            | MenuInstallFabric::Unsupported(is_quilt) => *is_quilt,
        }
    }
}

pub struct MenuInstallForge {
    pub forge_progress: ProgressBar<ForgeInstallProgress>,
    pub java_progress: ProgressBar<GenericProgress>,
    pub is_java_getting_installed: bool,
}

pub struct MenuLauncherUpdate {
    pub url: String,
    pub progress: Option<ProgressBar<GenericProgress>>,
}

pub struct MenuModsDownload {
    pub query: String,
    pub results: Option<SearchResult>,
    pub description: Option<MarkState>,

    pub mod_descriptions: HashMap<ModId, String>,
    pub mods_download_in_progress: HashMap<ModId, String>,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub scroll_offset: AbsoluteOffset,

    pub version_json: Box<VersionDetails>,
    pub config: InstanceConfigJson,
    pub mod_index: ModIndex,

    pub backend: StoreBackendType,
    pub query_type: QueryType,

    /// This is for the loading of continuation of the search,
    /// i.e. when you scroll down and more stuff appears
    pub is_loading_continuation: bool,
    pub has_continuation_ended: bool,
}

impl MenuModsDownload {
    pub fn reload_description(&mut self, images: &mut ImageState) {
        let (Some(selection), Some(results)) = (self.opened_mod, &self.results) else {
            return;
        };
        let Some(hit) = results.mods.get(selection) else {
            return;
        };
        let Some(info) = self
            .mod_descriptions
            .get(&ModId::from_pair(&hit.id, results.backend))
        else {
            return;
        };
        let description = match results.backend {
            StoreBackendType::Modrinth => MarkState::with_html_and_markdown(info),
            StoreBackendType::Curseforge => MarkState::with_html(info), // Optimization, curseforge only has HTML
        };
        let imgs = description.find_image_links();
        self.description = Some(description);

        for img in imgs {
            images.queue(&img);
        }
    }
}

pub struct MenuLauncherSettings {
    pub temp_scale: f64,
    pub selected_tab: LauncherSettingsTab,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LauncherSettingsTab {
    UserInterface,
    Internal,
    About,
}

impl std::fmt::Display for LauncherSettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LauncherSettingsTab::UserInterface => "Appearance",
                LauncherSettingsTab::Internal => "Game",
                LauncherSettingsTab::About => "About",
            }
        )
    }
}

impl LauncherSettingsTab {
    pub const ALL: &'static [Self] = &[Self::UserInterface, Self::Internal, Self::About];

    pub const fn next(self) -> Self {
        match self {
            Self::UserInterface => Self::Internal,
            Self::Internal | Self::About => Self::About,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::UserInterface | Self::Internal => Self::UserInterface,
            Self::About => Self::Internal,
        }
    }
}

pub struct MenuEditPresets {
    pub selected_mods: HashSet<SelectedMod>,
    pub selected_state: SelectedState,
    pub is_building: bool,

    pub progress: Option<ProgressBar<GenericProgress>>,
    pub sorted_mods_list: Vec<ModListEntry>,
    pub drag_and_drop_hovered: bool,
}

pub enum MenuRecommendedMods {
    Loading {
        progress: ProgressBar<GenericProgress>,
        config: InstanceConfigJson,
    },
    Loaded {
        mods: Vec<(bool, RecommendedMod)>,
        config: InstanceConfigJson,
    },
    InstallALoader,
    NotSupported,
}

pub enum MenuWelcome {
    P1InitialScreen,
    P2Theme,
    P3Auth,
}

pub struct MenuCurseforgeManualDownload {
    pub unsupported: HashSet<CurseforgeNotAllowed>,
    pub is_store: bool,
    pub delete_mods: bool,
}

pub struct MenuExportInstance {
    pub entries: Option<Vec<(DirItem, bool)>>,
    pub progress: Option<ProgressBar<GenericProgress>>,
}

pub struct MenuLoginAlternate {
    pub username: String,
    pub password: String,
    pub show_password: bool,

    pub is_loading: bool,
    pub otp: Option<String>,

    pub is_from_welcome_screen: bool,

    pub is_littleskin: bool,
    pub oauth: Option<LittleSkinOauth>,
    pub device_code_error: Option<String>,
}

pub struct LittleSkinOauth {
    // pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub device_code_expires_at: Instant,
}

pub struct MenuLoginMS {
    pub url: String,
    pub code: String,
    pub is_from_welcome_screen: bool,
    pub _cancel_handle: iced::task::Handle,
}

/// The enum that represents which menu is opened currently.
pub enum State {
    /// Default home screen
    Launch(MenuLaunch),
    Create(MenuCreateInstance),
    /// Screen to guide new users to the launcher
    Welcome(MenuWelcome),
    ChangeLog,
    UpdateFound(MenuLauncherUpdate),

    EditMods(MenuEditMods),
    ExportMods(MenuExportMods),
    EditJarMods(MenuEditJarMods),
    ImportModpack(ProgressBar<GenericProgress>),
    CurseforgeManualDownload(MenuCurseforgeManualDownload),
    ExportInstance(MenuExportInstance),

    Error {
        error: String,
    },
    /// "Are you sure you want to {msg1}?"
    /// screen. Used for confirming if the user
    /// wants to do certain actions.
    ConfirmAction {
        msg1: String,
        msg2: String,
        yes: Message,
        no: Message,
    },
    GenericMessage(String),

    /// Progress bar when logging into accounts
    AccountLoginProgress(ProgressBar<GenericProgress>),
    /// A parent menu to choose whether you want to log in
    /// with Microsoft, `ely.by`, `littleskin`, etc.
    AccountLogin,
    LoginMS(MenuLoginMS),
    LoginAlternate(MenuLoginAlternate),

    InstallPaper,
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),

    InstallJava,

    ModsDownload(MenuModsDownload),
    LauncherSettings(MenuLauncherSettings),
    ServerCreate(MenuServerCreate),
    ManagePresets(MenuEditPresets),
    RecommendedMods(MenuRecommendedMods),

    LogUploadResult {
        url: String,
    },

    License(MenuLicense),
}

pub struct MenuLicense {
    pub selected_tab: LicenseTab,
    pub content: iced::widget::text_editor::Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LicenseTab {
    Gpl3,
    ForgeInstallerApache,
    OpenFontLicense,
    PasswordAsterisks,
    Lwjgl,
}

impl LicenseTab {
    pub const ALL: &'static [Self] = &[
        Self::Gpl3,
        Self::ForgeInstallerApache,
        Self::OpenFontLicense,
        Self::PasswordAsterisks,
        Self::Lwjgl,
    ];

    pub const fn next(self) -> Self {
        match self {
            Self::Gpl3 => Self::ForgeInstallerApache,
            Self::ForgeInstallerApache => Self::OpenFontLicense,
            Self::OpenFontLicense => Self::PasswordAsterisks,
            Self::PasswordAsterisks | Self::Lwjgl => Self::Lwjgl,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Gpl3 | Self::ForgeInstallerApache => Self::Gpl3,
            Self::OpenFontLicense => Self::ForgeInstallerApache,
            Self::PasswordAsterisks => Self::OpenFontLicense,
            Self::Lwjgl => Self::PasswordAsterisks,
        }
    }

    pub fn get_text(self) -> &'static str {
        match self {
            LicenseTab::Gpl3 => include_str!("../../../LICENSE"),
            LicenseTab::OpenFontLicense => {
                concat!(
                    "For the Inter and JetBrains fonts used in QuantumLauncher:\n--------\n\n",
                    include_str!("../../../assets/licenses/OFL.txt"),
                )
            }
            LicenseTab::PasswordAsterisks => {
                concat!(
                    include_str!("../../../assets/fonts/password_asterisks/where.txt"),
                    "\n--------\n",
                    include_str!("../../../assets/licenses/CC_BY_SA_3_0.txt")
                )
            }
            LicenseTab::ForgeInstallerApache => {
                concat!(
                    "For the Forge Installer script used in QuantumLauncher:\n--------\n\n",
                    include_str!("../../../assets/licenses/APACHE_2.txt")
                )
            }
            LicenseTab::Lwjgl => include_str!("../../../assets/licenses/LWJGL.txt"),
        }
    }
}

impl std::fmt::Display for LicenseTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            LicenseTab::Gpl3 => "QuantumLauncher",
            LicenseTab::OpenFontLicense => "Fonts (Inter/Jetbrains Mono)",
            LicenseTab::PasswordAsterisks => "Password Asterisks Font",
            LicenseTab::ForgeInstallerApache => "Forge Installer",
            LicenseTab::Lwjgl => "LWJGL",
        };
        write!(f, "{name}")
    }
}

pub enum MenuServerCreate {
    LoadingList,
    Loaded {
        name: String,
        versions: Box<iced::widget::combo_box::State<ListEntry>>,
        selected_version: Option<ListEntry>,
    },
    Downloading {
        progress: ProgressBar<GenericProgress>,
    },
}

pub enum MenuInstallOptifine {
    Choosing {
        optifine_unique_version: Option<OptifineUniqueVersion>,
        delete_installer: bool,
        drag_and_drop_hovered: bool,
    },
    Installing {
        optifine_install_progress: ProgressBar<OptifineInstallProgress>,
        java_install_progress: Option<ProgressBar<GenericProgress>>,
        is_java_being_installed: bool,
    },
    InstallingB173,
}

impl MenuInstallOptifine {
    pub fn get_url(&self) -> &'static str {
        const OPTIFINE_DOWNLOADS: &str = "https://optifine.net/downloads";

        if let Self::Choosing {
            optifine_unique_version: Some(o),
            ..
        } = self
        {
            o.get_url().0
        } else {
            OPTIFINE_DOWNLOADS
        }
    }
}
