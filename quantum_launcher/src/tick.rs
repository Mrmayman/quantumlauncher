use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::Datelike;
use iced::Task;
use ql_core::{
    constants::OS_NAME, json::InstanceConfigJson, InstanceSelection, IntoIoError, IntoJsonError,
    IntoStringError, JsonFileError, ModId,
};
use ql_mod_manager::store::{ModConfig, ModIndex};

use crate::state::{
    AutoSaveKind, EditInstanceMessage, GameProcess, InstallModsMessage, InstanceLog, LaunchTab,
    Launcher, ManageJarModsMessage, MenuCreateInstance, MenuEditMods, MenuExportInstance,
    MenuInstallFabric, MenuInstallOptifine, MenuLaunch, MenuLoginMS, MenuModsDownload,
    MenuRecommendedMods, Message, ModListEntry, State,
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
            | State::Launch(_) => {}

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
            | State::InstallJava(_)
            | State::ExportMods(_) => {}
        }
    }

    pub fn tick(&mut self) -> Task<Message> {
        match &mut self.state {
            State::Launch(MenuLaunch {
                ref edit_instance,
                ref tab,
                ..
            }) => {
                let autosave_instancecfg =
                    if let (Some(edit), LaunchTab::Edit) = (&edit_instance, tab) {
                        let config = edit.config.clone();
                        self.tick_edit_instance(config)
                    } else {
                        Task::none()
                    };
                self.tick_processes_and_logs();
                let autosave_launchercfg = self.autosave_config();

                return Task::batch([autosave_instancecfg, autosave_launchercfg]);
            }
            State::Create(_) => {
                return self.autosave_config();
            }
            State::EditMods(menu) => {
                let instance_selection = self.selected_instance.as_ref().unwrap();
                let update_locally_installed_mods = menu.tick(instance_selection);
                return update_locally_installed_mods;
            }
            State::ModsDownload(_) => {
                return MenuModsDownload::tick(self.selected_instance.clone().unwrap())
            }
            State::LauncherSettings(_) => {
                let launcher_config = self.config.clone();
                return Task::perform(
                    async move { launcher_config.save().await.strerr() },
                    Message::CoreTickConfigSaved,
                );
            }
            State::EditJarMods(menu) => {
                if self.autosave.insert(AutoSaveKind::Jarmods) {
                    let mut jarmods = menu.jarmods.clone();
                    let selected_instance = self.selected_instance.clone().unwrap();
                    return Task::perform(
                        async move { (jarmods.save(&selected_instance).await.strerr(), jarmods) },
                        |n| Message::ManageJarMods(ManageJarModsMessage::AutosaveFinished(n)),
                    );
                }
            }

            _ => {}
        }

        Task::none()
    }

    pub fn autosave_config(&mut self) -> Task<Message> {
        if self.tick_timer.is_multiple_of(5) && self.autosave.insert(AutoSaveKind::LauncherConfig) {
            let launcher_config = self.config.clone();
            Task::perform(
                async move { launcher_config.save().await.strerr() },
                Message::CoreTickConfigSaved,
            )
        } else {
            Task::none()
        }
    }

    fn tick_edit_instance(&self, config: InstanceConfigJson) -> Task<Message> {
        let Some(instance) = self.selected_instance.clone() else {
            return Task::none();
        };
        Task::perform(Launcher::save_config(instance, config), |n| {
            Message::EditInstance(EditInstanceMessage::ConfigSaved(n.strerr()))
        })
    }

    fn tick_processes_and_logs(&mut self) {
        let mut killed_processes = Vec::new();
        for (name, process) in &mut self.processes {
            Self::read_game_logs(process, name, &mut self.logs);
            if let Ok(Some(_)) = process.child.child.lock().unwrap().try_wait() {
                // Game process has exited.
                killed_processes.push(name.to_owned());
            }
        }
        for name in killed_processes {
            self.processes.remove(&name);
        }
    }

    fn read_game_logs(
        process: &GameProcess,
        name: &InstanceSelection,
        logs: &mut HashMap<InstanceSelection, InstanceLog>,
    ) {
        while let Some(message) = process.receiver.as_ref().and_then(|n| n.try_recv().ok()) {
            let message = message.to_string().replace('\t', &" ".repeat(8));

            let mut log_start = vec![
                format!(
                    "{} ({})\n",
                    if name.is_server() {
                        "Starting Minecraft server"
                    } else {
                        "Launching Minecraft"
                    },
                    Self::get_current_date_formatted()
                ),
                format!("OS: {OS_NAME}\n"),
            ];

            if !logs.contains_key(name) {
                log_start.push(message);

                logs.insert(
                    name.to_owned(),
                    InstanceLog {
                        log: log_start,
                        has_crashed: false,
                        command: String::new(),
                    },
                );
            } else if let Some(log) = logs.get_mut(name) {
                if log.log.is_empty() {
                    log.log = log_start;
                }
                log.log.push(message);
            }
        }
    }

    fn get_current_date_formatted() -> String {
        // Get the current date and time in UTC
        let now = chrono::Local::now();

        // Extract the day, month, and year
        let day = now.day();
        let month = now.format("%B").to_string(); // Full month name (e.g., "September")
        let year = now.year();

        // Return the formatted string
        format!("{day} {month} {year}")
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
            |n| Message::InstallMods(InstallModsMessage::IndexUpdated(n.strerr())),
        )
    }
}

pub fn sort_dependencies(
    downloaded_mods: &HashMap<String, ModConfig>,
    locally_installed_mods: &HashSet<String>,
) -> Vec<ModListEntry> {
    let mut entries: Vec<ModListEntry> = downloaded_mods
        .iter()
        .map(|(k, v)| ModListEntry::Downloaded {
            id: ModId::from_index_str(k),
            config: Box::new(v.clone()),
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

        MenuEditMods::update_locally_installed_mods(&self.mods, instance_selection)
    }
}
