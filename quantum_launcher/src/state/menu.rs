use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::{
    config::{
        SIDEBAR_WIDTH,
        sidebar::{FolderId, SDragLocation, SidebarSelection},
    },
    message_handler::get_locally_installed_mods,
    state::{FsWatcher, NotesMessage},
};
use ezshortcut::Shortcut;
use frostmark::MarkState;
use iced::{
    Rectangle, Task,
    widget::{self, scrollable::AbsoluteOffset},
};
use ql_core::{
    DownloadProgress, GenericProgress, Instance, InstanceKind, IntoStringError, ListEntry,
    OptifineUniqueVersion,
    file_utils::DirItem,
    flags::log_verbose,
    jarmod::JarMods,
    json::{InstanceConfigJson, VersionDetails, instance_config::MainClassMode},
    pt,
};
use ql_mod_manager::{
    loaders::paper::PaperVersion,
    store::{Category, LocalMod, SearchMod},
};
use ql_mod_manager::{
    loaders::{self, forge::ForgeInstallProgress, optifine::OptifineInstallProgress},
    store::{
        CurseforgeNotAllowed, ModConfig, ModId, ModIndex, QueryType, RecommendedMod, SearchResult,
        SelectedMod, StoreBackendType,
    },
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
        f.write_str(match self {
            LaunchTab::Buttons => "Play",
            LaunchTab::Log => "Logs",
            LaunchTab::Edit => "Edit",
        })
    }
}

#[derive(Debug, Clone)]
pub enum LaunchModal {
    InstanceOptions,

    // Sidebar
    SCtxMenu(Option<(SidebarSelection, Arc<str>)>, (f32, f32)),
    SDragging {
        being_dragged: SidebarSelection,
        dragged_to: Option<SDragLocation>,
    },
    SRenamingFolder(FolderId, String, bool),
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
    pub message: Option<InfoMessage>,
    pub login_progress: Option<ProgressBar<GenericProgress>>,
    pub tab: LaunchTab,
    pub edit_instance: Option<MenuEditInstance>,
    pub notes: Option<InstanceNotes>,
    pub log_state: Option<LogState>,
    pub modal: Option<LaunchModal>,

    pub sidebar_scroll: SidebarScroll,
    pub sidebar_grid_state: widget::pane_grid::State<bool>,
    sidebar_split: Option<widget::pane_grid::Split>,

    pub is_uploading_mclogs: bool,
}

impl Default for MenuLaunch {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SidebarScroll {
    pub remaining: f32,
    pub offset: f32,
    pub bounds: Option<Rectangle>,
}

impl Default for SidebarScroll {
    fn default() -> Self {
        Self {
            remaining: 100.0,
            offset: 0.0,
            bounds: None,
        }
    }
}

impl MenuLaunch {
    pub fn new(message: Option<InfoMessage>) -> Self {
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
            sidebar_scroll: SidebarScroll::default(),
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

    pub fn reload_notes(&mut self, instance: Instance) -> Task<Message> {
        self.notes = None;
        Task::perform(ql_instances::notes::read(instance), |n| {
            NotesMessage::Loaded(n.strerr()).into()
        })
    }

    pub fn get_modal_drag(&self) -> Option<(&SidebarSelection, Option<&SDragLocation>)> {
        if let Some(LaunchModal::SDragging {
            being_dragged,
            dragged_to,
        }) = &self.modal
        {
            return Some((being_dragged, dragged_to.as_ref()));
        }
        None
    }
}

/// The screen where you can edit an instance/server.
pub struct MenuEditInstance {
    pub config: InstanceConfigJson,

    pub state_rename: EditInstanceRename,
    pub state_ram: EditInstanceRam,

    pub main_class_mode: Option<MainClassMode>,
    pub arg_split_by_space: bool,
}

pub struct EditInstanceRename {
    pub is_editing: bool,
    pub name: String,
    pub old_name: Arc<str>,
}

pub struct EditInstanceRam {
    pub slider_value: f32,
    pub slider_text: String,
    pub memory_input: String,
    pub system: sysinfo::System,
}

pub enum SelectedState {
    All,
    Some,
    None,
}

#[derive(Debug, Clone)]
pub enum ModListEntry {
    Downloaded { id: ModId, config: Box<ModConfig> },
    Local(LocalMod),
}

impl ModListEntry {
    pub fn is_manually_installed(&self) -> bool {
        match self {
            ModListEntry::Local(_) => true,
            ModListEntry::Downloaded { config, .. } => config.manually_installed,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ModListEntry::Local(l) => &l.0,
            ModListEntry::Downloaded { config, .. } => &config.name,
        }
    }

