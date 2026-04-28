use iced::{Task, futures::executor::block_on};
use ql_core::{InstanceKind, IntoIoError, IntoStringError, err, file_utils::DirItem, info};
use std::fmt::Write;
use tokio::io::AsyncWriteExt;

#[allow(unused)]
use owo_colors::OwoColorize;

#[cfg(feature = "auto_update")]
use crate::launcher_update::UpdateCheckInfo;
use crate::{
    state::{
        AutoSaveKind, CustomJarState, DirWatcher, GameProcess, InfoMessage, LaunchTab, Launcher,
        LauncherSettingsMessage, ManageModsMessage, MenuExportInstance, MenuLaunch, MenuLicense,
        MenuWelcome, Message, ProgressBar, State, dir_watch, get_entries,
    },
    stylesheet::styles::LauncherThemeLightness,
};

impl Launcher {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing | Message::CoreCleanComplete(Ok(())) => {}
            Message::Error(err) => self.set_error(err),
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
                    info!(no_log, "CTRL-Q pressed, closing launcher...");
                    std::process::exit(1);
                }
            }

            Message::CoreCleanComplete(Err(err)) => {
                err!(no_log, "{err}");
            }

            Message::UninstallLoaderEnd(Err(err))
            | Message::InstallForgeEnd(Err(err))
            | Message::LaunchGameExited(Err(err))
            | Message::CoreListLoaded(Err(err)) => self.set_error(err),

            Message::WelcomeContinueToTheme => {
                self.state = State::Welcome(MenuWelcome::P2Theme);
            }
            Message::WelcomeContinueToAuth => {
                self.state = State::Welcome(MenuWelcome::P3Auth);
            }

            Message::MainMenu(msg) => return self.update_main_menu(msg),
            Message::Sidebar(msg) => return self.update_sidebar(msg),
            Message::Account(msg) => return self.update_account(msg),
            Message::ManageMods(msg) => return self.update_manage_mods(msg),
            Message::ExportMods(msg) => return self.update_export_mods(msg),
            Message::ManageJarMods(msg) => return self.update_manage_jar_mods(msg),
            Message::RecommendedMods(msg) => return self.update_recommended_mods(msg),
            Message::Window(msg) => return self.update_window_msg(msg),
            Message::Notes(msg) => return self.update_notes(msg),
            Message::GameLog(msg) => return self.update_game_log(msg),
            Message::Shortcut(msg) => match self.update_shortcut(msg) {
                Ok(n) => return n,
                Err(e) => self.set_error(e),
            },

            Message::LauncherSettings(msg) => return self.update_launcher_settings(msg),
            Message::InstallOptifine(msg) => return self.update_install_optifine(msg),
            Message::InstallPaper(msg) => return self.update_install_paper(msg),
            Message::ModDescription(msg) => return self.update_mod_description(msg),

            Message::LaunchStart => return self.launch_start(),
            Message::LaunchEnd(result) => return self.finish_launching(result),
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => self.go_to_delete_instance_menu(),
            Message::DeleteInstance => return self.delete_instance_confirm(),

            Message::MScreenOpen {
                message,
                clear_selection,
            } => {
                if clear_selection {
                    self.unselect_instance();
                }
                return self.go_to_main_menu(message);
            }
            Message::EditInstance(message) => {
                if message.edits_config() {
                    self.autosave.remove(&AutoSaveKind::InstanceConfig);
                }
                match self.update_edit_instance(message) {
                    Ok(n) => return n,
                    Err(err) => self.set_error(err),
                }
            }
            Message::InstallFabric(message) => return self.update_install_fabric(message),
            Message::CoreOpenLink(dir) => _ = open::that_detached(&dir),
            Message::CoreOpenPath(dir) => {
                if !dir.exists() && dir.to_string_lossy().contains("jarmods") {
                    _ = std::fs::create_dir_all(&dir);
                }
                _ = open::that_detached(&dir);
            }
            Message::CoreCopyError => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("(QuantumLauncher): {error}"));
                }
            }
            Message::CoreCopyLog => {
                let text = ql_core::print::get();

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
                    err!(no_log, "Could not download image: {err}");
                }
            },
            Message::CoreTick => {
                self.tick_timer = self.tick_timer.wrapping_add(1);
                let mut tasks = self.images.task_get_imgs_to_load();
                tasks.push(self.tick());
                tasks.push(self.task_read_system_theme());

                // HOOK: Decorations
                // tasks.push(
                //     iced::window::get_latest()
                //         .and_then(iced::window::get_maximized)
                //         .map(|m| WindowMessage::IsMaximized(m).into()),
                // );

                let custom_jars_changed = self
                    .custom_jar
                    .as_ref()
                    .is_some_and(|n| n.watcher.has_changed());

                if custom_jars_changed {
                    tasks.push(CustomJarState::load());
                }

                let mut watch_reload = |watcher: Option<&DirWatcher>, kind| {
                    if watcher.is_some_and(DirWatcher::has_changed) {
                        tasks.push(Task::perform(get_entries(kind), Message::CoreListLoaded));
                    }
                };
                watch_reload(self.client_watcher.as_ref(), InstanceKind::Client);
                watch_reload(self.server_watcher.as_ref(), InstanceKind::Server);

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
            Message::InstallForgeEnd(Ok(())) => {
                return self.go_to_edit_mods_menu(Some(InfoMessage::success("Installed Forge")));
            }
            Message::UninstallLoaderEnd(Ok(())) => {
                return self.go_to_edit_mods_menu(Some(InfoMessage::success("Uninstalled loader")));
            }
            Message::LaunchGameExited(Ok((status, instance, diagnostic))) => {
                return self.set_game_exited(status, &instance, diagnostic);
            }
            Message::LaunchKill => return self.kill_selected_instance(),

            #[cfg(feature = "auto_update")]
            Message::UpdateCheckResult(res) => match res {
                Ok(UpdateCheckInfo::UpToDate) => {
                    self.config.update_set_now();
                    self.autosave.remove(&AutoSaveKind::LauncherConfig);
                    ql_core::pt!(no_log, "{}", "Latest version".bright_black());
                }
                Ok(UpdateCheckInfo::NewVersion { url }) => {
                    self.state = State::UpdateFound(crate::state::MenuLauncherUpdate {
                        url,
                        progress: None,
                    });
                }
                Err(err) => {
                    ql_core::pt!(no_log, "{}", err.bright_black());
                }
            },
            #[cfg(feature = "auto_update")]
            Message::UpdateDownloadStart => return self.update_download_start(),
            #[cfg(feature = "auto_update")]
            Message::UpdateDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(format!("Update installation failed! Try going to the website at\nmrmayman.github.io/quantumlauncher\nAnd download from there\n\n{err}"));
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
            Message::CoreListLoaded(Ok((list, kind))) => {
                self.core_list_loaded(list, kind);
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
                    no: ManageModsMessage::Open.into(),
                }
            }
            Message::ShowScreen(msg) => {
                self.state = State::GenericMessage(msg);
            }
            Message::CoreEvent(event, status) => return self.iced_event(event, status),
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
                            return self.go_to_main_menu(None);
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
            Message::CoreFocusNext => {
                return iced::widget::focus_next();
            }
            Message::CoreHideModal => {
                self.hide_submenu();
            }
        }
        Task::none()
    }

    fn core_list_loaded(&mut self, list: Vec<String>, kind: InstanceKind) {
        self.config.update_sidebar(&list, kind);
        self.autosave.remove(&AutoSaveKind::LauncherConfig);

        let persistent = self.config.c_persistent();

        let p_kind = persistent
            .selected_instance_kind
            .unwrap_or(InstanceKind::Client);
        if p_kind == kind
            && persistent
                .selected_instance
                .as_ref()
                .is_some_and(|p| !list.iter().any(|n| n == &**p))
        {
            // The previously selected instance no longer exists
            self.unselect_instance();
        }

        let (self_list, self_watcher) = match kind {
            InstanceKind::Client => (&mut self.client_list, &mut self.client_watcher),
            InstanceKind::Server => (&mut self.server_list, &mut self.server_watcher),
        };
        *self_list = Some(list);

        if self_watcher.is_none() {
            let dir = kind.get_root_directory();
            let watcher = match dir_watch(dir) {
                Ok(n) => n,
                Err(err) => {
                    err!("Couldn't start dir watcher! {err}");
                    return;
                }
            };
            *self_watcher = Some(watcher);
        }
    }

    fn task_read_system_theme(&mut self) -> Task<Message> {
        const INTERVAL: usize = 4;

        let is_auto_theme = self
            .config
            .ui_mode
            .is_none_or(|n| n == LauncherThemeLightness::Auto);
        #[allow(clippy::manual_is_multiple_of)] // Maintain Rust MSRV
        let interval = self.tick_timer % INTERVAL == 0;

        if is_auto_theme && interval {
            Task::perform(tokio::task::spawn_blocking(dark_light::detect), |n| {
                LauncherSettingsMessage::LoadedSystemTheme(
                    n.strerr().and_then(IntoStringError::strerr),
                )
                .into()
            })
        } else {
            Task::none()
        }
    }

    pub fn load_edit_instance(&mut self, new_tab: Option<LaunchTab>) {
        if let State::Launch(_) = &self.state {
        } else {
            _ = self.go_to_main_menu(None);
        }

        if let State::Launch(MenuLaunch {
            tab, edit_instance, ..
        }) = &mut self.state
        {
            if let (LaunchTab::Edit, Some(selected_instance)) =
                (new_tab.unwrap_or(*tab), self.selected_instance.as_ref())
            {
                self.autosave.insert(AutoSaveKind::InstanceConfig); // prevent it from saving *right now*
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
