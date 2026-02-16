use crate::config::SIDEBAR_WIDTH;
use crate::state::{
    AutoSaveKind, GameProcess, LaunchTab, LogState, MenuCreateInstance, MenuCreateInstanceChoosing,
    MenuInstallOptifine,
};
use crate::tick::sort_dependencies;
use crate::{
    get_entries,
    state::{
        EditPresetsMessage, ManageModsMessage, MenuEditInstance, MenuEditMods, MenuInstallForge,
        MenuLaunch, ProgressBar, SelectedState, State, OFFLINE_ACCOUNT_NAME,
    },
    Launcher, Message,
};
use iced::futures::executor::block_on;
use iced::widget::scrollable::AbsoluteOffset;
use iced::Task;
use ql_core::json::instance_config::ModTypeInfo;
use ql_core::json::VersionDetails;
use ql_core::read_log::{Diagnostic, ReadError};
use ql_core::{
    err, json::instance_config::InstanceConfigJson, GenericProgress, InstanceSelection,
    IntoIoError, IntoJsonError, IntoStringError, JsonFileError,
};
use ql_core::{info, pt, LaunchedProcess};
use ql_instances::auth::AccountData;
use ql_mod_manager::{loaders, store::ModIndex};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::mpsc::{Receiver, Sender},
};
use tokio::io::AsyncWriteExt;

pub const SIDEBAR_LIMIT_RIGHT: f32 = 140.0;
pub const SIDEBAR_LIMIT_LEFT: f32 = 135.0;

mod iced_event;

impl Launcher {
    pub fn on_instance_selected(&mut self) -> Task<Message> {
        let instance = self.instance().clone();

        self.load_edit_instance(None);

        {
            let persistent = self.config.c_persistent();
            if persistent.selected_remembered {
                match instance.clone() {
                    InstanceSelection::Instance(n) => persistent.selected_instance = Some(n),
                    InstanceSelection::Server(n) => persistent.selected_server = Some(n),
                }
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
            }
        }
        self.load_logs(instance.clone());
        if let State::Launch(menu) = &mut self.state {
            menu.modal = None;
            menu.reload_notes(instance)
        } else {
            Task::none()
        }
    }

    pub fn load_logs(&mut self, instance: InstanceSelection) {
        let State::Launch(menu) = &mut self.state else {
            return;
        };
        if let (Some(logs), LaunchTab::Log) = (self.logs.get(&instance), menu.tab) {
            if menu.log_state.is_some() && Some(instance) == self.selected_instance {
                return;
            }
            menu.log_state = Some(LogState {
                content: iced::widget::text_editor::Content::with_text(&logs.log.join("\n")),
            });
        } else {
            menu.log_state = None;
        }
    }