    /// Returns the project type ("mod", "resourcepack", "shader", "datapack")
    pub fn project_type(&self) -> QueryType {
        match self {
            ModListEntry::Local(l) => l.1,
            ModListEntry::Downloaded { config, .. } => config.project_type,
        }
    }
}

impl From<ModListEntry> for SelectedMod {
    fn from(value: ModListEntry) -> Self {
        match value {
            ModListEntry::Local(l) => SelectedMod::Local(l),
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
            (SelectedMod::Local(l1), ModListEntry::Local(l2)) => l1 == l2,
            _ => false,
        }
    }
}

impl From<LocalMod> for ModListEntry {
    fn from(l: LocalMod) -> Self {
        ModListEntry::Local(l)
    }
}

impl From<&LocalMod> for ModListEntry {
    fn from(l: &LocalMod) -> Self {
        ModListEntry::Local(l.clone())
    }
}

pub struct MenuEditMods {
    pub locally_installed_mods: HashSet<LocalMod>,
    pub sorted_mods_list: Vec<ModListEntry>,

    pub file_data: EditModsFileData,
    pub updates: EditModsUpdates,
    pub selection: EditModsSelection,
    pub ui_state: EditModsUiState,

    pub search: Option<String>,
    pub content_filter: Option<QueryType>,
}

impl MenuEditMods {
    pub fn toggle_local_mods_in_ui(&mut self, ids_local: &[LocalMod]) {
        let remove_old = |n: &SelectedMod| {
            if let SelectedMod::Local(l) = n {
                !ids_local.contains(l)
            } else {
                true
            }
        };

        let add_new = || {
            ids_local
                .iter()
                .map(|n| LocalMod(Arc::from(ql_mod_manager::store::flip_filename(&n.0)), n.1))
        };

        self.selection.selected_mods.retain(remove_old);
        self.selection
            .selected_mods
            .extend(add_new().map(SelectedMod::Local));
        self.locally_installed_mods
            .retain(|l| !ids_local.contains(l));
        self.locally_installed_mods.extend(add_new());
    }

    pub fn sort_mods(&mut self) {
        let downloaded_mods = &self.file_data.mod_index.mods;
        let locally_installed_mods = &self.locally_installed_mods;

        let mut entries: Vec<ModListEntry> = downloaded_mods
            .iter()
            .map(|(id, c)| ModListEntry::Downloaded {
                id: id.clone(),
                config: Box::new(c.clone()),
            })
            .chain(locally_installed_mods.iter().map(ModListEntry::from))
            .collect();

        entries.sort_by(|val1, val2| match (val1, val2) {
            (ModListEntry::Downloaded { .. }, ModListEntry::Local(_)) => Ordering::Less,
            (ModListEntry::Local(_), ModListEntry::Downloaded { .. }) => Ordering::Greater,

            (
                ModListEntry::Downloaded { config: c1, .. },
                ModListEntry::Downloaded { config: c2, .. },
            ) => match (c1.manually_installed, c2.manually_installed) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => match c1.project_type.cmp(&c2.project_type) {
                    Ordering::Equal => c1.name.cmp(&c2.name),
                    other => other,
                },
            },
            (ModListEntry::Local(l1), ModListEntry::Local(l2)) => match l1.1.cmp(&l2.1) {
                Ordering::Equal => l1.0.cmp(&l2.0),
                other => other,
            },
        });

        self.sorted_mods_list = entries;
    }
}

pub struct EditModsUpdates {
    pub progress: Option<ProgressBar<GenericProgress>>,
    pub check_handle: Option<iced::task::Handle>,
    pub available: Vec<(ModId, String, bool)>,
}

pub struct EditModsSelection {
    pub selected_mods: HashSet<SelectedMod>,
    pub shift_selected_mods: HashSet<SelectedMod>,
    pub state: SelectedState,
    /// Index of the item selected before pressing shift
    pub list_shift_index: Option<usize>,
}

pub struct EditModsUiState {
    pub info_message: Option<InfoMessage>,
    pub list_scroll: AbsoluteOffset,
    pub drag_and_drop_hovered: bool,
    pub modal: Option<MenuEditModsModal>,
    pub width_name: f32,
}

pub struct EditModsFileData {
    pub config: InstanceConfigJson,
    pub mod_index: ModIndex,
    // TODO: Use this for dynamically adjusting installable loader buttons
    pub details: Box<VersionDetails>,

