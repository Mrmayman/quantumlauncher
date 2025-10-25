use iced::{futures::executor::block_on, Task};
use ql_core::{
    err, err_no_log, file_utils::DirItem, info_no_log, open_file_explorer, InstanceSelection,
    IntoIoError, IntoStringError, LOGGER,
};
use ql_instances::UpdateCheckInfo;
use std::fmt::Write;
use tokio::io::AsyncWriteExt;

use crate::state::{
    CustomJarState, GameProcess, LaunchTabId, Launcher, ManageModsMessage, MenuExportInstance,
    MenuLaunch, MenuLauncherUpdate, MenuLicense, MenuServerCreate, MenuWelcome, Message,
    ProgressBar, State,
};

impl Launcher {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing | Message::CoreCleanComplete(Ok(())) => {}
            Message::Multiple(msgs) => {
                let mut task = Task::none();
                for msg in msgs {
                    task = task.chain(self.update(msg));
                }
                return task;
            }

            Message::CoreTryQuit => {
                let safe_to_exit = self.processes.is_empty()
                    && (self.key_escape_back(false).0 || matches!(self.state, State::Launch(_)));

                if safe_to_exit {
                    info_no_log!("CTRL-Q pressed, closing launcher...");
                    std::process::exit(1);
                }
            }