    pub fn launch_game(&mut self, account_data: Option<AccountData>) -> Task<Message> {
        let username = if let Some(account_data) = &account_data {
            // Logged in account
            account_data.nice_username.clone()
        } else {
            // Offline username
            self.config.username.clone()
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        self.java_recv = Some(ProgressBar::with_recv(receiver));

        let global_settings = self.config.global_settings.clone();
        let extra_java_args = self.config.extra_java_args.clone().unwrap_or_default();

        let instance_name = self.instance().get_name().to_owned();

        Task::perform(
            async move {
                ql_instances::launch(
                    instance_name,
                    username,
                    Some(sender),
                    account_data,
                    global_settings,
                    extra_java_args,
                )
                .await
                .strerr()
            },
            Message::LaunchEnd,
        )
    }

    pub fn finish_launching(&mut self, result: Result<LaunchedProcess, String>) -> Task<Message> {
        self.java_recv = None;
        self.is_launching_game = false;
        match result {
            Ok(child) => {
                let selected_instance = child.instance.clone();

                let server_input = block_on(child.child.lock())
                    .stdin
                    .take()
                    .map(|n| (n, false));

                let (sender, receiver) = std::sync::mpsc::channel();
                self.processes.insert(
                    selected_instance.clone(),
                    GameProcess {
                        child: child.clone(),
                        receiver: Some(receiver),
                        server_input,
                    },
                );

                let mut censors = Vec::new();
                for account in self.accounts.values() {
                    if let Some(token) = &account.access_token {
                        censors.push(token.clone());
                    }
                }

                let version_presence_task = {
                    let selected_instance = selected_instance.clone();
                    let launcher_client = self.discord_ipc_client.clone();

                    info!("Game launch event; updating rich presence.");

                    Task::perform(
                        async move {
                            if let Ok(version_details) =
                                VersionDetails::load(&selected_instance).await
                            {
                                if let Some(client) = launcher_client {
                                    let details =
                                        format!("Instance: {}", selected_instance.get_name());
                                    let state = format!("Minecraft v{}", version_details.id);

                                    client.set_activity(&details, &state).await.ok();
                                }
                            }
                        },
                        |_| Message::Nothing,
                    )
                };

                let launch_task = Task::perform(
                    async move {
                        let result = child.read_logs(censors, Some(sender)).await;
                        let default_output = Ok((ExitStatus::default(), selected_instance, None));

                        match result {
                            Some(Err(ReadError::Io(io)))
                                if io.kind() == std::io::ErrorKind::InvalidData =>
                            {
                                err!("Minecraft log contains invalid unicode! Stopping logs");
                                pt!("The game will continue to run");
                                default_output
                            }
                            Some(result) => result.strerr(),
                            None => default_output,
                        }
                    },
                    Message::LaunchGameExited,
                );

                return Task::batch([launch_task, version_presence_task]);
            }
            Err(err) => self.set_error(err),
        }
        Task::none()
    }

    pub fn delete_instance_confirm(&mut self) -> Task<Message> {
        let State::ConfirmAction { .. } = &self.state else {
            return Task::none();
        };

        let selected_instance = self.instance();
        let is_server = selected_instance.is_server();
        let deleted_instance_dir = selected_instance.get_instance_path();
        if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
            self.set_error(err);
            return Task::none();
        }

        self.unselect_instance();
        if is_server {
            self.go_to_server_manage_menu(Some("Deleted Server".to_owned()))
        } else {
            self.go_to_launch_screen(Some("Deleted Instance".to_owned()))
        }
    }

    pub fn unselect_instance(&mut self) {
        self.selected_instance = None;
        self.config.c_persistent().selected_instance = None;
        self.config.c_persistent().selected_server = None;
        self.autosave.remove(&AutoSaveKind::LauncherConfig);
    }

    pub fn load_edit_instance_inner(
        edit_instance: &mut Option<MenuEditInstance>,
        selected_instance: &InstanceSelection,
    ) -> Result<(), JsonFileError> {
        let config_path = selected_instance.get_instance_path().join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson =
            serde_json::from_str(&config_json).json(config_json)?;

        let slider_value = f32::log2(config_json.ram_in_mb as f32);
        let memory_mb = config_json.ram_in_mb;

        // Use this to check for performance impact
        // std::thread::sleep(std::time::Duration::from_millis(500));

        let instance_name = selected_instance.get_name();

        *edit_instance = Some(MenuEditInstance {
            main_class_mode: config_json.get_main_class_mode(),
            config: config_json,
            slider_value,
            instance_name: instance_name.to_owned(),
            old_instance_name: instance_name.to_owned(),
            slider_text: format_memory(memory_mb),
            memory_input: memory_mb.to_string(),
            is_editing_name: false,
            arg_split_by_space: true,
        });
        Ok(())
    }

