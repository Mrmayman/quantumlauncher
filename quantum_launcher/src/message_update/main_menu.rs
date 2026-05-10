use std::process::ExitStatus;

use iced::{Task, futures::executor::block_on};
use ql_core::{
    Instance, InstanceKind, IntoStringError, LaunchedProcess, err, info, pt,
    read_log::{Diagnostic, ReadError},
};
use ql_instances::auth::AccountData;
use tokio::io::AsyncWriteExt;

use crate::{
    config::{AfterLaunchBehavior, sidebar::SidebarSelection},
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    state::{
        AutoSaveKind, GameProcess, InfoMessage, LaunchMessage, LaunchModal, LaunchTab, Launcher,
        MainMenuMessage, MenuLaunch, Message, OFFLINE_ACCOUNT_NAME, ProgressBar, SidebarMessage,
        State,
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
        }
    }

    fn launch_start(&mut self) -> Task<Message> {
        let Some(selected_instance) = &self.selected_instance else {
            return Task::none();
        };
        if self.processes.contains_key(selected_instance) {
            return Task::none();
        }
        self.logs.remove(selected_instance);

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
                if let Some(account) = &account_data {
                    if account.access_token.is_none() || account.needs_refresh {
                        return self.account_refresh(account);
                    }
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

        let log_state = if let State::Launch(MenuLaunch {
            message, log_state, ..
        }) = &mut self.state
        {
            let has_crashed = !status.success();
            if has_crashed {
                let mut msg = format!("{kind} crashed! ({status})\nCheck \"Logs\" for more info");
                if let Some(diag) = diagnostic {
                    msg.push_str("\n\n");
                    msg.push_str(&diag.to_string());
                }
                *message = Some(InfoMessage::error(msg));
            }
            if let Some(log) = self.logs.get_mut(instance) {
                log.has_crashed = has_crashed;
            }
            log_state
        } else {
            &mut None
        };

        if let Some(process) = self.processes.remove(instance) {
            Self::read_game_logs(
                &process,
                instance,
                &mut self.logs,
                log_state,
                self.selected_instance.as_ref(),
            );
        }

        self.rpc_game_update(instance.clone(), true)
    }

    fn finish_launching(&mut self, result: Result<LaunchedProcess, String>) -> Task<Message> {
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

                let version_presence_task = self.rpc_game_update(selected_instance.clone(), false);

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
                        return Task::batch([log_task, minimize_task, version_presence_task]);
                    }
                }

                return Task::batch([log_task, version_presence_task]);
            }
            Err(err) => self.set_error(err),
        }
        Task::none()
    }

    fn kill_selected_instance(&mut self) -> Task<Message> {
        let Some(instance) = &self.selected_instance else {
            return Task::none();
        };
        match instance.kind {
            InstanceKind::Client => {
                if let Some(process) = self.processes.remove(instance) {
                    let mut child = block_on(process.child.child.lock());
                    _ = child.start_kill();
                }
            }
            InstanceKind::Server => {
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
                    }
                }
            }
        }
        Task::none()
    }

    pub fn update_main_menu(&mut self, msg: MainMenuMessage) -> Task<Message> {
        match msg {
            MainMenuMessage::ChangeTab(tab) => {
                // UX tweak: dragging instance to tab will open tab for that instance
                if let State::Launch(MenuLaunch { modal, .. }) = &mut self.state {
                    if let Some(LaunchModal::SDragging {
                        being_dragged: SidebarSelection::Instance(name, kind),
                        ..
                    }) = modal
                    {
                        if self.selected_instance.is_none() {
                            self.selected_instance = Some(Instance::new(name, *kind));
                        }
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
                if let State::Launch(MenuLaunch { sidebar_scroll, .. }) = &mut self.state {
                    *sidebar_scroll = scroll;
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
                if let State::Launch(MenuLaunch {
                    modal: Some(LaunchModal::SDragging { being_dragged, .. }),
                    ..
                }) = &mut self.state
                {
                    self.config.c_sidebar().drag_drop(being_dragged, location);
                }
                self.sidebar_update_state();
            }
            SidebarMessage::DragHover { location, entered } => {
                if let State::Launch(MenuLaunch {
                    modal: Some(LaunchModal::SDragging { dragged_to, .. }),
                    ..
                }) = &mut self.state
                {
                    if entered {
                        *dragged_to = Some(location);
                    } else if dragged_to.as_ref().is_some_and(|n| *n == location) {
                        *dragged_to = None;
                    }
                }
            }
            SidebarMessage::FolderRenameConfirm => {
                if let State::Launch(MenuLaunch {
                    modal: Some(LaunchModal::SRenamingFolder(id, name, _)),
                    ..
                }) = &self.state
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
