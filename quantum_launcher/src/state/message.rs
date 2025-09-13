use std::{
    collections::HashSet,
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex},
};

use iced::widget;
use ql_core::{
    file_utils::DirItem, jarmod::JarMods, InstanceSelection, ListEntry, ModId, StoreBackendType,
};
use ql_instances::{
    auth::{
        ms::{AuthCodeResponse, AuthTokenResponse},
        AccountData,
    },
    UpdateCheckInfo,
};
use ql_mod_manager::{
    loaders::fabric::FabricVersionListItem,
    store::{CurseforgeNotAllowed, ImageResult, ModIndex, QueryType, RecommendedMod, SearchResult},
};
use tokio::process::Child;

use super::{LaunchTabId, LauncherSettingsTab, LicenseTab, Res};

#[derive(Debug, Clone)]
pub enum InstallFabricMessage {
    End(Res),
    VersionSelected(String),
    VersionsLoaded(Res<Vec<FabricVersionListItem>>),
    ButtonClicked,
    ScreenOpen { is_quilt: bool },
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen,

    VersionsLoaded(Res<Vec<ListEntry>>),
    VersionSelected(ListEntry),
    NameInput(String),
    ChangeAssetToggle(bool),

    Start,
    End(Res<String>),
    Cancel,

    #[allow(unused)]
    Import,
    ImportResult(Res<Option<InstanceSelection>>),
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    ConfigSaved(Res),
    JavaOverride(String),
    MemoryChanged(f32),
    LoggingToggle(bool),
    CloseLauncherToggle(bool),
    JavaArgsAdd,
    JavaArgEdit(String, usize),
    JavaArgDelete(usize),
    JavaArgShiftUp(usize),
    JavaArgShiftDown(usize),
    JavaArgsModeChanged(ql_core::json::instance_config::JavaArgsMode),
    GameArgsAdd,
    GameArgEdit(String, usize),
    GameArgDelete(usize),
    GameArgShiftUp(usize),
    GameArgShiftDown(usize),
    PreLaunchPrefixAdd,
    PreLaunchPrefixEdit(String, usize),
    PreLaunchPrefixDelete(usize),
    PreLaunchPrefixShiftUp(usize),
    PreLaunchPrefixShiftDown(usize),
    PreLaunchPrefixModeChanged(ql_core::json::instance_config::PreLaunchPrefixMode),
    RenameEdit(String),
    RenameApply,
    WindowWidthChanged(String),
    WindowHeightChanged(String),

    CustomJarPathChanged(String),
    CustomJarLoaded(Res<Vec<String>>),
    AutoSetMainClassToggle(bool),
}

#[derive(Debug, Clone)]
pub enum ManageModsMessage {
    ScreenOpen,
    ScreenOpenWithoutUpdate,

    ToggleCheckbox(String, Option<ModId>),

    DeleteSelected,
    DeleteFinished(Res<Vec<ModId>>),
    LocalDeleteFinished(Res),
    LocalIndexLoaded(HashSet<String>),

    ToggleSelected,
    ToggleFinished(Res),

    UpdateMods,
    UpdateModsFinished(Res),
    UpdateCheckResult(Res<Vec<(ModId, String)>>),
    UpdateCheckToggle(usize, bool),

    SelectAll,
    AddFile(bool),
    AddFileDone(Res<HashSet<CurseforgeNotAllowed>>),
    ExportMenuOpen,
    ToggleSubmenu1,

    CurseforgeManualToggleDelete(bool),
}

#[derive(Debug, Clone)]
pub enum ExportModsMessage {
    ExportAsPlainText,
    ExportAsMarkdown,
    CopyMarkdownToClipboard,
    CopyPlainTextToClipboard,
}

#[derive(Debug, Clone)]
pub enum ManageJarModsMessage {
    Open,
    Loaded(Res<JarMods>),
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
    SearchResult(Res<SearchResult>),
    Open,
    SearchInput(String),
    ImageDownloaded(Res<ImageResult>),
    Click(usize),
    BackToMainScreen,
    LoadData(Res<(ModId, String)>),
    Download(usize),
    DownloadComplete(Res<(ModId, HashSet<CurseforgeNotAllowed>)>),
    IndexUpdated(Res<ModIndex>),
    Scrolled(widget::scrollable::Viewport),
    InstallModpack(ModId),

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

// FIXME: Look at the unused messages
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

