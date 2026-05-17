use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use iced::{Rectangle, Task, widget::text_editor};
use ql_core::{
    Instance, IntoIoError, IntoJsonError, IntoStringError, JsonFileError, constants::OS_NAME,
    json::InstanceConfigJson,
};
use ql_mod_manager::store::{ModConfig, ModId, ModIndex};

use crate::config::SIDEBAR_WIDTH;
use crate::state::{
    AutoSaveKind, EditInstanceMessage, GameProcess, InstallModsMessage, InstanceLog, LaunchModal,
    LaunchTab, Launcher, LogState, ManageJarModsMessage, MenuCreateInstance, MenuEditMods,
    MenuExportInstance, MenuInstallFabric, MenuInstallOptifine, MenuLaunch, MenuLoginMS,
    MenuModsDownload, MenuRecommendedMods, Message, ModListEntry, State,
};

impl Launcher {
    pub fn update_progress(&mut self, progress: Arc<dyn ql_core::Progress>) {
        match &mut self.state {
            State::InstallFabric(menu) => {
                if let MenuInstallFabric::Loaded {
                    progress: Some(progress_bar),
                    ..
                } = menu
                {
                    progress_bar.update(progress);
                }
            }
            State::InstallForge(bar, _)
            | State::AccountLoginProgress(bar)
            | State::ImportModpack(bar)
            | State::ExportInstance(MenuExportInstance {
                progress: Some(bar),
                ..
            }) => bar.update(progress),
            State::InstallOptifine(menu) => {
                if let MenuInstallOptifine::Installing(p) = menu {
                    p.update(progress);
                }
            }
            State::Create(menu) => match menu {
                MenuCreateInstance::Choosing { .. } => {}
                MenuCreateInstance::DownloadingInstance(bar)
                | MenuCreateInstance::ImportingInstance(bar) => {
                    bar.update(progress);
                }
            },
            #[cfg(feature = "auto_update")]
            State::UpdateFound(menu) => {
                if let Some(p) = &mut menu.progress {
                    p.update(progress);
                }
            }
            State::ManagePresets(menu) => {
                if let Some(p) = &mut menu.progress {
                    p.update(progress);
                }
            }
            State::RecommendedMods(menu) => {
                if let MenuRecommendedMods::Loading { progress: bar, .. } = menu {
                    bar.update(progress);
                }
            }

            State::EditMods(menu) => {
                if let Some(bar) = &mut menu.mod_update_progress {
                    if progress.generic().has_finished {
                        menu.mod_update_progress = None;
                    } else {
                        bar.update(progress);
                    }
                }
            }
            State::EditJarMods(_)
            | State::ModsDownload(_)
            | State::LauncherSettings(_)
            | State::Launch(_)
            | State::Error { .. }
            | State::LoginAlternate(_)
            | State::AccountLogin
            | State::ExportInstance(_)
            | State::ConfirmAction { .. }
            | State::ChangeLog
            | State::Welcome(_)
            | State::License(_)
            | State::LoginMS(MenuLoginMS { .. })
            | State::GenericMessage(_)
            | State::CurseforgeManualDownload(_)
            | State::LogUploadResult { .. }
            | State::InstallPaper(_)
            | State::InstallJava(_)
            | State::CreateShortcut(_)
            | State::ExportMods(_)
            | State::ModDescription(_) => {}
        }
    }

