use std::{collections::HashSet, path::PathBuf, process::ExitStatus};

use crate::{
    message_handler::ForgeKind,
    state::{LaunchModal, MenuEditModsModal},
    stylesheet::styles::{LauncherThemeColor, LauncherThemeLightness},
};
use filthy_rich::DiscordIPC;
use iced::widget::{self, scrollable::AbsoluteOffset};
use ql_core::{
    file_utils::DirItem,
    jarmod::JarMods,
    json::instance_config::{MainClassMode, PreLaunchPrefixMode},
    read_log::Diagnostic,
    InstanceSelection, LaunchedProcess, ListEntry, Loader, ModId, StoreBackendType,
};
use ql_instances::{
    auth::{
        ms::{AuthCodeResponse, AuthTokenResponse},
        AccountData, AccountType,
    },
    UpdateCheckInfo,
};
use ql_mod_manager::{
    loaders::{fabric, paper::PaperVersion},
    store::{CurseforgeNotAllowed, ImageResult, ModIndex, QueryType, RecommendedMod, SearchResult},
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
    VersionsLoaded(Res<Vec<ql_mod_manager::loaders::paper::PaperVersion>>),
    ButtonClicked,
    ScreenOpen,
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen {
        is_server: bool,
    },
    SidebarResize(f32),

    VersionsLoaded(Res<(Vec<ListEntry>, String)>),
    VersionSelected(ListEntry),
    NameInput(String),
    ChangeAssetToggle(bool),

    SearchInput(String),
    SearchSubmit,
    ContextMenuToggle,
    CategoryToggle(ql_core::ListEntryKind),

    Start,
    End(Res<InstanceSelection>),

    #[allow(unused)]
    Import,
    ImportResult(Res<Option<InstanceSelection>>),
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
    CloseLauncherToggle(bool),
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
    ScreenOpen,
    ScreenOpenWithoutUpdate,

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

    UpdateMods,
    UpdateModsFinished(Res),
    UpdateCheckResult(Res<Vec<(ModId, String)>>),
    UpdateCheckToggle(usize, bool),

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
    SearchInput(String),
    SearchResult(Res<SearchResult>),

    Click(usize),
    BackToMainScreen,
    LoadData(Res<(ModId, String)>),
    Download(usize),
    DownloadComplete(Res<(ModId, HashSet<CurseforgeNotAllowed>)>),
    IndexUpdated(Res<ModIndex>),
    Scrolled(widget::scrollable::Viewport),
    InstallModpack(ModId),
    Uninstall(usize),
    UninstallComplete(Res<Vec<ModId>>),

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
    Open,
    LoadedSystemTheme(Res<dark_light::Mode>),
    ThemePicked(LauncherThemeLightness),
    ColorSchemePicked(LauncherThemeColor),
    UiScale(f64),
    UiScaleApply,
    UiOpacity(f32),
    UiIdleFps(f64),
    ClearJavaInstalls,
    ClearJavaInstallsConfirm,
    ChangeTab(LauncherSettingsTab),
    DefaultMinecraftWidthChanged(String),
    DefaultMinecraftHeightChanged(String),

    ToggleAntialiasing(bool),
    ToggleDiscordRichPresence(bool),
    ToggleWindowSize(bool),
    ToggleInstanceRemembering(bool),
    #[allow(unused)]
    ToggleWindowDecorations(bool),

    GlobalJavaArgs(ListMessage),
    GlobalPreLaunchPrefix(ListMessage),
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

    ManageMods(ManageModsMessage),
    ManageJarMods(ManageJarModsMessage),
    InstallMods(InstallModsMessage),
    InstallOptifine(InstallOptifineMessage),
    InstallFabric(InstallFabricMessage),
    EditPresets(EditPresetsMessage),
    ExportMods(ExportModsMessage),
    RecommendedMods(RecommendedModMessage),

    DiscordIPCClientLaunched(Result<DiscordIPC, String>),

    LaunchInstanceSelected(InstanceSelection),
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchEnd(Res<LaunchedProcess>),
    LaunchKill,
    LaunchGameExited(Res<(ExitStatus, InstanceSelection, Option<Diagnostic>)>),

    MScreenOpen {
        message: Option<String>,
        clear_selection: bool,
        is_server: Option<bool>,
    },
    MChangeTab(LaunchTab),
    MModal(Option<LaunchModal>),

    MSidebarResize(f32),
    MSidebarScroll(f32),

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
    CoreTickConfigSaved(Res),
    CoreListLoaded(Res<(Vec<String>, bool)>),
    CoreOpenChangeLog,
    CoreOpenIntro,
    CoreEvent(iced::Event, iced::event::Status),
    CoreCleanComplete(Res),
    CoreFocusNext,
    CoreTryQuit,

    CoreImageDownloaded(Res<ImageResult>),

    CoreLogToggle,
    CoreLogScroll(isize),
    CoreLogScrollAbsolute(isize),

    #[allow(unused)]
    UpdateCheckResult(Res<UpdateCheckInfo>),
    UpdateDownloadStart,
    #[allow(unused)]
    UpdateDownloadEnd(Res),

    ServerCommandEdit(String),
    ServerCommandSubmit,

    LicenseOpen,
    LicenseChangeTab(LicenseTab),
    LicenseAction(widget::text_editor::Action),
}
