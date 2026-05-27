use std::collections::HashSet;

use crate::{
    config::UiWindowDecorations,
    message_update::MSG_RESIZE,
    state::{
        AutoSaveKind, Launcher, LauncherSettingsMessage, LauncherSettingsTab, Message, PathKind,
        State,
    },
};
use iced::Task;
use ql_core::{IntoStringError, err};

impl Launcher {
    pub fn update_launcher_settings(&mut self, msg: LauncherSettingsMessage) -> Task<Message> {
        match msg {
            LauncherSettingsMessage::ThemePicked(theme) => {
                self.config.ui_mode = Some(theme);
                self.theme.lightness = theme;
            }
            LauncherSettingsMessage::Open(tab) => {
                return self.go_to_launcher_settings(tab);
            }
            LauncherSettingsMessage::ColorSchemePicked(color) => {
                self.config.ui_theme = Some(color);
                self.theme.color = color;
            }
            LauncherSettingsMessage::UiScale(scale) => {
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.temp_scale = scale;
                }
            }
            LauncherSettingsMessage::UiOpacity(opacity) => {
                self.config.ui.get_or_insert_default().window_opacity = opacity;
                self.theme.alpha = opacity;
            }
            LauncherSettingsMessage::UiScaleApply => {
                if let State::LauncherSettings(menu) = &self.state {
                    self.config.ui_scale = Some(menu.temp_scale);
                    self.state = State::GenericMessage(MSG_RESIZE.to_owned());
                }
            }
            LauncherSettingsMessage::UiIdleFps(fps) => {
                debug_assert!(fps > 0.0);
                self.config.ui.get_or_insert_default().idle_fps = Some(fps as u64);
            }
            LauncherSettingsMessage::ClearJavaInstalls => {
                self.state = State::ConfirmAction {
                    msg1: "delete auto-installed Java files".to_owned(),
                    msg2: "They will get reinstalled automatically as needed.\nNote: This does take a while to redownload.".to_owned(),
                    yes: LauncherSettingsMessage::ClearJavaInstallsConfirm.into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Launcher).into(),
                };
            }

