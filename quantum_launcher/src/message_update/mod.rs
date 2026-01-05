use std::path::Path;

use frostmark::MarkState;
use iced::futures::executor::block_on;
use iced::widget::text_editor;
use iced::{widget::scrollable::AbsoluteOffset, Task};
use ql_core::{
    err, err_no_log, InstanceSelection, IntoStringError, Loader, ModId, OptifineUniqueVersion,
};
use ql_mod_manager::{
    loaders,
    store::{get_description, QueryType},
};

mod accounts;
mod create_instance;
mod edit_instance;
mod manage_mods;
mod presets;
mod recommended;

use crate::config::UiWindowDecorations;
use crate::state::{InstanceNotes, MenuLaunch, ModOperation, NotesMessage};
use crate::{
    config::UiSettings,
    state::{
        self, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage,
        InstallPaperMessage, Launcher, LauncherSettingsMessage, MenuCurseforgeManualDownload,
        MenuInstallFabric, MenuInstallOptifine, MenuInstallPaper, MenuModsDownload, Message,
        ProgressBar, State,
    },
};

pub const MSG_RESIZE: &str = "Resize your window to apply the changes.";

impl Launcher {
    pub fn update_install_fabric(&mut self, message: InstallFabricMessage) -> Task<Message> {
        match message {
            InstallFabricMessage::End(result) => match result {
                Ok(()) => return self.go_to_edit_mods_menu(false),
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::VersionSelected(selection) => {
                if let State::InstallFabric(MenuInstallFabric::Loaded { fabric_version, .. }) =
                    &mut self.state
                {
                    *fabric_version = selection;
                }
            }
            InstallFabricMessage::VersionsLoaded(result) => match result {
                Ok(list) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        let (regular_list, backend) = list.clone().just_get_one();
                        *menu = if let (false, Some(first)) =
                            (list.is_unsupported(), regular_list.first())
                        {
                            MenuInstallFabric::Loaded {
                                backend,
                                fabric_version: first.loader.version.clone(),
                                fabric_versions: list,
                                progress: None,
                            }
                        } else {
                            MenuInstallFabric::Unsupported(menu.is_quilt())
                        };
                    }
                }
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::ChangeBackend(b) => {
                if let State::InstallFabric(MenuInstallFabric::Loaded {
                    backend,
                    fabric_version,
                    fabric_versions,
                    ..
                }) = &mut self.state
                {
                    *backend = b;
                    if let Some(n) = fabric_versions
                        .clone()
                        .get_specific(b)
                        .and_then(|n| n.first().cloned())
                    {
                        *fabric_version = n.loader.version;
                    }
                }
            }
            InstallFabricMessage::ButtonClicked => {
                if let State::InstallFabric(MenuInstallFabric::Loaded {
                    fabric_version,
                    progress,
                    backend,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv(receiver));
                    let loader_version = fabric_version.clone();

                    let instance_name = self.selected_instance.clone().unwrap();
                    let backend = *backend;
                    return Task::perform(
                        async move {
                            loaders::fabric::install(
                                Some(loader_version),
                                instance_name,
                                Some(&sender),
                                backend,
                            )
                            .await
                        },
                        |m| Message::InstallFabric(InstallFabricMessage::End(m.strerr())),
                    );
                }
            }
            InstallFabricMessage::ScreenOpen { is_quilt } => {
                let instance_name = self.selected_instance.clone().unwrap();
                let (task, handle) = Task::perform(
                    loaders::fabric::get_list_of_versions(instance_name, is_quilt),
                    |m| Message::InstallFabric(InstallFabricMessage::VersionsLoaded(m.strerr())),
                )
                .abortable();

                self.state = State::InstallFabric(MenuInstallFabric::Loading {
                    is_quilt,
                    _loading_handle: handle.abort_on_drop(),
                });

                return task;
            }
        }
        Task::none()
    }

