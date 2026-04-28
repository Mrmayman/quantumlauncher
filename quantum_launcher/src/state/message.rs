use std::{collections::HashSet, path::PathBuf, process::ExitStatus};

use crate::{
    config::{
        discord_rpc::{PresenceStatusDisplayType, RpcText},
        sidebar::{FolderId, SDragLocation, SidebarSelection},
    },
    message_handler::ForgeKind,
    state::{InfoMessage, LaunchModal, MenuEditModsModal, SidebarScroll},
    stylesheet::styles::{LauncherThemeColor, LauncherThemeLightness},
};
use filthy_rich::PresenceClient;
use iced::widget::{self, scrollable::AbsoluteOffset};
use ql_core::{
    Instance, InstanceKind, LaunchedProcess, ListEntry, Loader,
    file_utils::DirItem,
    jarmod::JarMods,
    json::instance_config::{MainClassMode, PreLaunchPrefixMode},
    read_log::Diagnostic,
};
use ql_instances::auth::{
    AccountData, AccountType,
    ms::{AuthCodeResponse, AuthTokenResponse},
};
use ql_mod_manager::{
    loaders::{fabric, paper::PaperVersion},
    store::{
        Category, CurseforgeNotAllowed, ModId, ModIndex, QueryType, RecommendedMod, SearchMod,
        SearchResult, StoreBackendType,
    },
};

use super::{LaunchTab, LauncherSettingsTab, LicenseTab, Res};

#[derive(Debug, Clone)]
pub enum InstallFabricMessage {
    End(Res),
    VersionSelected(String),
    VersionsLoaded(Res<fabric::FabricVersionList>),
    ButtonClicked,
    ScreenOpen { is_quilt: bool },
    ChangeBackend(fabric::BackendType),
}

#[derive(Debug, Clone)]
pub enum InstallPaperMessage {
    End(Res),
    VersionSelected(PaperVersion),
    VersionsLoaded(Res<Vec<PaperVersion>>),
    ButtonClicked,
    ScreenOpen,
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen(InstanceKind),
    SidebarResize(f32),

    VersionsLoaded(Res<(Vec<ListEntry>, String)>),
    VersionSelected(ListEntry),
    NameInput(String),
    ChangeAssetToggle(bool),
    ChangeKind(InstanceKind),

    SearchInput(String),
    SearchSubmit,
    ContextMenuToggle,
    CategoryToggle(ql_core::ListEntryKind),

    Start,
    End(Res<Instance>),

    #[allow(unused)]
    Import,
    ImportResult(Res<Option<Instance>>),
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    ConfigSaved(Res),
    ReinstallLibraries,
    UpdateAssets,
    BrowseJavaOverride,

    JavaOverride(String),
    JavaOverrideVersion(usize),
    MemoryChanged(f32),
    MemoryInputChanged(String),
    LoggingToggle(bool),
    SetMainClass(Option<MainClassMode>, Option<String>),

    JavaArgs(ListMessage),
    JavaArgsModeChanged(bool),
    GameArgs(ListMessage),
    ToggleSplitArg(bool),

    PreLaunchPrefix(ListMessage),
    PreLaunchPrefixModeChanged(PreLaunchPrefixMode),

    RenameEdit(String),
    RenameApply,
    RenameToggle,

    WindowWidthChanged(String),
    WindowHeightChanged(String),

    CustomJarPathChanged(String),
    CustomJarLoaded(Res<Vec<String>>),
}

#[derive(Debug, Clone)]
pub enum ManageModsMessage {
    Open,
    ListScrolled(AbsoluteOffset),
    /// Simple, dumb selection
    SelectEnsure(String, Option<ModId>),
    /// More nuanced selection with ctrl/shift multi-select
    SelectMod(String, Option<ModId>),

    DeleteSelected,
    DeleteOptiforge(String),
    DeleteFinished(Res<Vec<ModId>>),
    LocalDeleteFinished(Res),
    LocalIndexLoaded(HashSet<String>),

    ToggleSelected,
    ToggleFinished(Res),
    ToggleOne(ModId),

    UpdateCheck,
    UpdateCheckResult(Res<Vec<(ModId, String)>>),
    UpdateCheckToggle(usize, bool),
    UpdatePerform,
    UpdatePerformDone(Res<(Option<ql_mod_manager::store::ChangelogFile>, bool)>),
    SetInfoMessage(Option<InfoMessage>),

    /// Add a mod, preset or modpack to the current instance.
    /// The field represents whether to delete the file after importing it.
    AddFile(bool),
    AddFileDone(Res<HashSet<CurseforgeNotAllowed>>),

