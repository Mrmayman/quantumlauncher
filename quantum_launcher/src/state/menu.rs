use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    config::SIDEBAR_WIDTH, message_handler::get_locally_installed_mods, state::NotesMessage,
};
use ezshortcut::Shortcut;
use frostmark::MarkState;
use iced::{
    widget::{self, scrollable::AbsoluteOffset},
    Task,
};
use ql_core::{
    file_utils::DirItem,
    jarmod::JarMods,
    json::{instance_config::MainClassMode, InstanceConfigJson, VersionDetails},
    DownloadProgress, GenericProgress, InstanceSelection, IntoStringError, ListEntry, ModId,
    OptifineUniqueVersion, SelectedMod, StoreBackendType,
};
use ql_mod_manager::loaders::paper::PaperVersion;
use ql_mod_manager::{
    loaders::{self, forge::ForgeInstallProgress, optifine::OptifineInstallProgress},
    store::{CurseforgeNotAllowed, ModConfig, ModIndex, QueryType, RecommendedMod, SearchResult},
};

use crate::state::ImageState;

use super::{ManageModsMessage, Message, ProgressBar};

#[derive(Clone, PartialEq, Eq, Debug, Default, Copy)]
pub enum LaunchTab {
    #[default]
    Buttons,
    Log,
    Edit,
}

impl std::fmt::Display for LaunchTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LaunchTab::Buttons => "Play",
                LaunchTab::Log => "Logs",
                LaunchTab::Edit => "Edit",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchModal {
    InstanceOptions,
    // More in the future
}

pub enum InstanceNotes {
    Viewing {
        content: String,
        mark_state: MarkState,
    },
    Editing {
        original: String,
        text_editor: widget::text_editor::Content,
    },
}

impl InstanceNotes {
    pub fn get_text(&self) -> &str {
        match self {
            InstanceNotes::Viewing { content, .. } => content,
            InstanceNotes::Editing { original, .. } => original,
        }
    }
}

pub struct LogState {
    pub content: widget::text_editor::Content,
}

/// The home screen of the launcher.
pub struct MenuLaunch {
    pub message: String,
    pub login_progress: Option<ProgressBar<GenericProgress>>,
    pub tab: LaunchTab,
    pub edit_instance: Option<MenuEditInstance>,
    pub notes: Option<InstanceNotes>,
    pub log_state: Option<LogState>,
    pub modal: Option<LaunchModal>,

    pub sidebar_scrolled: f32,
    pub sidebar_grid_state: widget::pane_grid::State<bool>,
    sidebar_split: Option<widget::pane_grid::Split>,

    pub is_viewing_server: bool,
    pub is_uploading_mclogs: bool,
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
            sidebar_grid_state.resize(split, SIDEBAR_WIDTH);
            Some(split)
        } else {
            None
        };
        Self {
            message,
            tab: LaunchTab::default(),
            edit_instance: None,
            login_progress: None,
            sidebar_scrolled: 100.0,
            is_viewing_server: false,
            sidebar_grid_state,
            log_state: None,
            is_uploading_mclogs: false,
            sidebar_split,
            notes: None,
            modal: None,
        }
    }

    pub fn resize_sidebar(&mut self, width: f32) {
        if let Some(split) = self.sidebar_split {
            self.sidebar_grid_state.resize(split, width);
        }
    }

    pub fn reload_notes(&mut self, instance: InstanceSelection) -> Task<Message> {
        self.notes = None;
        Task::perform(ql_instances::notes::read(instance), |n| {
            Message::Notes(NotesMessage::Loaded(n.strerr()))
        })
    }
}

/// The screen where you can edit an instance/server.
pub struct MenuEditInstance {
    pub config: InstanceConfigJson,

    // Renaming Instance:
    pub is_editing_name: bool,
    pub instance_name: String,
    pub old_instance_name: String,
    // Changing RAM:
    pub slider_value: f32,
    pub slider_text: String,
    pub memory_input: String,

    pub main_class_mode: Option<MainClassMode>,
    pub arg_split_by_space: bool,
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

    pub list_scroll: AbsoluteOffset,
    /// Index of the item selected before pressing shift
    pub list_shift_index: Option<usize>,
    pub drag_and_drop_hovered: bool,
    pub modal: Option<MenuEditModsModal>,
    pub search: Option<String>,