    pub fn tick(&mut self) -> Task<Message> {
        self.autosave_launcher_config();

        match &mut self.state {
            State::Launch(_) => {
                let autosave_instancecfg = if let State::Launch(MenuLaunch {
                    tab: LaunchTab::Edit,
                    edit_instance: Some(edit),
                    ..
                }) = &self.state
                    && self.autosave.insert(AutoSaveKind::InstanceConfig)
                    && let Some(instance) = self.selected_instance.clone()
                {
                    // Autosave instance config
                    let config = (*edit.config).clone();
                    Task::perform(Launcher::save_config(instance, config), |n| {
                        EditInstanceMessage::ConfigSaved(n.strerr()).into()
                    })
                } else {
                    Task::none()
                };

                for (instance, process) in &mut self.processes {
                    let log_state = if let State::Launch(menu) = &mut self.state {
                        &mut menu.log_state
                    } else {
                        &mut None
                    };
                    Self::read_game_logs(
                        process,
                        instance,
                        &mut self.logs,
                        log_state,
                        self.selected_instance.as_ref(),
                    );
                }

                let sidebar_scroll = if let State::Launch(menu) = &self.state {
                    self.tick_sidebar_auto_scroll(menu)
                } else {
                    Task::none()
                };

                return Task::batch([autosave_instancecfg, sidebar_scroll]);
            }
            State::EditMods(menu) => {
                let instance_selection = self.selected_instance.as_ref().unwrap();
                let update_locally_installed_mods = menu.tick(instance_selection);
                return update_locally_installed_mods;
            }
            State::ModsDownload(_) => {
                return MenuModsDownload::tick(self.selected_instance.clone().unwrap());
            }
            State::LauncherSettings(_) => {
                let launcher_config = self.config.clone();
                tokio::spawn(async move { launcher_config.save().await });
            }
            State::EditJarMods(menu) => {
                if self.autosave.insert(AutoSaveKind::Jarmods) {
                    let mut jarmods = menu.jarmods.clone();
                    let selected_instance = self.selected_instance.clone().unwrap();
                    return Task::perform(
                        async move { (jarmods.save(&selected_instance).await.strerr(), jarmods) },
                        |n| ManageJarModsMessage::AutosaveFinished(n).into(),
                    );
                }
            }

            _ => {}
        }

        Task::none()
    }

    pub fn tick_interval(&self) -> u64 {
        if let State::Launch(menu) = &self.state
            && let Some(LaunchModal::Dragging { .. }) = &menu.modal
        {
            // Faster tick rate for smoother auto-scrolling
            // while dragging in the sidebar
            return 15;
        }

        self.config.c_idle_fps()
    }

    /// Automatically scrolls the sidebar when dragging near the edges
    fn tick_sidebar_auto_scroll(&self, menu: &MenuLaunch) -> Task<Message> {
        const EDGE_THRESHOLD: f32 = 36.0;
        const MIN_SPEED: f32 = 2.0;
        const MAX_SPEED: f32 = 14.0;
        const FALLBACK_TOP: f32 = 60.0;
        const FALLBACK_BOTTOM: f32 = 80.0;

        let Some(LaunchModal::Dragging { .. }) = menu.modal.as_ref() else {
            return Task::none();
        };

        let scroll = menu.sidebar_scroll;
        if scroll.remaining <= 0.0 {
            return Task::none();
        }

        let bounds = scroll.bounds.unwrap_or_else(|| {
            let (width, height) = self.window_state.size;
            let sidebar_width = width * SIDEBAR_WIDTH;
            let usable_height = (height - FALLBACK_TOP - FALLBACK_BOTTOM).max(0.0);
            Rectangle {
                x: 0.0,
                y: FALLBACK_TOP,
                width: sidebar_width,
                height: usable_height,
            }
        });

        let (mouse_x, mouse_y) = self.window_state.mouse_pos;
        if mouse_x < bounds.x || mouse_x > bounds.x + bounds.width {
            return Task::none();
        }

        let top_dist = mouse_y - bounds.y;
        let bottom_dist = bounds.y + bounds.height - mouse_y;
        let mut delta = 0.0;

        if (0.0..EDGE_THRESHOLD).contains(&top_dist) {
            let strength = 1.0 - (top_dist / EDGE_THRESHOLD);
            let speed = MIN_SPEED + (MAX_SPEED - MIN_SPEED) * strength * strength;
            delta = -speed;
        } else if (0.0..EDGE_THRESHOLD).contains(&bottom_dist) {
            let strength = 1.0 - (bottom_dist / EDGE_THRESHOLD);
            let speed = MIN_SPEED + (MAX_SPEED - MIN_SPEED) * strength * strength;
            delta = speed;
        }

        if delta.abs() < f32::EPSILON {
            return Task::none();
        }

        let new_offset = (scroll.offset + delta).clamp(0.0, scroll.remaining);

        if (new_offset - scroll.offset).abs() < 0.25 {
            return Task::none();
        }

        iced::widget::operation::scroll_to(
            "MenuLaunch:sidebar",
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: new_offset,
            },
        )
    }

    fn autosave_launcher_config(&mut self) {
        if self.autosave.insert(AutoSaveKind::LauncherConfig) {
            let launcher_config = self.config.clone();
            tokio::spawn(async move { launcher_config.save().await });
        }
    }

    pub fn read_game_logs(
        process: &GameProcess,
        instance: &Instance,
        logs: &mut HashMap<Instance, InstanceLog>,
        log_state: &mut Option<LogState>,
        selected_instance: Option<&Instance>,
    ) {
        let update_ui = selected_instance.is_some_and(|n| n == instance);

        while let Some(message) = process.receiver.as_ref().and_then(|n| n.try_recv().ok()) {
            let message = message.to_string();

            logs.entry(instance.clone())
                .or_insert_with(|| {
                    let log_start = format!(
                        "[00:00:00] [launcher/INFO] {} (OS: {OS_NAME})\n",
                        if instance.is_server() {
                            "Starting Minecraft server"
                        } else {
                            "Launching Minecraft"
                        },
                    );

                    if update_ui {
                        *log_state = Some(LogState {
                            content: text_editor::Content::with_text(&log_start),
                        });
                    }
                    InstanceLog {
                        log: vec![log_start],
                        has_crashed: false,
                        command: String::new(),
                    }
                })
                .log
                .push(message.clone());

            if update_ui {
                update_log_render_state(log_state.as_mut(), message);
            }
        }
    }

    async fn save_config(
        instance: Instance,
        config: InstanceConfigJson,
    ) -> Result<(), JsonFileError> {
        let mut config = config.clone();
        if config.enable_logger.is_none() {
            config.enable_logger = Some(true);
        }
        let config_path = instance.get_instance_path().join("config.json");

        let config_json = serde_json::to_string(&config).json_to()?;
        tokio::fs::write(&config_path, config_json)
            .await
            .path(config_path)?;
        Ok(())
    }
}