    SelectAll,
    SetModal(Option<MenuEditModsModal>),
    RightClick(ModId),
    SetSearch(Option<String>),

    ExportMenuOpen,
    CurseforgeManualToggleDelete(bool),
}

#[derive(Debug, Clone, Copy)]
pub enum ExportModsMessage {
    ExportAsPlainText,
    ExportAsMarkdown,
    CopyMarkdownToClipboard,
    CopyPlainTextToClipboard,
}

#[derive(Debug, Clone)]
pub enum ManageJarModsMessage {
    Open,
    ToggleCheckbox(String, bool),
    DeleteSelected,
    AddFile,
    ToggleSelected,
    SelectAll,
    AutosaveFinished((Res, JarMods)),
    MoveUp,
    MoveDown,
}

#[derive(Debug, Clone)]
pub enum InstallModsMessage {
    Open,
    TickDesc(frostmark::UpdateMsg),

    BackToMainScreen,
    Click(usize),
    LoadedDescription(Res<(ModId, String)>),
    LoadedExtendedInfo(Res<(ModId, SearchMod)>),
    IndexUpdated(Res<ModIndex>),
    Scrolled(widget::scrollable::Viewport),

    SearchInput(String),
    SearchResult(Res<SearchResult>),
    Download(usize),
    DownloadComplete(Res<(ModId, HashSet<CurseforgeNotAllowed>)>),
    InstallModpack(ModId),
    Uninstall(usize),
    UninstallComplete(Res<Vec<ModId>>),

    CategoriesLoaded(Res<Vec<Category>>),
    CategoriesToggle(String),
    CategoriesUseAll(bool),

    ForceOpenSource(bool),
    ChangeBackend(StoreBackendType),
    ChangeQueryType(QueryType),
}

#[derive(Debug, Clone)]
pub enum InstallOptifineMessage {
    ScreenOpen,
    SelectInstallerStart,
    DeleteInstallerToggle(bool),
    End(Res),
}

#[derive(Debug, Clone)]
pub enum EditPresetsMessage {
    Open,
    ToggleCheckbox((String, ModId), bool),
    ToggleCheckboxLocal(String, bool),
    ToggleIncludeConfig(bool),
    SelectAll,
    BuildYourOwn,
    BuildYourOwnEnd(Res<Vec<u8>>),
    LoadComplete(Res<HashSet<CurseforgeNotAllowed>>),
}

#[derive(Debug, Clone)]
pub enum RecommendedModMessage {
    Open,
    ModCheckResult(Res<Vec<RecommendedMod>>),
    Toggle(usize, bool),
    Download,
    DownloadEnd(Res<HashSet<CurseforgeNotAllowed>>),
}

#[derive(Debug, Clone)]
pub enum WindowMessage {
    Dragged,
    // HOOK: Decorations
    // Resized(iced::window::Direction),
    ClickClose,
    ClickMinimize,
    ClickMaximize,
    // IsMaximized(bool),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum AccountMessage {
    Selected(String),
    Response1 {
        r: Res<AuthCodeResponse>,
        is_from_welcome_screen: bool,
    },
    Response2(Res<AuthTokenResponse>),
    Response3(Res<AccountData>),
    LogoutCheck,
    LogoutConfirm,
    RefreshComplete(Res<AccountData>),

    OpenMenu {
        is_from_welcome_screen: bool,
        kind: AccountType,
    },

    AltUsernameInput(String),
    AltPasswordInput(String),
    AltOtpInput(String),
    AltShowPassword(bool),
    AltLogin,
    AltLoginResponse(Res<ql_instances::auth::yggdrasil::Account>),

    LittleSkinOauthButtonClicked,
    LittleSkinDeviceCodeReady {
        user_code: String,
        verification_uri: String,
        expires_in: u64,
        interval: u64,
        device_code: String,
    },
    LittleSkinDeviceCodeError(String),
}

#[derive(Debug, Clone)]
pub enum LauncherSettingsMessage {
    Open(LauncherSettingsTab),
    LoadedSystemTheme(Res<dark_light::Mode>),
    ThemePicked(LauncherThemeLightness),
    ColorSchemePicked(LauncherThemeColor),
    UiScale(f64),
    UiScaleApply,
    UiOpacity(f32),
    UiIdleFps(f64),
    ClearJavaInstalls,
    ClearJavaInstallsConfirm,
    DefaultMinecraftWidthChanged(String),
    DefaultMinecraftHeightChanged(String),
    Rpc(RpcMessage),