    pub fn go_to_edit_mods_menu(&mut self, check_updates: bool) -> Task<Message> {
        async fn inner(
            this: &mut Launcher,
            check_updates: bool,
        ) -> Result<Task<Message>, JsonFileError> {
            let instance = this.selected_instance.as_ref().unwrap();

            let config_json = InstanceConfigJson::read(instance).await?;
            let version_json = Box::new(VersionDetails::load(instance).await?);

            let mods = ModIndex::load(instance).await?;
            let update_local_mods_task =
                MenuEditMods::update_locally_installed_mods(&mods, instance);

            let locally_installed_mods = HashSet::new();
            let sorted_mods_list = sort_dependencies(&mods.mods, &locally_installed_mods);

            let (update_cmd, update_check_handle) = if !check_updates
                || this.mod_updates_checked.contains_key(instance)
                || config_json.mod_type.is_vanilla()
            {
                (Task::none(), None)
            } else {
                let (a, b) = Task::perform(
                    ql_mod_manager::store::check_for_updates(instance.clone()),
                    |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n.strerr())),
                )
                .abortable();
                (a, Some(b.abort_on_drop()))
            };

            let available_updates = if let Some(updates) = this.mod_updates_checked.get(instance) {
                updates.clone()
            } else {
                Vec::new()
            };

            this.state = State::EditMods(MenuEditMods {
                config: config_json,
                mods,
                selected_mods: HashSet::new(),
                shift_selected_mods: HashSet::new(),
                sorted_mods_list,
                selected_state: SelectedState::None,
                available_updates,
                mod_update_progress: None,
                locally_installed_mods,
                drag_and_drop_hovered: false,
                update_check_handle,
                version_json,
                modal: None,
                search: None,
                width_name: 220.0,
                list_shift_index: None,
                list_scroll: AbsoluteOffset::default(),
            });