    pub content_watcher: ContentWatcher,
    pub index_watcher: FsWatcher,
}

pub struct ContentWatcher {
    pub mods: Option<FsWatcher>,
    pub resource_packs: Option<FsWatcher>,
    pub texture_packs: Option<FsWatcher>,
    pub shaders: Option<FsWatcher>,
    pub data_packs: Option<FsWatcher>,
}

impl ContentWatcher {
    pub fn new(dotmc_dir: &Path) -> Self {
        let watch = |dir| FsWatcher::new(dotmc_dir.join(dir)).ok();

        Self {
            mods: watch("mods"),
            resource_packs: watch("resourcepacks"),
            texture_packs: watch("texturepacks"),
            shaders: watch("shaderpacks"),
            data_packs: watch("datapacks"),
        }
    }

    pub fn tick(&self) -> Option<QueryType> {
        if let Some(w) = &self.mods {
            if w.has_changed() {
                if log_verbose() {
                    pt!("Watch: dir changed (mods)");
                }
                return Some(QueryType::Mods);
            }
        }
        if let Some(w) = &self.resource_packs {
            if w.has_changed() {
                if log_verbose() {
                    pt!("Watch: dir changed (resourcepacks)");
                }
                return Some(QueryType::ResourcePacks);
            }
        }
        if let Some(w) = &self.texture_packs {
            if w.has_changed() {
                if log_verbose() {
                    pt!("Watch: dir changed (texturepacks)");
                }
                return Some(QueryType::ResourcePacks);
            }
        }
        if let Some(w) = &self.shaders {
            if w.has_changed() {
                if log_verbose() {
                    pt!("Watch: dir changed (shaderpacks)");
                }
                return Some(QueryType::Shaders);
            }
        }
        if let Some(w) = &self.data_packs {
            if w.has_changed() {
                if log_verbose() {
                    pt!("Watch: dir changed (datapacks)");
                }
                return Some(QueryType::DataPacks);
            }
        }

        None
    }

    #[allow(unused)] // Just for type safety
    pub fn get(&self, query_type: QueryType) -> Option<&FsWatcher> {
        match query_type {
            QueryType::Mods => &self.mods,
            QueryType::Shaders => &self.shaders,
            QueryType::ModPacks => return None,
            QueryType::DataPacks => &self.data_packs,
            QueryType::ResourcePacks => &self.resource_packs,
        }
        .as_ref()
    }
}

#[derive(Debug, Clone)]
pub enum InfoMessageKind {
    Success,
    AtPath(PathBuf),
    Error,
}

#[derive(Debug, Clone)]
pub struct InfoMessage {
    pub text: String,
    pub kind: InfoMessageKind,
}

impl InfoMessage {
    pub fn error(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            kind: InfoMessageKind::Error,
        }
    }

    pub fn success(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            kind: InfoMessageKind::Success,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MenuEditModsModal {
    Submenu,
    RightClick(ModId, (f32, f32)),
    FolderMenu,
    AddFile,
}

impl MenuEditMods {
    pub fn update_locally_installed_mods(
        idx: &ModIndex,
        selected_instance: &Instance,
        project_type: QueryType,
    ) -> Task<Message> {
        let mut blacklist = HashSet::new();
        for mod_info in idx.mods.values() {
            if mod_info.project_type != project_type {
                continue;
            }
            for file in &mod_info.files {
                blacklist.insert(file.filename.clone());
                blacklist.insert(format!("{}.disabled", file.filename));
            }
        }
        Task::perform(
            get_locally_installed_mods(
                selected_instance.get_dot_minecraft_path(),
                blacklist,
                project_type,
            ),
            move |n| ManageModsMessage::LocalFilesLoaded(n, project_type).into(),
        )
    }

    /// Returns two `Vec`s that are:
    /// - The IDs of downloaded mods
    /// - The filenames of local mods
    ///
    /// ...respectively, from the mods selected in the mod menu.
    pub fn get_kinds_of_ids(&self) -> (Vec<ModId>, Vec<LocalMod>) {
        let ids_downloaded = self
            .selection
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        let ids_local = self
            .selection
            .selected_mods
            .iter()
            .filter_map(SelectedMod::local)
            .filter(|n| n.1.is_toggleable())
            .cloned()
            .collect();

        (ids_downloaded, ids_local)
    }

    pub fn update_selected_state(&mut self) {
        self.selection.state = if self.selection.selected_mods.is_empty() {
            SelectedState::None
        } else if self.selection.selected_mods.len() == self.sorted_mods_list.len() {
            SelectedState::All
        } else {
            SelectedState::Some
        };
    }

    pub fn is_selected(&self, clicked_id: &ModId) -> bool {
        self.selection.selected_mods.iter().any(|n| {
            if let SelectedMod::Downloaded { id, .. } = n {
                id == clicked_id
            } else {
                false
            }
        })
    }
}

pub struct MenuExportModsText {
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
    pub list: Result<Option<Vec<ListEntry>>, String>,
    // UI:
    pub kind: InstanceKind,
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

#[cfg(feature = "auto_update")]
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
    pub categories: ModCategoryState,

    pub mod_descriptions: HashMap<ModId, String>,
    pub mods_download_in_progress: HashMap<ModId, (Arc<str>, ModOperation)>,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub scroll_offset: AbsoluteOffset,

    pub version_json: Box<VersionDetails>,
    pub config: InstanceConfigJson,
    pub mod_index: ModIndex,

    pub backend: StoreBackendType,
    pub query_type: QueryType,
    pub force_open_source: bool,

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
            images.queue(&img, false);
        }
    }
}