    pub fn update_install_mods(&mut self, message: InstallModsMessage) -> Task<Message> {
        let is_server = matches!(&self.selected_instance, Some(InstanceSelection::Server(_)));

        match message {
            InstallModsMessage::LoadData(Err(err))
            | InstallModsMessage::DownloadComplete(Err(err))
            | InstallModsMessage::SearchResult(Err(err))
            | InstallModsMessage::IndexUpdated(Err(err))
            | InstallModsMessage::UninstallComplete(Err(err)) => {
                self.set_error(err);
            }

            InstallModsMessage::SearchResult(Ok(search)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.is_loading_continuation = false;
                    menu.has_continuation_ended = search.reached_end;

                    if search.start_time > menu.latest_load {
                        menu.latest_load = search.start_time;

                        if let (Some(results), true) = (&mut menu.results, search.offset > 0) {
                            results.mods.extend(search.mods);
                        } else {
                            menu.results = Some(search);
                        }
                    }
                }
            }
            InstallModsMessage::Scrolled(viewport) => {
                let total_height =
                    viewport.content_bounds().height - (viewport.bounds().height * 2.0);
                let absolute_offset = viewport.absolute_offset();
                let scroll_px = absolute_offset.y;

                if let State::ModsDownload(menu) = &mut self.state {
                    if menu.results.is_none() {
                        menu.has_continuation_ended = false;
                    }

                    menu.scroll_offset = absolute_offset;
                    if (scroll_px > total_height)
                        && !menu.is_loading_continuation
                        && !menu.has_continuation_ended
                    {
                        menu.is_loading_continuation = true;

                        let offset = if let Some(results) = &menu.results {
                            results.offset + results.mods.len()
                        } else {
                            0
                        };
                        return menu.search_store(is_server, offset);
                    }
                }
            }
            InstallModsMessage::Open => match self.open_mods_store() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            InstallModsMessage::TickDesc(update_msg) => {
                if let State::ModsDownload(MenuModsDownload {
                    description: Some(description),
                    ..
                }) = &mut self.state
                {
                    description.update(update_msg);
                }
            }
            InstallModsMessage::SearchInput(input) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query = input;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::Click(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    menu.reload_description(&mut self.images);
                    if let Some(results) = &menu.results {
                        let hit = results.mods.get(i).unwrap();
                        if !menu
                            .mod_descriptions
                            .contains_key(&ModId::from_pair(&hit.id, results.backend))
                        {
                            let backend = menu.backend;
                            let id = ModId::from_pair(&hit.id, backend);

                            return Task::perform(get_description(id), |n| {
                                Message::InstallMods(InstallModsMessage::LoadData(n.strerr()))
                            });
                        }
                    }
                }
            }
            InstallModsMessage::BackToMainScreen => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                    menu.description = None;
                    return iced::widget::scrollable::scroll_to(
                        iced::widget::scrollable::Id::new("MenuModsDownload:main:mods_list"),
                        menu.scroll_offset,
                    );
                }
            }
            InstallModsMessage::LoadData(Ok((id, description))) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_descriptions.insert(id, description);
                    menu.reload_description(&mut self.images);
                }
            }
            InstallModsMessage::Download(index) => {
                return self.mod_download(index);
            }
            InstallModsMessage::DownloadComplete(Ok((id, not_allowed))) => {
                let task = if let State::ModsDownload(menu) = &mut self.state {
                    menu.mods_download_in_progress.remove(&id);
                    Task::none()
                } else {
                    match self.open_mods_store() {
                        Ok(n) => n,
                        Err(err) => {
                            self.set_error(err);
                            Task::none()
                        }
                    }
                };

                if not_allowed.is_empty() {
                    return task;
                }
                self.state = State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                    unsupported: not_allowed,
                    is_store: true,
                    delete_mods: true,
                });
            }
            InstallModsMessage::IndexUpdated(Ok(idx)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_index = idx;
                }
            }

            InstallModsMessage::ChangeBackend(backend) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.backend = backend;
                    menu.results = None;
                    menu.scroll_offset = AbsoluteOffset::default();
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ChangeQueryType(query) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query_type = query;
                    menu.results = None;
                    menu.scroll_offset = AbsoluteOffset::default();
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::InstallModpack(id) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

                let selected_instance = self.selected_instance.clone().unwrap();
                self.mod_updates_checked.remove(&selected_instance);

                return Task::perform(
                    async move {
                        ql_mod_manager::store::download_mod(&id, &selected_instance, Some(sender))
                            .await
                            .map(|not_allowed| (id, not_allowed))
                    },
                    |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
                );
            }
            InstallModsMessage::Uninstall(index) => {
                let State::ModsDownload(MenuModsDownload {
                    results: Some(results),
                    mods_download_in_progress,
                    ..
                }) = &mut self.state
                else {
                    return Task::none();
                };
                let Some(hit) = results.mods.get(index) else {
                    err!("Couldn't uninstall mod: Index out of range");
                    return Task::none();
                };

                let mod_id = ModId::from_pair(&hit.id, results.backend);
                mods_download_in_progress
                    .insert(mod_id.clone(), (hit.title.clone(), ModOperation::Deleting));
                let selected_instance = self.instance().clone();

                return Task::perform(
                    ql_mod_manager::store::delete_mods(vec![mod_id], selected_instance),
                    |n| Message::InstallMods(InstallModsMessage::UninstallComplete(n.strerr())),
                );
            }
            InstallModsMessage::UninstallComplete(Ok(ids)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    for id in ids {
                        menu.mods_download_in_progress.remove(&id);
                        menu.mod_index.mods.remove(&id.get_index_str());
                    }
                }
            }
        }
        Task::none()
    }

    fn mod_download(&mut self, index: usize) -> Task<Message> {
        let selected_instance = self.instance().clone();
        let State::ModsDownload(menu) = &mut self.state else {
            return Task::none();
        };
        let Some(results) = &menu.results else {
            err!("Couldn't download mod: Search results empty");
            return Task::none();
        };
        let Some(hit) = results.mods.get(index) else {
            err!("Couldn't download mod: Not present in results");
            return Task::none();
        };

        menu.mods_download_in_progress.insert(
            ModId::from_pair(&hit.id, results.backend),
            (hit.title.clone(), ModOperation::Downloading),
        );

        let project_id = hit.id.clone();
        let backend = menu.backend;
        let id = ModId::from_pair(&project_id, backend);

        if let QueryType::ModPacks = menu.query_type {
            self.state = State::ConfirmAction {
                msg1: format!("install the modpack: {}", hit.title),
                msg2: "This might take a while, install many files, and use a lot of network..."
                    .to_owned(),
                yes: Message::InstallMods(InstallModsMessage::InstallModpack(id)),
                no: Message::InstallMods(InstallModsMessage::Open),
            };
            Task::none()
        } else {
            Task::perform(
                async move {
                    ql_mod_manager::store::download_mod(&id, &selected_instance, None)
                        .await
                        .map(|not_allowed| (ModId::Modrinth(project_id), not_allowed))
                },
                |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
            )
        }
    }

    pub fn update_install_optifine(&mut self, message: InstallOptifineMessage) -> Task<Message> {
        match message {
            InstallOptifineMessage::ScreenOpen => {
                let is_forge_installed = if let State::EditMods(menu) = &self.state {
                    menu.config.mod_type == Loader::Forge
                } else {
                    false
                };
                let optifine_unique_version = if is_forge_installed {
                    Some(OptifineUniqueVersion::Forge)
                } else {
                    block_on(OptifineUniqueVersion::get(self.instance()))
                };

                if let Some(version @ OptifineUniqueVersion::B1_7_3) = optifine_unique_version {
                    self.state = State::InstallOptifine(MenuInstallOptifine::InstallingB173);

                    let selected_instance = self.selected_instance.clone().unwrap();
                    let url = version.get_url().0;
                    return Task::perform(
                        loaders::optifine::install_b173(selected_instance, url),
                        |n| Message::InstallOptifine(InstallOptifineMessage::End(n.strerr())),
                    );
                }

                self.state = State::InstallOptifine(MenuInstallOptifine::Choosing {
                    optifine_unique_version,
                    delete_installer: true,
                    drag_and_drop_hovered: false,
                });
            }
            InstallOptifineMessage::DeleteInstallerToggle(t) => {
                if let State::InstallOptifine(MenuInstallOptifine::Choosing {
                    delete_installer,
                    ..
                }) = &mut self.state
                {
                    *delete_installer = t;
                }
            }
            InstallOptifineMessage::SelectInstallerStart => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("jar/zip", &["jar", "zip"])
                    .set_title("Select OptiFine Installer")
                    .pick_file()
                {
                    return self.install_optifine_confirm(&path);
                }
            }
            InstallOptifineMessage::End(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self.go_to_edit_mods_menu(false);
                }
            }
        }
        Task::none()
    }

    pub fn install_optifine_confirm(&mut self, installer_path: &Path) -> Task<Message> {
        let (p_sender, p_recv) = std::sync::mpsc::channel();
        let (j_sender, j_recv) = std::sync::mpsc::channel();

        let instance = self.instance();
        let instance_name = instance.get_name().to_owned();
        debug_assert!(!instance.is_server());

        let optifine_unique_version =
            if let State::InstallOptifine(MenuInstallOptifine::Choosing {
                optifine_unique_version,
                ..
            }) = &self.state
            {
                *optifine_unique_version
            } else {
                block_on(OptifineUniqueVersion::get(instance))
            };

        let delete_installer = if let State::InstallOptifine(MenuInstallOptifine::Choosing {
            delete_installer,
            ..
        }) = &self.state
        {
            *delete_installer
        } else {
            false
        };

        self.state = State::InstallOptifine(MenuInstallOptifine::Installing {
            optifine_install_progress: ProgressBar::with_recv(p_recv),
            java_install_progress: Some(ProgressBar::with_recv(j_recv)),
            is_java_being_installed: false,
        });

        let installer_path = installer_path.to_owned();

        Task::perform(
            // OptiFine does not support servers
            // so it's safe to assume we've selected an instance.
            loaders::optifine::install(
                instance_name,
                installer_path.clone(),
                Some(p_sender),
                Some(j_sender),
                optifine_unique_version,
            ),
            |n| Message::InstallOptifine(InstallOptifineMessage::End(n.strerr())),
        )
        .chain(Task::perform(
            async move {
                if delete_installer
                    && installer_path.extension().is_some_and(|n| {
                        let n = n.to_ascii_lowercase();
                        n == "jar" || n == "zip"
                    })
                {
                    _ = tokio::fs::remove_file(installer_path).await;
                }
            },
            |()| Message::Nothing,
        ))
    }

    pub fn update_launcher_settings(&mut self, msg: LauncherSettingsMessage) -> Task<Message> {
        match msg {
            LauncherSettingsMessage::ThemePicked(theme) => {
                self.config.ui_mode = Some(theme);
                self.theme.lightness = theme;
            }
            LauncherSettingsMessage::Open => {
                self.go_to_launcher_settings();
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
                self.config
                    .ui
                    .get_or_insert_with(UiSettings::default)
                    .window_opacity = opacity;
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
                self.config
                    .ui
                    .get_or_insert_with(UiSettings::default)
                    .idle_fps = Some(fps as u64);
            }
            LauncherSettingsMessage::ClearJavaInstalls => {
                self.confirm_clear_java_installs();
            }
            LauncherSettingsMessage::ClearJavaInstallsConfirm => {
                return Task::perform(ql_instances::delete_java_installs(), |()| Message::Nothing);
            }
            LauncherSettingsMessage::ChangeTab(tab) => {
                self.go_to_launcher_settings();
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.selected_tab = tab;
                }
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
                    persistent.selected_server = None;
                }
            }
            LauncherSettingsMessage::DefaultMinecraftWidthChanged(input) => {
                self.config.c_global().window_width = input.trim().parse::<u32>().ok();
            }
            LauncherSettingsMessage::DefaultMinecraftHeightChanged(input) => {
                self.config.c_global().window_height = input.trim().parse::<u32>().ok();
            }
            LauncherSettingsMessage::GlobalJavaArgs(msg) => {
                let split = self.should_split_args();
                msg.apply(
                    self.config.extra_java_args.get_or_insert_with(Vec::new),
                    split,
                );
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefix(msg) => {
                let split = self.should_split_args();
                msg.apply(
                    self.config
                        .c_global()
                        .pre_launch_prefix
                        .get_or_insert_with(Vec::new),
                    split,
                );
            }
            LauncherSettingsMessage::ToggleWindowDecorations(b) => {
                let decor = if b {
                    UiWindowDecorations::default()
                } else {
                    UiWindowDecorations::System
                };
                self.config
                    .ui
                    .get_or_insert_with(UiSettings::default)
                    .window_decorations = decor;
            }
            LauncherSettingsMessage::LoadedSystemTheme(res) => match res {
                Ok(mode) => {
                    self.theme.system_dark_mode = mode == dark_light::Mode::Dark;
                }
                Err(err) if err.contains("Timeout reached") => {
                    // The system is just lagging, nothing we can do
                }
                Err(err) => {
                    err_no_log!("while loading system theme: {err}");
                }
            },
        }
        Task::none()
    }

    pub fn should_split_args(&self) -> bool {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &self.state
        {
            menu.arg_split_by_space
        } else if let State::LauncherSettings(menu) = &self.state {
            menu.arg_split_by_space
        } else {
            true
        }
    }

    fn confirm_clear_java_installs(&mut self) {
        self.state = State::ConfirmAction {
            msg1: "delete auto-installed Java files".to_owned(),
            msg2: "They will get reinstalled automatically as needed".to_owned(),
            yes: Message::LauncherSettings(LauncherSettingsMessage::ClearJavaInstallsConfirm),
            no: Message::LauncherSettings(LauncherSettingsMessage::ChangeTab(
                state::LauncherSettingsTab::Internal,
            )),
        }
    }

    pub fn go_to_launcher_settings(&mut self) {
        if let State::LauncherSettings(_) = &self.state {
            return;
        }
        self.state = State::LauncherSettings(state::MenuLauncherSettings {
            temp_scale: self.config.ui_scale.unwrap_or(1.0),
            selected_tab: state::LauncherSettingsTab::UserInterface,
            arg_split_by_space: true,
        });
    }

    pub fn update_install_paper(&mut self, msg: InstallPaperMessage) -> Task<Message> {
        match msg {
            InstallPaperMessage::VersionSelected(v) => {
                if let State::InstallPaper(MenuInstallPaper::Loaded { version, .. }) =
                    &mut self.state
                {
                    *version = v;
                }
            }
            InstallPaperMessage::VersionsLoaded(res) => match res {
                Ok(list) => {
                    let Some(version) = list.first().cloned() else {
                        self.set_error("No compatible Paper versions found");
                        return Task::none();
                    };
                    self.state = State::InstallPaper(MenuInstallPaper::Loaded {
                        version,
                        versions: list,
                    });
                }
                Err(err) => self.set_error(err),
            },
            InstallPaperMessage::ScreenOpen => {
                if let State::EditMods(menu) = &self.state {
                    let (task, handle) = Task::perform(
                        loaders::paper::get_list_of_versions(menu.version_json.get_id().to_owned()),
                        |n| Message::InstallPaper(InstallPaperMessage::VersionsLoaded(n.strerr())),
                    )
                    .abortable();
                    self.state = State::InstallPaper(MenuInstallPaper::Loading { _handle: handle });
                    return task;
                }
            }
            InstallPaperMessage::ButtonClicked => {
                let instance_name = self.instance().get_name().to_owned();
                let version =
                    if let State::InstallPaper(MenuInstallPaper::Loaded { version, .. }) =
                        &self.state
                    {
                        Some(version.clone())
                    } else {
                        None
                    };
                self.state = State::InstallPaper(MenuInstallPaper::Installing);
                return Task::perform(
                    loaders::paper::install(instance_name, version.into()),
                    |n| Message::InstallPaper(InstallPaperMessage::End(n.strerr())),
                );
            }
            InstallPaperMessage::End(res) => {
                if let Err(err) = res {
                    self.set_error(err);
                } else {
                    return self.go_to_edit_mods_menu(false);
                }
            }
        }
        Task::none()
    }

    /*pub fn update_window_msg(&mut self, msg: WindowMessage) -> Task<Message> {
        match msg {
            WindowMessage::Dragged => iced::window::get_latest().and_then(iced::window::drag),
            // WindowMessage::Resized(dir) => {
            //     return iced::window::get_latest()
            //         .and_then(move |id| iced::window::drag_resize(id, dir));
            // }
            WindowMessage::ClickMinimize => {
                iced::window::get_latest().and_then(|id| iced::window::minimize(id, true))
            }
            WindowMessage::ClickMaximize => iced::window::get_latest().and_then(|id| {
                iced::window::get_maximized(id)
                    .map(Some)
                    .and_then(move |max| iced::window::maximize(id, !max))
            }),
            WindowMessage::ClickClose => std::process::exit(0),
            // WindowMessage::IsMaximized(n) => {
            //     self.window_state.is_maximized = n;
            //     Task::none()
            // }
        }
    }*/

    pub fn update_notes(&mut self, msg: NotesMessage) -> Task<Message> {
        match msg {
            NotesMessage::Loaded(res) => match res {
                Ok(notes) => {
                    if let State::Launch(menu) = &mut self.state {
                        let mark_state = MarkState::with_html_and_markdown(&notes);
                        menu.notes = Some(InstanceNotes::Viewing {
                            content: notes,
                            mark_state,
                        });
                    }
                }
                Err(err) => err_no_log!("While loading instance notes: {err}"),
            },
            NotesMessage::OpenEdit => {
                if let State::Launch(MenuLaunch {
                    notes: Some(notes), ..
                }) = &mut self.state
                {
                    let content = notes.get_text();
                    *notes = InstanceNotes::Editing {
                        text_editor: text_editor::Content::with_text(content),
                        original: content.to_owned(),
                    };
                }
            }
            NotesMessage::Edit(action) => {
                if let State::Launch(MenuLaunch {
                    notes: Some(InstanceNotes::Editing { text_editor, .. }),
                    ..
                }) = &mut self.state
                {
                    text_editor.perform(action);
                }
            }
            NotesMessage::SaveEdit => {
                if let State::Launch(MenuLaunch {
                    notes: Some(notes), ..
                }) = &mut self.state
                {
                    if let InstanceNotes::Editing { text_editor, .. } = notes {
                        let content = text_editor.text();

                        *notes = InstanceNotes::Viewing {
                            mark_state: MarkState::with_html_and_markdown(&content),
                            content: content.clone(),
                        };

                        return Task::perform(
                            ql_instances::notes::write(self.instance().clone(), content),
                            |r| {
                                if let Err(err) = r {
                                    err_no_log!("While saving instance notes: {err}");
                                }
                                Message::Nothing
                            },
                        );
                    }
                }
            }
            NotesMessage::CancelEdit => {
                if let State::Launch(MenuLaunch {
                    notes: Some(notes), ..
                }) = &mut self.state
                {
                    let content = notes.get_text();
                    *notes = InstanceNotes::Viewing {
                        mark_state: MarkState::with_html_and_markdown(content),
                        content: content.to_owned(),
                    }
                }
            }
        }
        Task::none()
    }
}