    pub width_name: f32,
}

#[derive(Debug, Clone)]
pub enum MenuEditModsModal {
    Submenu,
    RightClick(ModId, (f32, f32)),
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

    pub fn is_selected(&self, clicked_id: &ModId) -> bool {
        self.selected_mods.iter().any(|n| {
            if let SelectedMod::Downloaded { id, .. } = n {
                id == clicked_id
            } else {
                false
            }
        })
    }
}

pub struct MenuExportMods {
    pub selected_mods: HashSet<SelectedMod>,
}

pub struct MenuEditJarMods {
    pub jarmods: JarMods,
    pub selected_state: SelectedState,
    pub selected_mods: HashSet<String>,
    pub drag_and_drop_hovered: bool,
}

pub enum MenuCreateInstance {
    Choosing(MenuCreateInstanceChoosing),
    DownloadingInstance(ProgressBar<DownloadProgress>),
    ImportingInstance(ProgressBar<GenericProgress>),
}

pub struct MenuCreateInstanceChoosing {
    pub _loading_list_handle: iced::task::Handle,
    pub list: Option<Vec<ListEntry>>,
    // UI:
    pub is_server: bool,
    pub search_box: String,
    pub show_category_dropdown: bool,
    pub selected_categories: HashSet<ql_core::ListEntryKind>,
    // Sidebar resizing:
    pub sidebar_grid_state: widget::pane_grid::State<bool>,
    pub sidebar_split: Option<widget::pane_grid::Split>,
    // Instance info:
    pub selected_version: ListEntry,
    pub instance_name: String,
    pub download_assets: bool,
}

pub enum MenuInstallFabric {
    Loading {
        is_quilt: bool,
        _loading_handle: iced::task::Handle,
    },
    Loaded {
        backend: loaders::fabric::BackendType,
        fabric_version: String,
        fabric_versions: loaders::fabric::FabricVersionList,
        progress: Option<ProgressBar<GenericProgress>>,
    },
    Unsupported(bool),
}

impl MenuInstallFabric {
    pub fn is_quilt(&self) -> bool {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. }
            | MenuInstallFabric::Unsupported(is_quilt) => *is_quilt,
            MenuInstallFabric::Loaded { backend, .. } => backend.is_quilt(),
        }
    }
}

pub enum MenuInstallPaper {
    Loading {
        _handle: iced::task::Handle,
    },
    Loaded {
        version: PaperVersion,
        versions: Vec<PaperVersion>,
    },
    Installing,
}

pub struct MenuInstallForge {
    pub forge_progress: ProgressBar<ForgeInstallProgress>,
    pub java_progress: ProgressBar<GenericProgress>,
    pub is_java_getting_installed: bool,
}

#[allow(unused)]
pub struct MenuLauncherUpdate {
    pub url: String,
    pub progress: Option<ProgressBar<GenericProgress>>,
}

#[derive(Clone, Copy, Debug)]
pub enum ModOperation {
    Downloading,
    Deleting,
}

pub struct MenuModsDownload {
    pub query: String,
    pub results: Option<SearchResult>,
    pub description: Option<MarkState>,

    pub mod_descriptions: HashMap<ModId, String>,
    pub mods_download_in_progress: HashMap<ModId, (String, ModOperation)>,
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
    pub arg_split_by_space: bool,
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
    pub include_config: bool,

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
    pub is_incorrect_password: bool,

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

    InstallPaper(MenuInstallPaper),
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),

    InstallJava,

    ModsDownload(MenuModsDownload),
    LauncherSettings(MenuLauncherSettings),
    ManagePresets(MenuEditPresets),
    RecommendedMods(MenuRecommendedMods),

    LogUploadResult {
        url: String,
    },
    CreateShortcut(MenuShortcut),

    License(MenuLicense),
}

pub struct MenuShortcut {
    pub shortcut: Shortcut,
    pub add_to_menu: bool,
    pub add_to_desktop: bool,
    pub account: String,
    pub account_offline: String,
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
            if let OptifineUniqueVersion::Forge = o {
                OPTIFINE_DOWNLOADS
            } else {
                o.get_url().0
            }
        } else {
            OPTIFINE_DOWNLOADS
        }
    }
}
