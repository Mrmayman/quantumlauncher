use std::{collections::HashMap, process::ExitStatus};

use chrono::Utc;
use iced::{Task, futures::executor::block_on};
use ql_core::{
    Instance, InstanceConfigJson, InstanceKind, IntoStringError, LaunchedProcess, err, info, pt,
    read_log::{Diagnostic, ReadError},
};
use ql_instances::auth::AccountData;
use tokio::io::AsyncWriteExt;

use crate::{
    config::{AfterLaunchBehavior, sidebar::SidebarSelection},
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    state::{
        AutoSaveKind, GameProcess, InfoMessage, LaunchMessage, LaunchModal, LaunchTab, Launcher,
        MainMenuMessage, Message, OFFLINE_ACCOUNT_NAME, ProgressBar, SidebarMessage, State,
    },
};

impl Launcher {
    pub fn update_launch(&mut self, msg: LaunchMessage) -> Task<Message> {
        match msg {
            LaunchMessage::GameExited(Err(err)) => {
                self.set_error(err);
                Task::none()
            }
            LaunchMessage::GameExited(Ok((status, instance, diagnostic))) => {
                self.set_game_exited(status, &instance, diagnostic)
            }
            LaunchMessage::Start => self.launch_start(),
            LaunchMessage::End(result) => self.finish_launching(result),
            LaunchMessage::Kill => self.kill_selected_instance(),
            LaunchMessage::LaunchedGamesLoaded(result) => self.load_launched_games(result),
            LaunchMessage::LaunchedGamesSaved(Err(err)) => {
                err!("Could not save launched games: {err}");
                Task::none()
            }
            LaunchMessage::LaunchedGamesSaved(Ok(())) => Task::none(),
            LaunchMessage::ProcessesChecked(result) => self.remove_closed_games(result),
            LaunchMessage::PersistedGameKilled { instance, result } => {
                self.finish_killing_persisted_game(instance, result)
            }
        }
    }

    fn load_launched_games(
        &mut self,
        result: Result<Vec<crate::launched_games::LaunchedGame>, String>,
    ) -> Task<Message> {
        match result {
            Ok(games) => {
                for game in games {
                    self.processes
                        .entry(game.instance())
                        .or_insert_with(|| GameProcess::restored(&game));
                }
                self.save_launched_games()
            }
            Err(err) => {
                err!("Could not load launched games: {err}");
                Task::none()
            }
        }
    }

    fn remove_closed_games(&mut self, result: Result<Vec<Instance>, String>) -> Task<Message> {
        self.is_checking_processes = false;
        let closed = match result {
            Ok(closed) => closed,
            Err(err) => {
                err!("Could not check launched games: {err}");
                return Task::none();
            }
        };

        if closed.is_empty() {
            return Task::none();
        }

        let mut tasks = Vec::with_capacity(closed.len() + 1);
        for instance in closed {
            if let Some((_, secs)) =
                end_process(&mut self.processes, &mut self.last_play_time, &instance)
            {
                tasks.push(self.add_play_time(&instance, secs));
                tasks.push(self.rpc_game_update(instance, true));
            }
        }
        tasks.push(self.save_launched_games());
        Task::batch(tasks)
    }

    fn finish_killing_persisted_game(
        &mut self,
        instance: Instance,
        result: Result<(), String>,
    ) -> Task<Message> {
        if let Err(err) = result {
            err!("Could not kill {}: {err}", instance.get_name());
            return Task::none();
        }

        let play_time = end_process(&mut self.processes, &mut self.last_play_time, &instance)
            .map(|(_, secs)| self.add_play_time(&instance, secs))
            .unwrap_or_else(Task::none);
        Task::batch([self.save_launched_games(), play_time])
    }

