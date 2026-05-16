use crate::{
    config::UiWindowDecorations,
    message_update::MSG_RESIZE,
    state::{AutoSaveKind, Launcher, LauncherSettingsMessage, LauncherSettingsTab, Message, State},
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
                self.go_to_launcher_settings(tab);
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
                    msg2: "They will get reinstalled automatically as needed".to_owned(),
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
                        menu.cleaned_bytes = Some(format_memory_bytes(b));
                    }
                }
                Err(err) => self.set_error(err),
            },
            LauncherSettingsMessage::ClearDownloadCache => {
                self.state = State::ConfirmAction {
                    msg1: "delete cache for downloads".to_owned(),
                    msg2: "Caches will be rebuilt once you start downloading content again"
                        .to_owned(),
                    yes: LauncherSettingsMessage::ClearDownloadCacheConfirm.into(),
                    no: LauncherSettingsMessage::Open(LauncherSettingsTab::Launcher).into(),
                }
            }
            LauncherSettingsMessage::ClearDownloadCacheConfirm => {
                return Task::perform(ql_core::clean::clear_cache_dir(true), |_| {
                    LauncherSettingsMessage::Open(LauncherSettingsTab::Launcher).into()
                });
            }
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
                self.config.do_cache = Some(t);
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

fn format_memory_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * MB;
    const MB: u64 = 1024 * KB;
    const KB: u64 = 1024;

    let b = bytes as f64;

    if bytes >= GB {
        format!("{:.2} GB", b / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", b / MB as f64)
    } else if bytes >= KB {
        format!("{:.?} KB", b / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