            LauncherSettingsMessage::ClearJavaInstallsConfirm => {
                return Task::perform(ql_instances::delete_java_installs(), |()| {
                    LauncherSettingsMessage::Open(LauncherSettingsTab::Launcher).into()
                });
            }
            LauncherSettingsMessage::CleanAssets => {
                return Task::perform(ql_core::clean::assets_dir(), |r| {
                    LauncherSettingsMessage::CleanAssetsFinished(r.strerr()).into()
                });
            }
            LauncherSettingsMessage::CleanAssetsFinished(r) => match r {
                Ok(b) => {
                    if let State::LauncherSettings(menu) = &mut self.state {
                        menu.outmsg = Some(super::format_memory_bytes(b));
                        menu.outmsg_at = crate::state::SettingsOutmsg::Assets;
                    }
                }
                Err(err) => self.set_error(err),
            },
            LauncherSettingsMessage::ClearDownloadCache => {
                return Task::perform(ql_core::clean::clear_cache_dir(), |r| {
                    LauncherSettingsMessage::ClearDownloadCacheDone(r.strerr()).into()
                });
            }
            LauncherSettingsMessage::ClearDownloadCacheDone(res) => match res {
                Ok(b) => {
                    if let State::LauncherSettings(menu) = &mut self.state {
                        menu.outmsg = Some(super::format_memory_bytes(b));
                        menu.outmsg_at = crate::state::SettingsOutmsg::Cache;
                    }
                }
                Err(err) => self.set_error(err),
            },
            LauncherSettingsMessage::ToggleAntialiasing(t) => {
                self.config.ui_antialiasing = Some(t);
            }
            LauncherSettingsMessage::ToggleWindowSize(t) => {
                self.config.c_window().save_window_size = t;
            }
            LauncherSettingsMessage::ToggleInstanceRemembering(t) => {
                let persistent = self.config.c_persistent();
                persistent.selected_remembered = t;
                if !t {
                    persistent.selected_instance = None;
                    persistent.selected_instance_kind = None;
                }
            }
            LauncherSettingsMessage::ToggleModUpdateChangelog(t) => {
                self.config.c_persistent().write_mod_update_changelog = t;
            }
            LauncherSettingsMessage::ToggleCaching(t) => {
                self.config.do_cache = t;
            }
            LauncherSettingsMessage::AfterLaunchBehaviorChanged(behavior) => {
                self.config.ui.get_or_insert_default().after_game_opens = behavior;
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
            }
            LauncherSettingsMessage::DefaultMinecraftWidthChanged(input) => {
                self.config.c_global().window_width = input.trim().parse::<u32>().ok();
            }
            LauncherSettingsMessage::DefaultMinecraftHeightChanged(input) => {
                self.config.c_global().window_height = input.trim().parse::<u32>().ok();
            }
            LauncherSettingsMessage::GlobalJavaArgs(msg) => {
                let split = self.should_split_args();
                msg.apply(self.config.extra_java_args.get_or_insert_default(), split);
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefix(msg) => {
                let split = self.should_split_args();
                msg.apply(
                    self.config
                        .c_global()
                        .pre_launch_prefix
                        .get_or_insert_default(),
                    split,
                );
            }
            LauncherSettingsMessage::ToggleWindowDecorations(b) => {
                let decor = if b {
                    UiWindowDecorations::default()
                } else {
                    UiWindowDecorations::System
                };
                self.config.ui.get_or_insert_default().window_decorations = decor;
            }
            LauncherSettingsMessage::ApplyRestart => {
                if let Ok(runtime) = tokio::runtime::Runtime::new() {
                    _ = runtime.block_on(self.config.save());
                }
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.close_launcher();
            }
            LauncherSettingsMessage::ToggleSafeMode(t) => {
                if t {
                    self.config.enable_safe_mode = Some(true);
                    self.autosave.remove(&AutoSaveKind::LauncherConfig);
                } else {
                    self.state = State::ConfirmAction {
                        msg1: "disable automatic Safe Mode".to_owned(),
                        msg2: "Disabling Safe Mode might cause the launcher to CRASH ON STARTUP. This is highly discouraged.\n\nAre you sure you know what you are doing?".to_owned(),
                        yes: LauncherSettingsMessage::ToggleSafeModeConfirm(false).into(),
                        no: LauncherSettingsMessage::ToggleSafeModeConfirm(true).into(),
                    };
                }
            }
            LauncherSettingsMessage::ToggleSafeModeConfirm(t) => {
                self.config.enable_safe_mode = Some(t);
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
                return self.go_to_launcher_settings(LauncherSettingsTab::UserInterface);
            }
            LauncherSettingsMessage::EnablePortableMode => {
                let (path, flags, current_status) =
                    if let State::LauncherSettings(menu) = &self.state {
                        (
                            menu.temp_paths.portable.clone(),
                            menu.temp_paths.portable_flags.clone(),
                            menu.portable_mode_status.portable.clone(),
                        )
                    } else {
                        (String::new(), HashSet::new(), None)
                    };

                let (msg1, msg2) = if let Some(current) = current_status {
                    let current_path = current
                        .path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if path != current_path {
                        (
                            "change portable storage path".to_owned(),
                            "The launcher will change its portable storage directory.\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.".to_owned(),
                        )
                    } else {
                        (
                            "change graphics backend".to_owned(),
                            "The launcher will restart to apply the new rendering backend."
                                .to_owned(),
                        )
                    }
                } else {
                    (
                        "enable portable mode".to_owned(),
                        "The launcher will store data next to the executable instead.\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.".to_owned(),
                    )
                };

                self.state = State::ConfirmAction {
                    msg1,
                    msg2,
                    yes: LauncherSettingsMessage::EnablePortableModeConfirm(path, flags).into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Location).into(),
                };
            }
            LauncherSettingsMessage::EnablePortableModeConfirm(path, flags) => {
                if let Err(err) = ql_core::create_portable_file(path, flags) {
                    self.set_error(err.to_string());
                    return Task::none();
                }

                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.close_launcher();
            }
            LauncherSettingsMessage::DisablePortableMode => {
                self.state = State::ConfirmAction {
                    msg1: "disable portable mode".to_owned(),
                    msg2: "The launcher will store data in the system data directory instead.\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.".to_owned(),
                    yes: LauncherSettingsMessage::DisablePortableModeConfirm.into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Location)
                        .into(),
                };
            }
            LauncherSettingsMessage::DisablePortableModeConfirm => {
                if let Err(err) = ql_core::delete_portable_file() {
                    self.set_error(err.to_string());
                    return Task::none();
                }

                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.close_launcher();
            }
            LauncherSettingsMessage::PickPortablePath => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select Portable Data Folder")
                    .pick_folder()
                {
                    return self.update(Message::LauncherSettings(
                        LauncherSettingsMessage::SetTempPath(
                            PathKind::Portable,
                            path.to_string_lossy().into_owned(),
                        ),
                    ));
                }
            }
            LauncherSettingsMessage::PortableModeStatusLoaded(status) => {
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.portable_mode_status = status.clone();

                    if let Some(portable) = &status.portable {
                        menu.temp_paths.portable = portable
                            .path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        menu.temp_paths.portable_flags = portable.flags.clone();
                    } else {
                        menu.temp_paths.portable = String::new();
                        menu.temp_paths.portable_flags = HashSet::new();
                    }

                    if let Some(system_redirect) = &status.system_redirect {
                        menu.temp_paths.system_redirect = system_redirect
                            .path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        menu.temp_paths.system_redirect_flags = system_redirect.flags.clone();
                    } else {
                        menu.temp_paths.system_redirect = String::new();
                        menu.temp_paths.system_redirect_flags = HashSet::new();
                    }
                }
            }
            LauncherSettingsMessage::AppearanceGraphicsBackend(backend) => {
                self.config.launcher_render = Some(backend);
                self.autosave.remove(&AutoSaveKind::LauncherConfig);

                self.state = State::ConfirmAction {
                    msg1: "restart to apply rendering backend".to_owned(),
                    msg2: "The launcher will restart to apply the new rendering backend."
                        .to_owned(),
                    yes: LauncherSettingsMessage::ApplyRestart.into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::UserInterface).into(),
                };
            }
            LauncherSettingsMessage::EnableSystemRedirect => {
                let (mut path, flags, current_status) =
                    if let State::LauncherSettings(menu) = &self.state {
                        (
                            menu.temp_paths.system_redirect.clone(),
                            menu.temp_paths.system_redirect_flags.clone(),
                            menu.portable_mode_status.system_redirect.clone(),
                        )
                    } else {
                        (String::new(), HashSet::new(), None)
                    };

                if path.is_empty() {
                    path = ".".to_owned();
                }

                let (msg1, msg2) = if let Some(current) = current_status {
                    let current_path = current
                        .path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let current_path = if current_path.is_empty() {
                        ".".to_owned()
                    } else {
                        current_path
                    };

                    if path != current_path {
                        (
                            "change system-wide redirection path".to_owned(),
                            format!(
                                "The launcher will change its global redirection directory to: {}\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.",
                                if path == "." {
                                    "system data directory"
                                } else {
                                    &path
                                }
                            ),
                        )
                    } else {
                        (
                            "change graphics backend".to_owned(),
                            "The launcher will restart to apply the new rendering backend."
                                .to_owned(),
                        )
                    }
                } else {
                    (
                        "enable system-wide redirection".to_owned(),
                        format!(
                            "The launcher will store data globally in: {}\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.",
                            if path == "." {
                                "system data directory"
                            } else {
                                &path
                            }
                        ),
                    )
                };

                self.state = State::ConfirmAction {
                    msg1,
                    msg2,
                    yes: LauncherSettingsMessage::EnableSystemRedirectConfirm(path, flags).into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Location).into(),
                };
            }
            LauncherSettingsMessage::EnableSystemRedirectConfirm(path, flags) => {
                if let Err(err) = ql_core::create_system_redirect_file(path, flags) {
                    self.set_error(err.to_string());
                    return Task::none();
                }

                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.close_launcher();
            }
            LauncherSettingsMessage::DisableSystemRedirect => {
                self.state = State::ConfirmAction {
                    msg1: "disable system-wide redirection".to_owned(),
                    msg2: "The launcher will stop reading data globally from the system data directory.\nYour existing data will NOT be moved automatically.\n\nThe launcher will automatically restart.".to_owned(),
                    yes: LauncherSettingsMessage::DisableSystemRedirectConfirm.into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Location)
                        .into(),
                };
            }
            LauncherSettingsMessage::DisableSystemRedirectConfirm => {
                if let Err(err) = ql_core::delete_system_redirect_file() {
                    self.set_error(err.to_string());
                    return Task::none();
                }

                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                self.close_launcher();
            }
            LauncherSettingsMessage::PickSystemRedirectPath => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select System Redirect Folder")
                    .pick_folder()
                {
                    return self.update(Message::LauncherSettings(
                        LauncherSettingsMessage::SetTempPath(
                            PathKind::SystemRedirect,
                            path.to_string_lossy().into_owned(),
                        ),
                    ));
                }
            }
            LauncherSettingsMessage::SetTempPath(kind, path) => {
                if let State::LauncherSettings(menu) = &mut self.state {
                    match kind {
                        PathKind::Portable => menu.temp_paths.portable = path,
                        PathKind::SystemRedirect => menu.temp_paths.system_redirect = path,
                    }
                }
            }
            LauncherSettingsMessage::LoadedSystemTheme(res) => match res {
                Ok(mode) => {
                    self.theme.system_dark_mode = mode == dark_light::Mode::Dark;
                }
                Err(err) if err.contains("Timeout reached") => {
                    // The system is just lagging, nothing we can do
                }
                Err(err) if err.contains("org.freedesktop.portal.Error.NotFound") => {
                    // User is on barebones desktop environment
                    // that doesn't support light/dark mode.
                    // eg: Raspberry Pi OS, LXDE, Openbox, etc
                }
                Err(err) => {
                    err!(no_log, "while loading system theme: {err}");
                }
            },
            LauncherSettingsMessage::Rpc(msg) => return self.update_rpc(msg),
        }
        Task::none()
    }
}