    fn launch_start(&mut self) -> Task<Message> {
        let Some(selected_instance) = &self.selected_instance else {
            return Task::none();
        };
        if self.processes.contains_key(selected_instance) {
            return Task::none();
        }
        self.logs.remove(selected_instance);
        self.last_play_time.remove(selected_instance);

        match selected_instance.kind {
            InstanceKind::Client => {
                if self.account_selected == OFFLINE_ACCOUNT_NAME
                    && (self.config.username.is_empty() || self.config.username.contains(' '))
                {
                    return Task::none();
                }

                self.is_launching_game = true;
                let account_data = self.get_selected_account_data();
                // If the user is loading an existing login from disk
                // then first refresh the tokens
                if let Some(account) = &account_data
                    && (account.access_token.is_none() || account.needs_refresh)
                {
                    return self.account_refresh(account);
                }
                // Or, if the account is already refreshed/freshly added,
                // directly launch the game
                self.launch_game(account_data)
            }
            InstanceKind::Server => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.java_recv = Some(ProgressBar::with_recv(receiver));

                let server = selected_instance.name.clone();
                Task::perform(ql_servers::run(server, Some(sender)), |n| {
                    LaunchMessage::End(n.strerr()).into()
                })
            }
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

        let instance_name = self.instance().name.clone();
        Task::perform(
            ql_instances::launch(
                instance_name,
                username,
                Some(sender),
                account_data,
                global_settings,
                extra_java_args,
            ),
            |n| LaunchMessage::End(n.strerr()).into(),
        )
    }

    fn set_game_exited(
        &mut self,
        status: ExitStatus,
        instance: &Instance,
        diagnostic: Option<Diagnostic>,
    ) -> Task<Message> {
        let kind = if instance.is_server() {
            "Server"
        } else {
            "Game"
        };
        info!("Game exited ({status})");

        let log_state = if let State::Launch(menu) = &mut self.state {
            let has_crashed = !status.success();

            if has_crashed {
                let mut msg = format!("{kind} crashed! ({status})\nCheck \"Logs\" for more info");
                if let Some(diag) = diagnostic {
                    msg.push_str("\n\n");
                    msg.push_str(&diag.to_string());
                }
                menu.message = Some(InfoMessage::error(msg));
            }

            if let Some(log) = self.logs.get_mut(instance) {
                log.has_crashed = has_crashed;
            }

            &mut menu.log_state
        } else {
            &mut None
        };

        let mut play_time = Task::none();
        if let Some((process, secs)) =
            end_process(&mut self.processes, &mut self.last_play_time, instance)
        {
            Self::read_game_logs(
                &process,
                instance,
                &mut self.logs,
                log_state,
                self.selected_instance.as_ref(),
            );
            play_time = self.add_play_time(instance, secs);
        }

        Task::batch([
            self.save_launched_games(),
            self.rpc_game_update(instance.clone(), true),
            play_time,
        ])
    }