    OpenMicrosoft {
        is_from_welcome_screen: bool,
    },
    OpenElyBy {
        is_from_welcome_screen: bool,
    },

    OpenLittleSkin {
        is_from_welcome_screen: bool,
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
    ThemePicked(String),
    ColorSchemePicked(String),
    UiScale(f64),
    UiScaleApply,
    ClearJavaInstalls,
    ClearJavaInstallsConfirm,
    ChangeTab(LauncherSettingsTab),
    DefaultMinecraftWidthChanged(String),
    DefaultMinecraftHeightChanged(String),

    ToggleAntialiasing(bool),
    ToggleWindowSize(bool),

    // Global Java arguments
    GlobalJavaArgsAdd,
    GlobalJavaArgEdit(String, usize),
    GlobalJavaArgDelete(usize),
    GlobalJavaArgShiftUp(usize),
    GlobalJavaArgShiftDown(usize),

    // Global pre-launch prefix
    GlobalPreLaunchPrefixAdd,
    GlobalPreLaunchPrefixEdit(String, usize),
    GlobalPreLaunchPrefixDelete(usize),
    GlobalPreLaunchPrefixShiftUp(usize),
    GlobalPreLaunchPrefixShiftDown(usize),
}

#[derive(Debug, Clone)]
pub enum Message {
    Nothing,
    #[allow(unused)]
    Multiple(Vec<Message>),

    WelcomeContinueToTheme,
    WelcomeContinueToAuth,

    Account(AccountMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    ManageMods(ManageModsMessage),
    ExportMods(ExportModsMessage),
    ManageJarMods(ManageJarModsMessage),
    InstallMods(InstallModsMessage),
    InstallOptifine(InstallOptifineMessage),
    InstallFabric(InstallFabricMessage),
    EditPresets(EditPresetsMessage),
    LauncherSettings(LauncherSettingsMessage),
    RecommendedMods(RecommendedModMessage),

    LaunchInstanceSelected {
        name: String,
        is_server: bool,
    },
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchScreenOpen {
        message: Option<String>,
        clear_selection: bool,
    },
    LaunchEnd(Res<Arc<Mutex<Child>>>),
    LaunchKill,
    LaunchKillEnd(Res),
    LaunchChangeTab(LaunchTabId),

    LaunchScrollSidebar(f32),

    DeleteInstanceMenu,
    DeleteInstance,

    InstallForgeStart {
        is_neoforge: bool,
    },
    InstallForgeEnd(Res),
    InstallPaperStart,
    InstallPaperEnd(Res),

    UninstallLoaderConfirm(Box<Message>, String),
    UninstallLoaderFabricStart,
    UninstallLoaderForgeStart,
    UninstallLoaderOptiFineStart,
    UninstallLoaderPaperStart,
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

    CoreLogToggle,
    CoreLogScroll(isize),
    CoreLogScrollAbsolute(isize),

    LaunchLogScroll(isize),
    LaunchLogScrollAbsolute(isize),
    LaunchEndedLog(Res<(ExitStatus, String)>),
    LaunchCopyLog,
    LaunchUploadLog,
    LaunchUploadLogResult(Res<String>),

    UpdateCheckResult(Res<UpdateCheckInfo>),
    UpdateDownloadStart,
    UpdateDownloadEnd(Res),

    ServerManageOpen {
        selected_server: Option<String>,
        message: Option<String>,
    },
    ServerManageStartServer(String),
    ServerManageStartServerFinish(Res<(Arc<Mutex<Child>>, bool)>),
    ServerManageEndedLog(Res<(ExitStatus, String)>),
    ServerManageKillServer(String),
    ServerManageEditCommand(String, String),
    ServerManageSubmitCommand(String),

    ServerCreateScreenOpen,
    ServerCreateVersionsLoaded(Res<Vec<ListEntry>>),
    ServerCreateNameInput(String),
    ServerCreateVersionSelected(ListEntry),
    ServerCreateStart,
    ServerCreateEnd(Res<String>),

    LicenseOpen,
    LicenseChangeTab(LicenseTab),
    LicenseAction(widget::text_editor::Action),
}