pub struct ModCategoryState {
    pub categories: Result<Vec<Category>, String>,
    pub selected: HashSet<String>,
    /// Whether to search for mods containing *all*
    /// the categories, instead of just any of them.
    ///
    /// Only works in modrinth, no effect on curseforge
    pub use_all: bool,
}

impl Default for ModCategoryState {
    fn default() -> Self {
        Self {
            categories: Ok(Vec::new()),
            selected: HashSet::new(),
            use_all: true,
        }
    }
}

impl ModCategoryState {
    pub fn reset(&mut self) {
        self.categories = Ok(Vec::new());
        self.selected.clear();
    }

    pub fn toggle(&mut self, slug: &str) {
        if self.selected.contains(slug) {
            self.selected.remove(slug);
        } else {
            self.selected.insert(slug.to_string());
        }
    }
}

pub struct MenuLauncherSettings {
    pub selected_tab: LauncherSettingsTab,

    pub temp_scale: f64,
    pub arg_split_by_space: bool,

    pub outmsg: Option<String>,
    pub outmsg_at: SettingsOutmsg,
}

pub enum SettingsOutmsg {
    Assets,
    Cache,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LauncherSettingsTab {
    #[default]
    UserInterface,
    Presence,
    Launcher,
    Game,
    About,
}

impl std::fmt::Display for LauncherSettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LauncherSettingsTab::UserInterface => "Appearance",
            LauncherSettingsTab::Game => "Game",
            LauncherSettingsTab::Launcher => "Launcher",
            LauncherSettingsTab::About => "About",
            LauncherSettingsTab::Presence => "Discord Presence",
        })
    }
}

impl LauncherSettingsTab {
    pub const ALL: &'static [Self] = &[
        Self::UserInterface,
        Self::Presence,
        Self::Game,
        Self::Launcher,
        Self::About,
    ];

    pub const fn next(self) -> Self {
        match self {
            Self::UserInterface => Self::Presence,
            Self::Presence => Self::Game,
            Self::Game => Self::Launcher,
            Self::Launcher | Self::About => Self::About,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::UserInterface | Self::Presence => Self::UserInterface,
            Self::Game => Self::Presence,
            Self::Launcher => Self::Game,
            Self::About => Self::Launcher,
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
        filters: HashSet<ql_mod_manager::store::recommended::Category>,
        mod_info: HashMap<ModId, SearchMod>,
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
    pub not_allowed: HashSet<CurseforgeNotAllowed>,
    pub delete_mods: bool,
}

pub struct MenuExportInstance {
    pub entries: Option<Vec<(DirItem, bool)>>,
    pub progress: Option<ProgressBar<GenericProgress>>,
    /// Whether the instance is being exported, or cloned
    pub is_exporting: bool,
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

pub struct MenuModDescription {
    pub description: Result<Option<MarkState>, String>,
    pub details: Option<SearchMod>,
    pub mod_id: ModId,
    pub _handle: [iced::task::Handle; 2],
}

/// The enum that represents which menu is opened currently.
pub enum State {
    /// Default home screen
    Launch(MenuLaunch),
    Create(MenuCreateInstance),
    /// Screen to guide new users to the launcher
    Welcome(MenuWelcome),
    ChangeLog,
    #[cfg(feature = "auto_update")]
    UpdateFound(MenuLauncherUpdate),

    EditMods(MenuEditMods),
    ExportModsText(MenuExportModsText),
    EditJarMods(MenuEditJarMods),
    ImportModpack(ProgressBar<GenericProgress>),
    CurseforgeManualDownload(MenuCurseforgeManualDownload),
    /// Exporting or cloning an instance
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
    ModDescription(MenuModDescription),
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
    pub content: widget::text_editor::Content,
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