    fn finish_launching(&mut self, result: Result<LaunchedProcess, String>) -> Task<Message> {
        self.java_recv = None;
        self.is_launching_game = false;
        match result {
            Ok(child) => {
                let selected_instance = child.instance.clone();

                let (server_input, pid) = {
                    let mut process = block_on(child.child.lock());
                    (process.stdin.take().map(|n| (n, false)), process.id())
                };

                let (sender, receiver) = std::sync::mpsc::channel();
                self.processes.insert(
                    selected_instance.clone(),
                    GameProcess {
                        child: Some(child.clone()),
                        pid,
                        launched_at: Utc::now(),
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

                let version_presence_task = self.rpc_game_update(selected_instance.clone(), false);
                let save_launched_games = self.save_launched_games();

                let log_task = Task::perform(
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
                    |n| LaunchMessage::GameExited(n).into(),
                );

                match self.config.c_after_launch_behavior() {
                    AfterLaunchBehavior::DoNothing => {}
                    AfterLaunchBehavior::CloseLauncher => {
                        ql_core::logger_finish();
                        self.close_launcher();
                    }
                    AfterLaunchBehavior::MinimizeLauncher => {
                        let minimize_task = iced::window::get_latest()
                            .and_then(|id| iced::window::minimize(id, true));
                        return Task::batch([
                            log_task,
                            minimize_task,
                            version_presence_task,
                            save_launched_games,
                        ]);
                    }
                }

                return Task::batch([log_task, version_presence_task, save_launched_games]);
            }
            Err(err) => self.set_error(err),
        }
        Task::none()
    }

    fn kill_selected_instance(&mut self) -> Task<Message> {
        let Some(instance) = self.selected_instance.clone() else {
            return Task::none();
        };
        match instance.kind {
            InstanceKind::Client => {
                if let Some(process) = self.processes.get(&instance)
                    && process.is_restored()
                    && let Some(pid) = process.pid
                {
                    return Task::perform(crate::launched_games::kill(pid), move |result| {
                        LaunchMessage::PersistedGameKilled {
                            instance: instance.clone(),
                            result,
                        }
                        .into()
                    });
                }

                if let Some((process, secs)) =
                    end_process(&mut self.processes, &mut self.last_play_time, &instance)
                    && let Some(child) = process.child
                {
                    let mut child = block_on(child.child.lock());
                    _ = child.start_kill();
                    return Task::batch([
                        self.save_launched_games(),
                        self.add_play_time(&instance, secs),
                    ]);
                }
            }
            InstanceKind::Server => {
                if let Some(GameProcess {
                    server_input: Some((stdin, has_issued_stop_command)),
                    child: Some(child),
                    ..
                }) = self.processes.get_mut(&instance)
                {
                    *has_issued_stop_command = true;
                    if child.is_classic_server {
                        _ = block_on(child.child.lock()).start_kill();
                    } else {
                        let future = stdin.write_all("stop\n".as_bytes());
                        _ = block_on(future);
                    }
                } else if let Some(process) = self.processes.get(&instance)
                    && let Some(pid) = process.pid
                {
                    return Task::perform(crate::launched_games::kill(pid), move |result| {
                        LaunchMessage::PersistedGameKilled {
                            instance: instance.clone(),
                            result,
                        }
                        .into()
                    });
                }
            }
        }
        Task::none()
    }

    /// Persists a finished session's play time to the instance's `config.json`
    /// and reports the new total back through [`MainMenuMessage::TotalPlayTimeLoaded`].
    fn add_play_time(&self, instance: &Instance, secs: u64) -> Task<Message> {
        if secs == 0 {
            return Task::none();
        }
        let future_instance = instance.clone();
        let msg_instance = instance.clone();
        Task::perform(
            async move {
                InstanceConfigJson::add_play_time(&future_instance, secs)
                    .await
                    .strerr()
            },
            move |res| MainMenuMessage::TotalPlayTimeLoaded(msg_instance.clone(), res).into(),
        )
    }

    fn save_launched_games(&self) -> Task<Message> {
        let games = self
            .processes
            .iter()
            .filter_map(|(instance, process)| process.launched_game(instance))
            .collect();
        *self.launched_games.lock().unwrap() = games;

        Task::perform(
            crate::launched_games::save(self.launched_games.clone()),
            |n| LaunchMessage::LaunchedGamesSaved(n).into(),
        )
    }

    pub fn update_main_menu(&mut self, msg: MainMenuMessage) -> Task<Message> {
        match msg {
            MainMenuMessage::ChangeTab(tab) => {
                // UX tweak: dragging instance to tab will open tab for that instance
                if let State::Launch(menu) = &mut self.state {
                    let modal = &mut menu.modal;

                    if let Some(LaunchModal::SDragging {
                        being_dragged: SidebarSelection::Instance(name, kind),
                        ..
                    }) = modal
                        && self.selected_instance.is_none()
                    {
                        self.selected_instance = Some(Instance::new(name, *kind));
                    }
                    *modal = None;
                }

                self.load_edit_instance(Some(tab));
                if let LaunchTab::Log = tab {
                    self.load_logs();
                }
            }
            MainMenuMessage::Modal(modal) => {
                if let State::Launch(menu) = &mut self.state {
                    let t = if let Some(LaunchModal::SRenamingFolder(_, _, _)) = &modal {
                        iced::widget::text_input::focus("MenuLaunch:rename_folder")
                    } else {
                        Task::none()
                    };
                    menu.modal = match (&modal, &menu.modal) {
                        // Unset if you click on it again
                        (
                            Some(LaunchModal::InstanceOptions),
                            Some(LaunchModal::InstanceOptions),
                        ) => None,
                        _ => modal.clone(),
                    };
                    return t;
                }
            }
            MainMenuMessage::InstanceSelected(inst) => {
                self.selected_instance = Some(inst);
                return self.on_selecting_instance();
            }
            MainMenuMessage::UsernameSet(username) => {
                self.config.username = username;
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
            }
            MainMenuMessage::SetInfoMessage(msg) => {
                if let State::Launch(menu) = &mut self.state {
                    menu.message = msg;
                }
            }
            MainMenuMessage::TotalPlayTimeLoaded(instance, res) => {
                if self.selected_instance.as_ref() != Some(&instance) {
                    return Task::none();
                }
                match res {
                    Ok(secs) => {
                        if let State::Launch(menu) = &mut self.state {
                            menu.total_play_time_secs = Some(secs);
                            // Keep the edit screen's config in sync so autosave
                            // doesn't overwrite the new total with a stale value
                            if let Some(edit) = &mut menu.edit_instance {
                                edit.config.total_play_time_secs = Some(secs);
                            }
                        }
                    }
                    Err(err) => err!(no_log, "While loading instance play time: {err}"),
                }
            }
        }
        Task::none()
    }

    fn sidebar_update_state(&mut self) {
        self.hide_submenu();
        self.config.c_sidebar().fix();
        self.autosave.remove(&AutoSaveKind::LauncherConfig);
    }

    pub fn update_sidebar(&mut self, message: SidebarMessage) -> Task<Message> {
        match message {
            SidebarMessage::Resize(ratio) => {
                if let State::Launch(menu) = &mut self.state {
                    let window_width = self.window_state.size.0;
                    let ratio = ratio * window_width;
                    menu.resize_sidebar(
                        ratio.clamp(SIDEBAR_LIMIT_LEFT, window_width - SIDEBAR_LIMIT_RIGHT)
                            / window_width,
                    );
                }
            }
            SidebarMessage::Scroll(scroll) => {
                if let State::Launch(menu) = &mut self.state {
                    menu.sidebar_scroll = scroll;
                }
            }
            SidebarMessage::NewFolder(at_position) => {
                let folder_id = self
                    .config
                    .c_sidebar()
                    .new_folder_at(at_position, "New Folder");
                self.sidebar_update_state();
                if let State::Launch(menu) = &mut self.state {
                    menu.modal = Some(LaunchModal::SRenamingFolder(
                        folder_id,
                        "New Folder".to_owned(),
                        true,
                    ));
                    return iced::widget::text_input::focus("MenuLaunch:rename_folder");
                }
            }
            SidebarMessage::DeleteFolder(folder) => {
                self.config.c_sidebar().delete_folder(folder);
                self.sidebar_update_state();
            }
            SidebarMessage::ToggleFolderVisibility(id) => {
                let sidebar = self.config.c_sidebar();
                sidebar.toggle_visibility(id);
                self.sidebar_update_state();
            }
            SidebarMessage::DragDrop(location) => {
                if let State::Launch(menu) = &mut self.state
                    && let Some(LaunchModal::SDragging { being_dragged, .. }) = &mut menu.modal
                {
                    self.config.c_sidebar().drag_drop(being_dragged, location);
                }
                self.sidebar_update_state();
            }
            SidebarMessage::DragHover { location, entered } => {
                if let State::Launch(menu) = &mut self.state
                    && let Some(LaunchModal::SDragging { dragged_to, .. }) = &mut menu.modal
                {
                    if entered {
                        *dragged_to = Some(location);
                    } else if dragged_to.as_ref().is_some_and(|n| *n == location) {
                        *dragged_to = None;
                    }
                }
            }
            SidebarMessage::FolderRenameConfirm => {
                if let State::Launch(menu) = &mut self.state
                    && let Some(LaunchModal::SRenamingFolder(id, name, _)) = &mut menu.modal
                {
                    self.config
                        .c_sidebar()
                        .rename(&SidebarSelection::Folder(*id), name);
                    self.hide_submenu();
                }
                self.sidebar_update_state();
            }
        }
        Task::none()
    }
}

/// Removes a tracked process and freezes its play time for the UI.
/// Returns the process and its elapsed play time in seconds.
fn end_process(
    processes: &mut HashMap<Instance, GameProcess>,
    last_play_time: &mut HashMap<Instance, u64>,
    instance: &Instance,
) -> Option<(GameProcess, u64)> {
    let process = processes.remove(instance)?;
    let secs = process.play_time_secs();
    last_play_time.insert(instance.clone(), secs);
    Some((process, secs))
}