            Ok(Task::batch([update_local_mods_task, update_cmd]))
        }
        match block_on(inner(self, check_updates)) {
            Ok(n) => n,
            Err(err) => {
                self.set_error(format!("While opening Mods screen:\n{err}"));
                Task::none()
            }
        }
    }

    pub fn set_game_exited(
        &mut self,
        status: ExitStatus,
        instance: &InstanceSelection,
        diagnostic: Option<Diagnostic>,
    ) -> Task<Message> {
        let kind = if instance.is_server() {
            "Server"
        } else {
            "Game"
        };
        info!("Game exited ({status})");

        let log_state = if let State::Launch(MenuLaunch {
            message, log_state, ..
        }) = &mut self.state
        {
            let has_crashed = !status.success();
            if has_crashed {
                *message = format!("{kind} crashed! ({status})\nCheck \"Logs\" for more info");
                if let Some(diag) = diagnostic {
                    message.push_str("\n\n");
                    message.push_str(&diag.to_string());
                }
            }
            if let Some(log) = self.logs.get_mut(instance) {
                log.has_crashed = has_crashed;
            }
            log_state
        } else {
            &mut None
        };

        if let Some(process) = self.processes.remove(instance) {
            Self::read_game_logs(&process, instance, &mut self.logs, log_state);
        }

        let presence_task = {
            let instance = instance.clone();
            let client = self.discord_ipc_client.clone();

            Task::perform(
                async move {
                    let details = VersionDetails::load(&instance).await;
                    info!("Exit event; updating rich presence.");

                    // TODO: might implement time here? since VersionDetails exposes this
                    // but I'm not sure whether this is fully implemented yet

                    if let Ok(details) = details {
                        if let Some(client) = client {
                            client
                                .set_activity(
                                    &format!("Last played version: {}", details.id),
                                    &format!("Instance: {}", instance.get_name()),
                                )
                                .await
                                .ok();
                        }
                    }
                },
                |_| Message::Nothing,
            )
        };

        presence_task
    }

    pub fn start_discord_ipc_run(&self) -> Task<Message> {
        if let Some(client) = &self.discord_ipc_client {
            let mut client = client.clone();

            let version = env!("CARGO_PKG_VERSION");

            Task::perform(
                async move {
                    // Start the IPC run loop
                    if client.run().await.is_ok() {
                        // After run() is successful, update presence
                        client
                            .set_activity("Started launcher", &format!("Version {version}"))
                            .await
                            .ok();
                    }
                },
                |_| Message::Nothing,
            )
        } else {
            Task::none()
        }
    }

    pub fn update_mods(&mut self) -> Task<Message> {
        if let State::EditMods(menu) = &mut self.state {
            let updates = menu
                .available_updates
                .clone()
                .into_iter()
                .map(|(n, _, _)| n)
                .collect();
            let (sender, receiver) = std::sync::mpsc::channel();
            menu.mod_update_progress = Some(ProgressBar::with_recv_and_msg(
                receiver,
                "Deleting Mods".to_owned(),
            ));
            let selected_instance = self.selected_instance.clone().unwrap();
            Task::perform(
                ql_mod_manager::store::apply_updates(selected_instance, updates, Some(sender)),
                |n| Message::ManageMods(ManageModsMessage::UpdateModsFinished(n.strerr())),
            )
        } else {
            Task::none()
        }
    }

    pub fn go_to_server_manage_menu(&mut self, message: Option<String>) -> Task<Message> {
        if let State::Launch(menu) = &mut self.state {
            menu.is_viewing_server = true;
            if let Some(message) = message {
                menu.message = message;
            }
        } else {
            let mut menu_launch = match message {
                Some(message) => MenuLaunch::with_message(message),
                None => MenuLaunch::default(),
            };
            menu_launch.is_viewing_server = true;
            menu_launch.resize_sidebar(SIDEBAR_WIDTH);
            self.state = State::Launch(menu_launch);
        }

        let get_entries = Task::perform(get_entries(true), Message::CoreListLoaded);
        match &self.selected_instance {
            Some(InstanceSelection::Instance(_)) => self.selected_instance = None,
            Some(i @ InstanceSelection::Server(_)) => {
                if let State::Launch(menu) = &mut self.state {
                    return Task::batch([get_entries, menu.reload_notes(i.clone())]);
                }
            }
            None => {}
        }
        get_entries
    }

    pub fn install_forge(&mut self, kind: ForgeKind) -> Task<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver): (Sender<GenericProgress>, Receiver<GenericProgress>) =
            std::sync::mpsc::channel();

        let instance_selection = self.selected_instance.clone().unwrap();
        let instance_selection2 = instance_selection.clone();

        let command = Task::perform(
            async move {
                if matches!(kind, ForgeKind::NeoForge) {
                    // TODO: Add UI to specify NeoForge version
                    loaders::neoforge::install(
                        None,
                        instance_selection2,
                        Some(f_sender),
                        Some(j_sender),
                    )
                    .await
                } else {
                    loaders::forge::install(
                        None,
                        instance_selection2,
                        Some(f_sender),
                        Some(j_sender),
                    )
                    .await
                }
                .strerr()?;
                if matches!(kind, ForgeKind::OptiFine) {
                    copy_optifine_over(&instance_selection)
                        .await
                        .map_err(|n| format!("Couldn't install OptiFine with Forge:\n{n}"))?;
                    loaders::optifine::uninstall(instance_selection.get_name().to_owned(), false)
                        .await
                        .strerr()?;
                }
                Ok(())
            },
            Message::InstallForgeEnd,
        );

        self.state = State::InstallForge(MenuInstallForge {
            forge_progress: ProgressBar::with_recv(f_receiver),
            java_progress: ProgressBar::with_recv(j_receiver),
            is_java_getting_installed: false,
        });
        command
    }

    pub fn go_to_main_menu_with_message(
        &mut self,
        message: Option<impl ToString>,
    ) -> Task<Message> {
        let message = message.map(|n| n.to_string());
        if self.server_selected() {
            self.go_to_server_manage_menu(message)
        } else {
            self.go_to_launch_screen::<String>(message)
        }
    }

    pub fn server_selected(&self) -> bool {
        self.selected_instance
            .as_ref()
            .is_some_and(|n| n.is_server())
            || if let State::Launch(menu) = &self.state {
                menu.is_viewing_server
            } else if let State::Create(MenuCreateInstance::Choosing(
                MenuCreateInstanceChoosing { is_server, .. },
            )) = &self.state
            {
                *is_server
            } else {
                false
            }
    }

    pub fn get_selected_dot_minecraft_dir(&self) -> Option<PathBuf> {
        Some(self.selected_instance.as_ref()?.get_dot_minecraft_path())
    }

    fn load_modpack_from_path(&mut self, path: PathBuf) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

        Task::perform(
            ql_mod_manager::add_files(
                self.selected_instance.clone().unwrap(),
                vec![path],
                Some(sender),
            ),
            |n| Message::ManageMods(ManageModsMessage::AddFileDone(n.strerr())),
        )
    }

    fn load_jar_from_path(&mut self, path: &Path, filename: &str) {
        let selected_instance = self.instance();
        let new_path = selected_instance
            .get_dot_minecraft_path()
            .join("mods")
            .join(filename);
        if *path != new_path {
            if let Err(err) = std::fs::copy(path, &new_path) {
                err!("Couldn't drag and drop mod file in: {err}");
            }
        }
    }

    pub fn load_qmp_from_path(&mut self, path: &Path) -> Task<Message> {
        let file = match std::fs::read(path) {
            Ok(n) => n,
            Err(err) => {
                err!("Couldn't drag and drop preset file: {err}");
                return Task::none();
            }
        };
        match tokio::runtime::Handle::current().block_on(ql_mod_manager::Preset::load(
            self.selected_instance.clone().unwrap(),
            file,
            true,
        )) {
            Ok(mods) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::EditMods(_) = &self.state {
                    self.go_to_edit_presets_menu();
                }
                if let State::ManagePresets(menu) = &mut self.state {
                    menu.progress = Some(ProgressBar::with_recv(receiver));
                }
                let instance_name = self.selected_instance.clone().unwrap();
                Task::perform(
                    ql_mod_manager::store::download_mods_bulk(
                        mods.to_install,
                        instance_name,
                        Some(sender),
                    ),
                    |n| Message::EditPresets(EditPresetsMessage::LoadComplete(n.strerr())),
                )
            }
            Err(err) => {
                self.set_error(err);
                Task::none()
            }
        }
    }

    fn set_drag_and_drop_hover(&mut self, is_hovered: bool) {
        if let State::EditMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::ManagePresets(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::EditJarMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::InstallOptifine(MenuInstallOptifine::Choosing {
            drag_and_drop_hovered,
            ..
        }) = &mut self.state
        {
            *drag_and_drop_hovered = is_hovered;
        }
    }
    #[cfg(feature = "auto_update")]
    pub fn update_download_start(&mut self) -> Task<Message> {
        if let State::UpdateFound(crate::state::MenuLauncherUpdate { url, progress, .. }) =
            &mut self.state
        {
            let (sender, update_receiver) = std::sync::mpsc::channel();
            *progress = Some(ProgressBar::with_recv_and_msg(
                update_receiver,
                "Starting Update".to_owned(),
            ));

            let url = url.clone();

            Task::perform(
                async move {
                    ql_instances::install_launcher_update(url, sender)
                        .await
                        .strerr()
                },
                Message::UpdateDownloadEnd,
            )
        } else {
            Task::none()
        }
    }

    pub fn kill_selected_instance(&mut self) -> Task<Message> {
        let Some(instance) = &self.selected_instance else {
            return Task::none();
        };
        match instance {
            InstanceSelection::Instance(_) => {
                if let Some(process) = self.processes.remove(instance) {
                    let mut child = block_on(process.child.child.lock());
                    _ = child.start_kill();
                }
            }
            InstanceSelection::Server(_) => {
                if let Some(GameProcess {
                    server_input: Some((stdin, has_issued_stop_command)),
                    child,
                    ..
                }) = self.processes.get_mut(instance)
                {
                    *has_issued_stop_command = true;
                    if child.is_classic_server {
                        _ = block_on(child.child.lock()).start_kill();
                    } else {
                        let future = stdin.write_all("stop\n".as_bytes());
                        _ = block_on(future);
                    };
                }
            }
        }
        Task::none()
    }

    pub fn go_to_delete_instance_menu(&mut self) {
        let instance = self.instance();
        self.state = State::ConfirmAction {
            msg1: format!(
                "delete the {} {}",
                if instance.is_server() {
                    "server"
                } else {
                    "instance"
                },
                instance.get_name()
            ),
            msg2: "All your data, including worlds, will be lost".to_owned(),
            yes: Message::DeleteInstance,
            no: Message::MScreenOpen {
                message: None,
                clear_selection: false,
                is_server: None,
            },
        };
    }

    pub fn launch_start(&mut self) -> Task<Message> {
        let Some(selected_instance) = &self.selected_instance else {
            return Task::none();
        };
        if self.processes.contains_key(selected_instance) {
            return Task::none();
        }
        self.logs.remove(selected_instance);

        match selected_instance {
            InstanceSelection::Instance(_) => {
                if let Some(account) = &self.accounts_selected {
                    if account == OFFLINE_ACCOUNT_NAME
                        && (self.config.username.is_empty() || self.config.username.contains(' '))
                    {
                        return Task::none();
                    }
                }

                self.is_launching_game = true;
                let account_data = self.get_selected_account_data();
                // If the user is loading an existing login from disk
                // then first refresh the tokens
                if let Some(account) = &account_data {
                    if account.access_token.is_none() || account.needs_refresh {
                        return self.account_refresh(account);
                    }
                }
                // Or, if the account is already refreshed/freshly added,
                // directly launch the game
                self.launch_game(account_data)
            }
            InstanceSelection::Server(server) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.java_recv = Some(ProgressBar::with_recv(receiver));

                let server = server.clone();
                Task::perform(
                    async move { ql_servers::run(server, Some(sender)).await.strerr() },
                    Message::LaunchEnd,
                )
            }
        }
    }
}

