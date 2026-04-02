use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use iced::{Rectangle, Task, widget::text_editor};
use ql_core::{
    InstanceSelection, IntoIoError, IntoJsonError, IntoStringError, JsonFileError,
    constants::OS_NAME, json::InstanceConfigJson,
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
    pub fn tick(&mut self) -> Task<Message> {
        match &mut self.state {
            State::Launch(_) => {
                if let Some(receiver) = &mut self.java_recv {
                    if receiver.tick() {
                        self.state = State::InstallJava;
                        return Task::none();
                    }
                }

                let mut commands = Vec::new();

                let edit_config = if let State::Launch(MenuLaunch {
                    edit_instance: Some(edit),
                    tab: LaunchTab::Edit,
                    ..
                }) = &self.state
                {
                    Some(edit.config.clone())
                } else {
                    None
                };

                if let Some(config) = edit_config {
                    if self.autosave.insert(AutoSaveKind::InstanceConfig)
                        || self.tick_timer % 5 == 0
                    {
                        self.autosave_instance_config(config, &mut commands);
                    }
                }

                for (name, process) in &mut self.processes {
                    let log_state = if let State::Launch(menu) = &mut self.state {
                        &mut menu.log_state
                    } else {
                        &mut None
                    };
                    Self::read_game_logs(process, name, &mut self.logs, log_state);
                }

                if let State::Launch(menu) = &self.state {
                    self.tick_sidebar_auto_scroll(menu, &mut commands);
                }
                self.autosave_launcher_config();

                return Task::batch(commands);
            }
            State::Create(menu) => {
                menu.tick();
                self.autosave_launcher_config();
            }
            State::EditMods(menu) => {
                let instance_selection = self.selected_instance.as_ref().unwrap();
                let update_locally_installed_mods = menu.tick(instance_selection);
                return update_locally_installed_mods;
            }
            State::InstallFabric(menu) => {
                if let MenuInstallFabric::Loaded {
                    progress: Some(progress),
                    ..
                } = menu
                {
                    progress.tick();
                }
            }
            State::InstallForge(menu) => {
                menu.forge_progress.tick();
                if menu.java_progress.tick() {
                    menu.is_java_getting_installed = true;
                }
            }
            #[cfg(feature = "auto_update")]
            State::UpdateFound(menu) => {
                if let Some(progress) = &mut menu.progress {
                    progress.tick();
                }
            }
            State::InstallJava => {
                let has_finished = if let Some(progress) = &mut self.java_recv {
                    progress.tick();
                    progress.progress.has_finished
                } else {
                    true
                };
                if has_finished {
                    self.java_recv = None;
                    return self.go_to_main_menu_with_message(Some("Installed Java"));
                }
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
            State::InstallOptifine(menu) => match menu {
                MenuInstallOptifine::Choosing { .. } | MenuInstallOptifine::InstallingB173 => {}
                MenuInstallOptifine::Installing {
                    optifine_install_progress,
                    java_install_progress,
                    is_java_being_installed,
                    ..
                } => {
                    optifine_install_progress.tick();
                    if let Some(java_progress) = java_install_progress {
                        if java_progress.tick() {
                            *is_java_being_installed = true;
                        }
                    }
                }
            },
            State::ManagePresets(menu) => {
                if let Some(progress) = &mut menu.progress {
                    progress.tick();
                }
            }
            State::RecommendedMods(menu) => {
                if let MenuRecommendedMods::Loading { progress, .. } = menu {
                    progress.tick();
                }
            }
            State::AccountLoginProgress(progress)
            | State::ImportModpack(progress)
            | State::ExportInstance(MenuExportInstance {
                progress: Some(progress),
                ..
            }) => {
                progress.tick();
            }

            // These menus don't require background ticking
            State::Error { .. }
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
            | State::CreateShortcut(_)
            | State::ModDescription(_)
            | State::ExportMods(_) => {}
        }

        Task::none()
    }

    pub fn tick_interval(&self) -> u64 {
        if let State::Launch(menu) = &self.state {
            if let Some(LaunchModal::SDragging { .. }) = &menu.modal {
                // Faster tick rate for smoother auto-scrolling
                // while dragging in the sidebar
                return 15;
            }
        }

        self.config.c_idle_fps()
    }

    /// Automatically scrolls the sidebar when dragging near the edges
    fn tick_sidebar_auto_scroll(&self, menu: &MenuLaunch, commands: &mut Vec<Task<Message>>) {
        const EDGE_THRESHOLD: f32 = 36.0;
        const MIN_SPEED: f32 = 2.0;
        const MAX_SPEED: f32 = 14.0;
        const FALLBACK_TOP: f32 = 60.0;
        const FALLBACK_BOTTOM: f32 = 80.0;

        let Some(LaunchModal::SDragging { .. }) = menu.modal.as_ref() else {
            return;
        };

        if menu.sidebar_scroll_total <= 0.0 {
            return;
        }

        let bounds = menu.sidebar_scroll_bounds.unwrap_or_else(|| {
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
            return;
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
            return;
        }

        let new_offset = (menu.sidebar_scroll_offset + delta).clamp(0.0, menu.sidebar_scroll_total);

        if (new_offset - menu.sidebar_scroll_offset).abs() < 0.25 {
            return;
        }

        commands.push(iced::widget::scrollable::scroll_to(
            iced::widget::scrollable::Id::new("MenuLaunch:sidebar"),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: new_offset,
            },
        ));
    }

    pub fn autosave_launcher_config(&mut self) {
        if self.autosave.insert(AutoSaveKind::LauncherConfig) {
            let launcher_config = self.config.clone();
            tokio::spawn(async move { launcher_config.save().await });
        }
    }

    fn autosave_instance_config(
        &self,
        config: InstanceConfigJson,
        commands: &mut Vec<Task<Message>>,
    ) {
        let Some(instance) = self.selected_instance.clone() else {
            return;
        };
        let cmd = Task::perform(Launcher::save_config(instance, config), |n| {
            EditInstanceMessage::ConfigSaved(n.strerr()).into()
        });
        commands.push(cmd);
    }

    pub fn read_game_logs(
        process: &GameProcess,
        instance: &InstanceSelection,
        logs: &mut HashMap<InstanceSelection, InstanceLog>,
        log_state: &mut Option<LogState>,
    ) {
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

                    *log_state = Some(LogState {
                        content: text_editor::Content::with_text(&log_start),
                    });
                    InstanceLog {
                        log: vec![log_start],
                        has_crashed: false,
                        command: String::new(),
                    }
                })
                .log
                .push(message.clone());

            update_log_render_state(log_state.as_mut(), message);
        }
    }

    async fn save_config(
        instance: InstanceSelection,
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
    pub fn tick(selected_instance: InstanceSelection) -> Task<Message> {
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
    fn tick(&mut self, instance_selection: &InstanceSelection) -> Task<Message> {
        self.sorted_mods_list = sort_dependencies(&self.mods.mods, &self.locally_installed_mods);

        if let Some(progress) = &mut self.mod_update_progress {
            progress.tick();
            if progress.progress.has_finished {
                self.mod_update_progress = None;
            }
        }

        MenuEditMods::update_locally_installed_mods(&self.mods, instance_selection)
    }
}

impl MenuCreateInstance {
    pub fn tick(&mut self) {
        match self {
            MenuCreateInstance::Choosing { .. } => {}
            MenuCreateInstance::DownloadingInstance(progress) => {
                progress.tick();
            }
            MenuCreateInstance::ImportingInstance(progress) => {
                progress.tick();
            }
        }
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