    ToggleAntialiasing(bool),
    ToggleWindowSize(bool),
    ToggleInstanceRemembering(bool),
    ToggleModUpdateChangelog(bool),
    AfterLaunchBehaviorChanged(crate::config::AfterLaunchBehavior),
    #[allow(unused)]
    ToggleWindowDecorations(bool),

    GlobalJavaArgs(ListMessage),
    GlobalPreLaunchPrefix(ListMessage),
}

#[derive(Debug, Clone)]
pub enum RpcMessage {
    RunStarted(Option<PresenceClient>),
    Toggle(bool),
    DefaultChanged(RpcInnerMessage),
    TogglePresenceOnGameEvent(bool),
    StatusDisplayTypePicked(PresenceStatusDisplayType),
    SetName(String),
    ToggleCompeting(bool),
    GameOpen(RpcInnerMessage),
    GameExit(RpcInnerMessage),
    SetPresenceNow,
    ResetPresence,
}

#[derive(Debug, Clone)]
pub enum RpcInnerMessage {
    TopTextChanged(String),
    TopTextURLChanged(String),
    BottomTextChanged(String),
    BottomTextURLChanged(String),
}

impl RpcText {
    pub fn apply(&mut self, msg: RpcInnerMessage) {
        match msg {
            RpcInnerMessage::TopTextChanged(text) => {
                self.top_text = (!text.is_empty()).then_some(text);
            }
            RpcInnerMessage::TopTextURLChanged(text) => {
                self.top_text_url = (!text.is_empty()).then_some(text);
            }
            RpcInnerMessage::BottomTextChanged(text) => {
                self.bottom_text = (!text.is_empty()).then_some(text);
            }
            RpcInnerMessage::BottomTextURLChanged(text) => {
                self.bottom_text_url = (!text.is_empty()).then_some(text);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ListMessage {
    Add,
    Edit(String, usize),
    Delete(usize),
    ShiftUp(usize),
    ShiftDown(usize),
}

impl ListMessage {
    pub fn apply(self, l: &mut Vec<String>, split: bool) {
        match self {
            ListMessage::Add => {
                l.push(String::new());
            }
            ListMessage::Edit(msg, idx) => {
                if split && msg.contains(' ') {
                    l.remove(idx);
                    let mut insert_idx = idx;
                    for s in msg.split(' ').filter(|n| !n.is_empty()) {
                        l.insert(insert_idx, s.to_owned());
                        insert_idx += 1;
                    }
                } else if let Some(entry) = l.get_mut(idx) {
                    *entry = msg;
                }
            }
            ListMessage::Delete(i) => {
                if i < l.len() {
                    l.remove(i);
                }
            }
            ListMessage::ShiftUp(idx) => {
                if idx > 0 {
                    l.swap(idx, idx - 1);
                }
            }
            ListMessage::ShiftDown(idx) => {
                if idx + 1 < l.len() {
                    l.swap(idx, idx + 1);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotesMessage {
    Loaded(Res<String>),
    OpenEdit,
    Edit(widget::text_editor::Action),
    SaveEdit,
    CancelEdit,
}

#[derive(Debug, Clone)]
pub enum GameLogMessage {
    Action(widget::text_editor::Action),
    Copy,
    Upload,
    Uploaded(Res<String>),
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    Resize(f32),
    Scroll(SidebarScroll),
    FolderRenameConfirm,

    NewFolder(Option<SidebarSelection>),
    DeleteFolder(FolderId),
    ToggleFolderVisibility(FolderId),
    DragDrop(Option<SDragLocation>),
    DragHover {
        location: SDragLocation,
        entered: bool,
    },
}

#[derive(Debug, Clone)]
pub enum MainMenuMessage {
    ChangeTab(LaunchTab),
    Modal(Option<LaunchModal>),
    InstanceSelected(Instance),
    UsernameSet(String),
    SetInfoMessage(Option<InfoMessage>),
}

#[derive(Debug, Clone)]
pub enum ShortcutMessage {
    Open,
    OpenFolder,
    ToggleAddToMenu(bool),
    ToggleAddToDesktop(bool),
    EditName(String),
    EditDescription(String),

    AccountSelected(String),
    AccountOffline(String),

    SaveCustom,
    SaveCustomPicked(PathBuf),
    SaveMenu,
    Done(Res),
}

#[derive(Debug, Clone)]
pub enum ModDescriptionMessage {
    Open(ModId),
    LoadedDetails(Res<SearchMod>),
    LoadedDescription(Res<String>),
}

#[derive(Debug, Clone)]
pub enum Message {
    Nothing,
    Error(String),
    Multiple(Vec<Message>),
    ShowScreen(String),

    WelcomeContinueToTheme,
    WelcomeContinueToAuth,

    Account(AccountMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    LauncherSettings(LauncherSettingsMessage),
    Notes(NotesMessage),
    GameLog(GameLogMessage),
    Window(WindowMessage),
    Shortcut(ShortcutMessage),

    ManageMods(ManageModsMessage),
    ManageJarMods(ManageJarModsMessage),
    InstallMods(InstallModsMessage),
    InstallOptifine(InstallOptifineMessage),
    InstallFabric(InstallFabricMessage),
    EditPresets(EditPresetsMessage),
    ExportMods(ExportModsMessage),
    RecommendedMods(RecommendedModMessage),
    MainMenu(MainMenuMessage),
    Sidebar(SidebarMessage),
    ModDescription(ModDescriptionMessage),

    MScreenOpen {
        message: Option<InfoMessage>,
        clear_selection: bool,
    },
    LaunchStart,
    LaunchEnd(Res<LaunchedProcess>),
    LaunchKill,
    LaunchGameExited(Res<(ExitStatus, Instance, Option<Diagnostic>)>),

    DeleteInstanceMenu,
    DeleteInstance,

    InstallForge(ForgeKind),
    InstallForgeEnd(Res),
    InstallPaper(InstallPaperMessage),

    UninstallLoaderConfirm(Box<Message>, Loader),
    UninstallLoaderStart,
    UninstallLoaderEnd(Res),

    #[allow(unused)]
    ExportInstanceOpen,
    ExportInstanceToggleItem(usize, bool),
    ExportInstanceStart,
    ExportInstanceFinished(Res<Vec<u8>>),
    ExportInstanceLoaded(Res<Vec<DirItem>>),

    CoreCopyError,
    CoreCopyLog,
    CoreOpenLink(String),
    CoreOpenPath(PathBuf),
    CoreCopyText(String),
    CoreTick,
    CoreListLoaded(Res<(Vec<String>, InstanceKind)>),
    CoreOpenChangeLog,
    CoreOpenIntro,
    CoreEvent(iced::Event, iced::event::Status),
    CoreCleanComplete(Res),
    CoreFocusNext,
    CoreTryQuit,
    CoreHideModal,

    CoreImageDownloaded(Res<ql_mod_manager::store::image::Output>),

    CoreLogToggle,
    CoreLogScroll(isize),
    CoreLogScrollAbsolute(isize),

    #[cfg(feature = "auto_update")]
    UpdateCheckResult(Res<crate::launcher_update::UpdateCheckInfo>),
    #[cfg(feature = "auto_update")]
    UpdateDownloadStart,
    #[cfg(feature = "auto_update")]
    UpdateDownloadEnd(Res),

    ServerCommandEdit(String),
    ServerCommandSubmit,

    LicenseOpen,
    LicenseChangeTab(LicenseTab),
    LicenseAction(widget::text_editor::Action),
}

macro_rules! from_m {
    ($field:ident, $t:ty) => {
        impl From<$t> for Message {
            fn from(value: $t) -> Self {
                Message::$field(value)
            }
        }
    };
}

from_m!(MainMenu, MainMenuMessage);
from_m!(Sidebar, SidebarMessage);
from_m!(ManageMods, ManageModsMessage);
from_m!(ManageJarMods, ManageJarModsMessage);
from_m!(InstallMods, InstallModsMessage);
from_m!(InstallOptifine, InstallOptifineMessage);
from_m!(InstallFabric, InstallFabricMessage);
from_m!(EditPresets, EditPresetsMessage);
from_m!(ExportMods, ExportModsMessage);
from_m!(RecommendedMods, RecommendedModMessage);
from_m!(Account, AccountMessage);
from_m!(CreateInstance, CreateInstanceMessage);
from_m!(EditInstance, EditInstanceMessage);
from_m!(LauncherSettings, LauncherSettingsMessage);
from_m!(Notes, NotesMessage);
from_m!(GameLog, GameLogMessage);
from_m!(Window, WindowMessage);
from_m!(Shortcut, ShortcutMessage);
from_m!(ModDescription, ModDescriptionMessage);

impl From<RpcMessage> for LauncherSettingsMessage {
    fn from(value: RpcMessage) -> Self {
        Self::Rpc(value)
    }
}

impl From<RpcMessage> for Message {
    fn from(value: RpcMessage) -> Self {
        Self::LauncherSettings(value.into())
    }
}