            Message::CoreTickConfigSaved(result) | Message::UpdateDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }

            Message::UpdateCheckResult(Err(err)) | Message::CoreCleanComplete(Err(err)) => {
                err_no_log!("{err}");
            }

            Message::ServerCreateEnd(Err(err))
            | Message::ServerCreateVersionsLoaded(Err(err))
            | Message::UninstallLoaderEnd(Err(err))
            | Message::InstallForgeEnd(Err(err))
            | Message::LaunchGameExited(Err(err))
            | Message::CoreListLoaded(Err(err)) => self.set_error(err),

            Message::WelcomeContinueToTheme => {
                self.state = State::Welcome(MenuWelcome::P2Theme);
            }
            Message::WelcomeContinueToAuth => {
                self.state = State::Welcome(MenuWelcome::P3Auth);
            }

            Message::Account(msg) => return self.update_account(msg),
            Message::ManageMods(message) => return self.update_manage_mods(message),
            Message::ExportMods(message) => return self.update_export_mods(message),
            Message::ManageJarMods(message) => return self.update_manage_jar_mods(message),
            Message::RecommendedMods(message) => return self.update_recommended_mods(message),
            Message::LaunchInstanceSelected { name, is_server } => {
                self.selected_instance = Some(InstanceSelection::new(&name, is_server));
                self.load_edit_instance(None);
            }
            Message::LauncherSettings(msg) => return self.update_launcher_settings(msg),
            Message::InstallOptifine(msg) => return self.update_install_optifine(msg),
            Message::InstallPaper(msg) => return self.update_install_paper(msg),

            Message::LaunchUsernameSet(username) => {
                self.config.username = username;
            }
            Message::LaunchStart => return self.launch_start(),
            Message::LaunchEnd(result) => return self.finish_launching(result),
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => self.go_to_delete_instance_menu(),
            Message::DeleteInstance => return self.delete_instance_confirm(),

            Message::LaunchScreenOpen {
                message,
                clear_selection,
            } => {
                if clear_selection {
                    self.selected_instance = None;
                }
                return self.go_to_launch_screen(message);
            }
            Message::EditInstance(message) => match self.update_edit_instance(message) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::InstallFabric(message) => return self.update_install_fabric(message),
            Message::CoreOpenLink(dir) => open_file_explorer(&dir),
            Message::CoreOpenPath(dir) => {
                if !dir.exists() && dir.to_string_lossy().contains("jarmods") {
                    _ = std::fs::create_dir_all(&dir);
                }
                open_file_explorer(&dir)
            }
            Message::CoreCopyError => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("(QuantumLauncher): {error}"));
                }
            }
            Message::CoreCopyLog => {
                let text = {
                    if let Some(logger) = LOGGER.as_ref() {
                        let logger = logger.lock().unwrap();
                        logger.text.clone()
                    } else {
                        Vec::new()
                    }
                };

                let mut log = String::new();
                for (line, kind) in text {
                    _ = writeln!(log, "{kind} {line}");
                }
                return iced::clipboard::write(format!("QuantumLauncher Log:\n{log}"));
            }
            Message::CoreImageDownloaded(res) => match res {
                Ok(image) => {
                    self.images.insert_image(image);
                }
                Err(err) => {
                    err!("Could not download image: {err}");
                }
            },
            Message::CoreTick => {
                self.tick_timer = self.tick_timer.wrapping_add(1);
                let mut tasks = self.images.get_imgs_to_load();
                let command = self.tick();
                tasks.push(command);

                if self
                    .custom_jar
                    .as_ref()
                    .and_then(|n| n.recv.try_recv().ok())
                    .is_some()
                {
                    tasks.push(CustomJarState::load());
                }

                return Task::batch(tasks);
            }
            Message::UninstallLoaderStart => {
                let instance = self.instance().clone();
                return Task::perform(
                    ql_mod_manager::loaders::uninstall_loader(instance),
                    Message::UninstallLoaderEnd,
                );
            }
            Message::InstallForge(kind) => {
                return self.install_forge(kind);
            }
            Message::InstallForgeEnd(Ok(())) | Message::UninstallLoaderEnd(Ok(())) => {
                return self.go_to_edit_mods_menu(false);
            }
            Message::LaunchGameExited(Ok((status, instance))) => {
                self.set_game_exited(status, &instance);
            }
            Message::LaunchKill => return self.kill_selected_instance(),
            Message::LaunchCopyLog => {
                let instance = self.instance();
                if let Some(log) = self.logs.get(instance) {
                    return iced::clipboard::write(log.log.join(""));
                }
            }
            Message::LaunchUploadLog => {
                if let State::Launch(menu) = &mut self.state {
                    menu.is_uploading_mclogs = true;
                }

                let instance = self.instance();

                if let Some(log) = self.logs.get(instance) {
                    let log_content = log.log.join("");
                    if !log_content.trim().is_empty() {
                        return Task::perform(
                            crate::mclog_upload::upload_log(log_content),
                            |res| Message::LaunchUploadLogResult(res.strerr()),
                        );
                    }
                }
            }
            Message::LaunchUploadLogResult(result) => match result {
                Ok(url) => {
                    self.state = State::LogUploadResult { url };
                }
                Err(error) => {
                    self.state = State::Error {
                        error: format!("Failed to upload log: {error}"),
                    };
                }
            },
            #[cfg(feature = "auto_update")]
            Message::UpdateCheckResult(Ok(info)) => match info {
                UpdateCheckInfo::UpToDate => {
                    info_no_log!("Launcher is latest version. No new updates");
                }
                UpdateCheckInfo::NewVersion { url } => {
                    self.state = State::UpdateFound(MenuLauncherUpdate {
                        url,
                        progress: None,
                    });
                }
            },
            #[cfg(feature = "auto_update")]
            Message::UpdateDownloadStart => return self.update_download_start(),
            #[cfg(not(feature = "auto_update"))]
            Message::UpdateDownloadStart | Message::UpdateCheckResult(_) => return Task::none(),

            Message::ServerManageOpen {
                selected_server,
                message,
            } => {
                self.selected_instance = selected_server.map(InstanceSelection::Server);
                return self.go_to_server_manage_menu(message);
            }
            Message::ServerCreateScreenOpen => {
                if let Some(cache) = &self.server_version_list_cache {
                    self.state = State::ServerCreate(MenuServerCreate::Loaded {
                        name: String::new(),
                        versions: Box::new(iced::widget::combo_box::State::new(cache.clone())),
                        selected_version: None,
                    });
                } else {
                    self.state = State::ServerCreate(MenuServerCreate::LoadingList);

                    return Task::perform(
                        async move { ql_servers::list().await.strerr() },
                        Message::ServerCreateVersionsLoaded,
                    );
                }
            }
            Message::ServerCreateNameInput(new_name) => {
                if let State::ServerCreate(MenuServerCreate::Loaded { name, .. }) = &mut self.state
                {
                    *name = new_name;
                }
            }
            Message::ServerCreateVersionSelected(list_entry) => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    selected_version, ..
                }) = &mut self.state
                {
                    *selected_version = Some(list_entry);
                }
            }
            Message::ServerCreateStart => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    name,
                    selected_version: Some(selected_version),
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    let name = name.clone();
                    let selected_version = selected_version.clone();
                    self.state = State::ServerCreate(MenuServerCreate::Downloading {
                        progress: ProgressBar::with_recv(receiver),
                    });
                    return Task::perform(
                        async move {
                            let sender = sender;
                            ql_servers::create_server(name, selected_version, Some(&sender))
                                .await
                                .strerr()
                        },
                        Message::ServerCreateEnd,
                    );
                }
            }
            Message::ServerCreateEnd(Ok(name)) => {
                self.selected_instance = Some(InstanceSelection::Server(name));
                return self.go_to_server_manage_menu(Some("Created Server".to_owned()));
            }
            Message::ServerCreateVersionsLoaded(Ok(vec)) => {
                self.server_version_list_cache = Some(vec.clone());
                if let State::ServerCreate(_) = &self.state {
                    self.state = State::ServerCreate(MenuServerCreate::Loaded {
                        versions: Box::new(iced::widget::combo_box::State::new(vec)),
                        selected_version: None,
                        name: String::new(),
                    });
                }
            }
            Message::ServerCommandEdit(command) => {
                let server = self.selected_instance.as_ref().unwrap();
                debug_assert!(server.is_server());
                if let Some(log) = self.logs.get_mut(server) {
                    log.command = command;
                }
            }
            Message::ServerCommandSubmit => {
                let server = self.selected_instance.as_ref().unwrap();
                debug_assert!(server.is_server());
                if let (
                    Some(log),
                    Some(GameProcess {
                        server_input: Some((stdin, _)),
                        ..
                    }),
                ) = (self.logs.get_mut(server), self.processes.get_mut(server))
                {
                    let log_cloned = format!("{}\n", log.command);
                    let future = stdin.write_all(log_cloned.as_bytes());
                    // Make the input command visible in the log
                    log.log.push(format!("> {}", log.command));

                    log.command.clear();
                    _ = block_on(future);
                }
            }
            Message::CoreListLoaded(Ok((list, is_server))) => {
                if is_server {
                    self.server_list = Some(list);
                } else {
                    self.client_list = Some(list);
                }
            }
            Message::CoreCopyText(txt) => {
                return iced::clipboard::write(txt);
            }
            Message::InstallMods(msg) => return self.update_install_mods(msg),
            Message::CoreOpenChangeLog => {
                self.state = State::ChangeLog;
            }
            Message::CoreOpenIntro => {
                self.state = State::Welcome(MenuWelcome::P1InitialScreen);
            }
            Message::EditPresets(msg) => return self.update_edit_presets(msg),
            Message::UninstallLoaderConfirm(msg, name) => {
                self.state = State::ConfirmAction {
                    msg1: format!("uninstall {name}"),
                    msg2: "This should be fine, you can always reinstall it later".to_owned(),
                    yes: Message::Multiple(vec![
                        Message::ShowScreen("Uninstalling...".to_owned()),
                        (*msg).clone(),
                    ]),
                    no: Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate),
                }
            }
            Message::ShowScreen(msg) => {
                self.state = State::GenericMessage(msg);
            }
            Message::CoreEvent(event, status) => return self.iced_event(event, status),
            Message::LaunchChangeTab(launch_tab_id) => {
                self.load_edit_instance(Some(launch_tab_id));
            }
            Message::CoreLogToggle => {
                self.is_log_open = !self.is_log_open;
            }
            Message::CoreLogScroll(lines) => {
                let new_scroll = self.log_scroll - lines;
                if new_scroll >= 0 {
                    self.log_scroll = new_scroll;
                }
            }
            Message::CoreLogScrollAbsolute(lines) => {
                self.log_scroll = lines;
            }
            Message::LaunchLogScroll(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    let new_scroll = *log_scroll - lines;
                    if new_scroll >= 0 {
                        *log_scroll = new_scroll;
                    }
                }
            }
            Message::LaunchLogScrollAbsolute(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    *log_scroll = lines;
                }
            }
            Message::LaunchScrollSidebar(total) => {
                if let State::Launch(MenuLaunch { sidebar_height, .. }) = &mut self.state {
                    *sidebar_height = total;
                }
            }

            Message::ExportInstanceOpen => {
                self.state = State::ExportInstance(MenuExportInstance {
                    entries: None,
                    progress: None,
                });
                return Task::perform(
                    ql_core::file_utils::read_filenames_from_dir(
                        self.selected_instance
                            .clone()
                            .unwrap()
                            .get_dot_minecraft_path(),
                    ),
                    |n| Message::ExportInstanceLoaded(n.strerr()),
                );
            }
            Message::ExportInstanceLoaded(res) => {
                let mut entries: Vec<(DirItem, bool)> = match res {
                    Ok(n) => n
                        .into_iter()
                        .map(|n| {
                            let enabled = !(n.name == ".fabric"
                                || n.name == "logs"
                                || n.name == "command_history.txt"
                                || n.name == "realms_persistence.json"
                                || n.name == "debug"
                                || n.name == ".cache"
                                // Common mods...
                                || n.name == "authlib-injector.log"
                                || n.name == "easy_npc"
                                || n.name == "CustomSkinLoader"
                                || n.name == ".bobby");
                            (n, enabled)
                        })
                        .filter(|(n, _)| {
                            !(n.name == "mod_index.json" || n.name == "launcher_profiles.json")
                        })
                        .collect(),
                    Err(err) => {
                        self.set_error(err);
                        return Task::none();
                    }
                };
                entries.sort_by(|(a, _), (b, _)| {
                    // Folders before files, and then sorted alphabetically
                    a.is_file.cmp(&b.is_file).then_with(|| a.name.cmp(&b.name))
                });
                if let State::ExportInstance(menu) = &mut self.state {
                    menu.entries = Some(entries);
                }
            }
            Message::ExportInstanceToggleItem(idx, t) => {
                if let State::ExportInstance(MenuExportInstance {
                    entries: Some(entries),
                    ..
                }) = &mut self.state
                {
                    if let Some((_, b)) = entries.get_mut(idx) {
                        *b = t;
                    }
                }
            }
            Message::ExportInstanceStart => {
                if let State::ExportInstance(MenuExportInstance {
                    entries: Some(entries),
                    progress,
                }) = &mut self.state
                {
                    let (send, recv) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv(recv));

                    let exceptions = entries
                        .iter()
                        .filter_map(|(n, b)| (!b).then_some(format!(".minecraft/{}", n.name)))
                        .collect();

                    return Task::perform(
                        ql_packager::export_instance(
                            self.selected_instance.clone().unwrap(),
                            exceptions,
                            Some(send),
                        ),
                        |n| Message::ExportInstanceFinished(n.strerr()),
                    );
                }
            }
            Message::ExportInstanceFinished(res) => match res {
                Ok(bytes) => {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        if let Err(err) = std::fs::write(&path, bytes).path(path) {
                            self.set_error(err);
                        } else {
                            return self.go_to_launch_screen(None::<String>);
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::LicenseOpen => {
                self.go_to_licenses_menu();
            }
            Message::LicenseChangeTab(tab) => {
                self.go_to_licenses_menu();
                if let State::License(menu) = &mut self.state {
                    menu.selected_tab = tab;
                    menu.content = iced::widget::text_editor::Content::with_text(tab.get_text());
                }
            }
            Message::LicenseAction(action) => {
                match action {
                    // Stop anyone from editing the license text
                    iced::widget::text_editor::Action::Edit(_) => {}
                    // Allow all other actions (movement, selection, clicking, scrolling, etc.)
                    _ => {
                        if let State::License(menu) = &mut self.state {
                            menu.content.perform(action);
                        }
                    }
                }
            }
        }
        Task::none()
    }

    pub fn load_edit_instance(&mut self, new_tab: Option<LaunchTabId>) {
        if let State::Launch(MenuLaunch {
            tab, edit_instance, ..
        }) = &mut self.state
        {
            if let (LaunchTabId::Edit, Some(selected_instance)) =
                (new_tab.unwrap_or(*tab), self.selected_instance.as_ref())
            {
                if let Err(err) = Self::load_edit_instance_inner(edit_instance, selected_instance) {
                    err!("Could not open edit instance menu: {err}");
                    *edit_instance = None;
                }
            } else {
                *edit_instance = None;
            }
            if let Some(new_tab) = new_tab {
                *tab = new_tab;
            }
        }
    }

    fn go_to_licenses_menu(&mut self) {
        if let State::License(_) = self.state {
            return;
        }
        let selected_tab = crate::state::LicenseTab::Gpl3;
        self.state = State::License(MenuLicense {
            selected_tab,
            content: iced::widget::text_editor::Content::with_text(selected_tab.get_text()),
        });
    }
}