impl MenuModsDownload {
    fn tick(selected_instance: Instance) -> Task<Message> {
        Task::perform(
            async move { ModIndex::load(&selected_instance).await },
            |n| InstallModsMessage::IndexUpdated(n.strerr()).into(),
        )
    }
}

pub fn sort_dependencies(
    downloaded_mods: &HashMap<ModId, ModConfig>,
    locally_installed_mods: &HashSet<String>,
) -> Vec<ModListEntry> {
    let mut entries: Vec<ModListEntry> = downloaded_mods
        .iter()
        .map(|(id, c)| ModListEntry::Downloaded {
            id: id.clone(),
            config: Box::new(c.clone()),
        })
        .chain(locally_installed_mods.iter().map(|n| ModListEntry::Local {
            file_name: n.clone(),
        }))
        .collect();
    entries.sort_by(|val1, val2| match (val1, val2) {
        (
            ModListEntry::Downloaded { config, .. },
            ModListEntry::Downloaded {
                config: config2, ..
            },
        ) => match (config.manually_installed, config2.manually_installed) {
            (true, true) | (false, false) => config.name.cmp(&config2.name),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        },
        (ModListEntry::Downloaded { config, .. }, ModListEntry::Local { .. }) => {
            if config.manually_installed {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (ModListEntry::Local { .. }, ModListEntry::Downloaded { config, .. }) => {
            if config.manually_installed {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (
            ModListEntry::Local { file_name },
            ModListEntry::Local {
                file_name: file_name2,
            },
        ) => file_name.cmp(file_name2),
    });

    entries
}

impl MenuEditMods {
    fn tick(&mut self, instance_selection: &Instance) -> Task<Message> {
        self.sorted_mods_list = sort_dependencies(&self.mods.mods, &self.locally_installed_mods);

        MenuEditMods::update_locally_installed_mods(&self.mods, instance_selection)
    }
}

fn update_log_render_state(log_state: Option<&mut LogState>, mut message: String) {
    if let Some(state) = log_state {
        use iced::widget::text_editor::{Action, Edit, Motion};
        // TODO: preserve selection
        message = message.replace('\t', "    ");
        let content = &mut state.content;
        content.perform(Action::Move(Motion::DocumentEnd));
        content.perform(Action::Edit(Edit::Paste(Arc::new(message))));
    }
}
