use std::path::Path;
use std::str::FromStr;

use iced::futures::executor::block_on;
use iced::{widget::scrollable::AbsoluteOffset, Task};
use ql_core::{err, info, InstanceSelection, IntoStringError, ModId, OptifineUniqueVersion};
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

use crate::{
    state::{
        self, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage, Launcher,
        LauncherSettingsMessage, MenuCurseforgeManualDownload, MenuInstallFabric,
        MenuInstallOptifine, Message, ProgressBar, State,
    },
    stylesheet::styles::{LauncherThemeColor, LauncherThemeLightness},
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
            | InstallModsMessage::IndexUpdated(Err(err)) => {
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
            InstallModsMessage::SearchInput(input) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query = input;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::Click(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
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
                    return iced::widget::scrollable::scroll_to(
                        iced::widget::scrollable::Id::new("MenuModsDownload:main:mods_list"),
                        menu.scroll_offset,
                    );
                }
            }
            InstallModsMessage::LoadData(Ok((id, description))) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_descriptions.insert(id, description);
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
        }
        Task::none()
    }

    fn mod_download(&mut self, index: usize) -> Task<Message> {
        let Some(selected_instance) = self.selected_instance.clone() else {
            return Task::none();
        };
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

        menu.mods_download_in_progress
            .insert(ModId::Modrinth(hit.id.clone()), hit.title.clone());

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
                    menu.config.mod_type == "Forge"
                } else {
                    false
                };
                let optifine_unique_version = if is_forge_installed {
                    Some(OptifineUniqueVersion::Forge)
                } else {
                    block_on(OptifineUniqueVersion::get(
                        self.selected_instance.as_ref().unwrap(),
                    ))
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

        let instance = self.selected_instance.as_ref().unwrap();
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

        let get_name = self
            .selected_instance
            .as_ref()
            .unwrap()
            .get_name()
            .to_owned();

        let installer_path = installer_path.to_owned();

        Task::perform(
            // Note: OptiFine does not support servers
            // so it's safe to assume we've selected an instance.
            loaders::optifine::install(
                get_name,
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
                info!("Setting color mode {theme}");
                self.config.theme = Some(theme.clone());

                match theme.as_str() {
                    "Light" => self.theme.lightness = LauncherThemeLightness::Light,
                    "Dark" => self.theme.lightness = LauncherThemeLightness::Dark,
                    _ => err!("Invalid color mode {theme}"),
                }
            }
            LauncherSettingsMessage::Open => {
                self.go_to_launcher_settings();
            }
            LauncherSettingsMessage::ColorSchemePicked(color) => {
                info!("Setting color scheme {color}");
                self.config.style = Some(color.clone());
                self.theme.color = LauncherThemeColor::from_str(&color).unwrap_or_default();
            }
            LauncherSettingsMessage::UiScale(scale) => {
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.temp_scale = scale;
                }
            }
            LauncherSettingsMessage::UiScaleApply => {
                if let State::LauncherSettings(menu) = &self.state {
                    self.config.ui_scale = Some(menu.temp_scale);
                    self.state = State::GenericMessage(MSG_RESIZE.to_owned());
                }
            }
            LauncherSettingsMessage::ClearJavaInstalls => {
                self.state = State::ConfirmAction {
                    msg1: "delete auto-installed Java files".to_owned(),
                    msg2: "They will get reinstalled automatically as needed".to_owned(),
                    yes: Message::LauncherSettings(
                        LauncherSettingsMessage::ClearJavaInstallsConfirm,
                    ),
                    no: Message::LauncherSettings(LauncherSettingsMessage::ChangeTab(
                        state::LauncherSettingsTab::Internal,
                    )),
                }
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
                self.config.antialiasing = Some(t);
            }
            LauncherSettingsMessage::ToggleWindowSize(t) => {
                self.config
                    .window
                    .get_or_insert_with(Default::default)
                    .save_window_size = t;
            }
            LauncherSettingsMessage::DefaultMinecraftWidthChanged(input) => {
                self.config
                    .global_settings
                    .get_or_insert_with(Default::default)
                    .window_width = if input.trim().is_empty() {
                    None
                } else {
                    input.trim().parse::<u32>().ok()
                };
            }
            LauncherSettingsMessage::DefaultMinecraftHeightChanged(input) => {
                self.config
                    .global_settings
                    .get_or_insert_with(Default::default)
                    .window_height = if input.trim().is_empty() {
                    None
                } else {
                    input.trim().parse::<u32>().ok()
                };
            }
            LauncherSettingsMessage::GlobalJavaArgsAdd => {
                self.config
                    .extra_java_args
                    .get_or_insert_with(Vec::new)
                    .push(String::new());
            }
            LauncherSettingsMessage::GlobalJavaArgEdit(arg, idx) => {
                if let Some(args) = self.config.extra_java_args.as_mut() {
                    add_to_arguments_list(arg, args, idx);
                }
            }
            LauncherSettingsMessage::GlobalJavaArgDelete(idx) => {
                if let Some(args) = self.config.extra_java_args.as_mut() {
                    if idx < args.len() {
                        args.remove(idx);
                    }
                }
            }
            LauncherSettingsMessage::GlobalJavaArgShiftUp(idx) => {
                if let Some(args) = self.config.extra_java_args.as_mut() {
                    if idx > 0 && idx < args.len() {
                        args.swap(idx, idx - 1);
                    }
                }
            }
            LauncherSettingsMessage::GlobalJavaArgShiftDown(idx) => {
                if let Some(args) = self.config.extra_java_args.as_mut() {
                    if idx + 1 < args.len() {
                        args.swap(idx, idx + 1);
                    }
                }
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefixAdd => {
                self.config.get_launch_prefix().push(String::new());
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefixEdit(arg, idx) => {
                add_to_arguments_list(arg, self.config.get_launch_prefix(), idx);
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefixDelete(idx) => {
                let args = self.config.get_launch_prefix();
                if idx < args.len() {
                    args.remove(idx);
                }
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefixShiftUp(idx) => {
                let args = self.config.get_launch_prefix();
                if idx > 0 && idx < args.len() {
                    args.swap(idx, idx - 1);
                }
            }
            LauncherSettingsMessage::GlobalPreLaunchPrefixShiftDown(idx) => {
                let args = self.config.get_launch_prefix();
                if idx + 1 < args.len() {
                    args.swap(idx, idx + 1);
                }
            }
        }
        Task::none()
    }

    pub fn go_to_launcher_settings(&mut self) {
        if let State::LauncherSettings(_) = &self.state {
            return;
        }
        self.state = State::LauncherSettings(state::MenuLauncherSettings {
            temp_scale: self.config.ui_scale.unwrap_or(1.0),
            selected_tab: state::LauncherSettingsTab::UserInterface,
        });
    }
}

fn add_to_arguments_list(msg: String, args: &mut Vec<String>, idx: usize) {
    if msg.contains(' ') {
        args.remove(idx);
        let mut insert_idx = idx;
        for s in msg.split(' ').filter(|n| !n.is_empty()) {
            args.insert(insert_idx, s.to_owned());
            insert_idx += 1;
        }
    } else if let Some(arg) = args.get_mut(idx) {
        *arg = msg;
    }
}