pub async fn get_locally_installed_mods(
    selected_instance: PathBuf,
    blacklist: Vec<String>,
) -> HashSet<String> {
    let mods_dir_path = selected_instance.join("mods");

    let Ok(mut dir) = tokio::fs::read_dir(&mods_dir_path).await else {
        err!("Error reading mods directory");
        return HashSet::new();
    };
    let mut set = HashSet::new();
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if blacklist.contains(&file_name.to_owned()) {
            continue;
        }
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension == "jar" || extension == "disabled" {
            set.insert(file_name.to_owned());
        }
    }
    set
}

pub fn format_memory(memory_bytes: usize) -> String {
    const MB_TO_GB: usize = 1024;

    if memory_bytes >= MB_TO_GB {
        format!("{:.2} GB", memory_bytes as f64 / MB_TO_GB as f64)
    } else {
        format!("{memory_bytes} MB")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ForgeKind {
    Normal,
    NeoForge,
    OptiFine,
}

async fn copy_optifine_over(instance: &InstanceSelection) -> Result<(), String> {
    let instance_dir = instance.get_instance_path();
    let installer_path = instance_dir.join("optifine/OptiFine.jar");
    let mods_dir = instance_dir.join(".minecraft/mods");

    if !installer_path.exists() {
        return Ok(());
    }
    if !mods_dir.exists() {
        tokio::fs::create_dir_all(&mods_dir)
            .await
            .path(&mods_dir)
            .strerr()?;
    }
    let new_path = mods_dir.join("optifine.jar");
    tokio::fs::copy(&installer_path, &new_path).await.strerr()?;

    let mut config = InstanceConfigJson::read(instance).await.strerr()?;
    config
        .mod_type_info
        .get_or_insert_with(ModTypeInfo::default)
        .optifine_jar = Some("optifine.jar".to_owned());
    config.save(instance).await.strerr()?;

    Ok(())
}
