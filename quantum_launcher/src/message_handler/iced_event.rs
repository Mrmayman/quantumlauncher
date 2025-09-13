use super::{SIDEBAR_DRAG_LEEWAY, SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT};
use crate::message_update::MSG_RESIZE;
use crate::state::{
    Launcher, LauncherSettingsMessage, LauncherSettingsTab, MenuCreateInstance, MenuEditJarMods,
    MenuEditMods, MenuEditPresets, MenuExportInstance, MenuInstallFabric, MenuInstallOptifine,
    MenuLaunch, MenuLauncherSettings, MenuLauncherUpdate, MenuLoginAlternate, MenuLoginMS,
    MenuRecommendedMods, MenuServerCreate, Message, State,
};
use iced::{
    keyboard::{self, key::Named, Key},
    Task,
};
use ql_core::jarmod::JarMods;
use ql_core::{err, info, info_no_log, jarmod::JarMod, InstanceSelection};
use std::ffi::{OsStr, OsString};
use std::path::Path;

impl Launcher {
    pub fn iced_event(&mut self, event: iced::Event, status: iced::event::Status) -> Task<Message> {
        self.validate_sidebar_width();

        match event {
            iced::Event::Window(event) => match event {
                iced::window::Event::CloseRequested => {
                    info_no_log!("Shutting down launcher (1)");
                    std::process::exit(0);
                }
                iced::window::Event::Closed => {
                    info!("Shutting down launcher (2)");
                }
                iced::window::Event::Resized(size) => {
                    self.window_size = (size.width, size.height);
                    // Save window size to config for persistence
                    let window = self.config.window.get_or_insert_with(Default::default);
                    window.width = Some(size.width);
                    window.height = Some(size.height);

                    if let State::GenericMessage(msg) = &self.state {
                        if msg == MSG_RESIZE {
                            return self.update(Message::LauncherSettings(
                                LauncherSettingsMessage::ChangeTab(
                                    LauncherSettingsTab::UserInterface,
                                ),
                            ));
                        }
                    }
                }
                iced::window::Event::FileHovered(_) => {
                    self.set_drag_and_drop_hover(true);
                }
                iced::window::Event::FilesHoveredLeft => {
                    self.set_drag_and_drop_hover(false);
                }
                iced::window::Event::FileDropped(path) => {
                    self.set_drag_and_drop_hover(false);

                    if let (Some(extension), Some(filename)) = (
                        path.extension().map(OsStr::to_ascii_lowercase),
                        path.file_name().and_then(OsStr::to_str),
                    ) {
                        return self.drag_and_drop(&path, extension, filename);
                    }
                }
                iced::window::Event::RedrawRequested(_)
                | iced::window::Event::Moved { .. }
                | iced::window::Event::Opened { .. }
                | iced::window::Event::Focused
                | iced::window::Event::Unfocused => {}
            },
            iced::Event::Keyboard(event) => match event {
                keyboard::Event::KeyPressed {
                    key,
                    // location,
                    modifiers,
                    ..
                } => {
                    if let iced::event::Status::Ignored = status {
                        if let Key::Named(Named::Escape) = key {
                            return self.key_escape_back(true).1;
                        }
                        if let Key::Named(Named::ArrowUp) = key {
                            return self.key_change_selected_instance(false);
                        } else if let Key::Named(Named::ArrowDown) = key {
                            return self.key_change_selected_instance(true);
                        } else if let Key::Named(Named::Enter) = key {
                            if modifiers.command() {
                                return self.launch_start();
                            }
                        } else if let Key::Named(Named::Backspace) = key {
                            match self.selected_instance.clone() {
                                Some(InstanceSelection::Instance(_)) => {
                                    return self.kill_selected_instance();
                                }
                                Some(InstanceSelection::Server(server)) => {
                                    self.kill_selected_server(&server);
                                }
                                None => {}
                            }
                        } else if let Key::Character(ch) = &key {
                            if modifiers.command() {
                                if ch == "q" {
                                    let safe_to_exit = self.client_processes.is_empty()
                                        && self.server_processes.is_empty()
                                        && (self.key_escape_back(false).0
                                            || matches!(self.state, State::Launch(_)));

                                    if safe_to_exit {
                                        info_no_log!("CTRL-Q pressed, closing launcher...");
                                        std::process::exit(1);
                                    }
                                } else if ch == "a" {
                                    if let State::EditMods(_) = &self.state {
                                        return Task::done(Message::ManageMods(
                                            crate::state::ManageModsMessage::SelectAll,
                                        ));
                                    } else if let State::EditJarMods(_) = &self.state {
                                        return Task::done(Message::ManageJarMods(
                                            crate::state::ManageJarModsMessage::SelectAll,
                                        ));
                                    }
                                }
                            }
                        }

                        self.keys_pressed.insert(key);
                    } else {
                        // FUTURE
                    }
                }
                keyboard::Event::KeyReleased { key, .. } => {
                    self.keys_pressed.remove(&key);
                }
                keyboard::Event::ModifiersChanged(modifiers) => {
                    if let iced::event::Status::Ignored = status {
                        self.modifiers_pressed = modifiers;
                    }
                }
            },
            iced::Event::Mouse(mouse) => match mouse {
                iced::mouse::Event::CursorMoved { position } => {
                    self.mouse_pos.0 = position.x;
                    self.mouse_pos.1 = position.y;

                    if let State::Launch(MenuLaunch {
                        sidebar_width,
                        sidebar_dragging: true,
                        ..
                    }) = &mut self.state
                    {
                        if self.mouse_pos.0 < SIDEBAR_LIMIT_LEFT {
                            *sidebar_width = SIDEBAR_LIMIT_LEFT as u16;
                        } else if (self.mouse_pos.0 + f32::from(SIDEBAR_LIMIT_RIGHT)
                            > self.window_size.0)
                            && self.window_size.0 as u16 > SIDEBAR_LIMIT_RIGHT
                        {
                            *sidebar_width = self.window_size.0 as u16 - SIDEBAR_LIMIT_RIGHT;
                        } else {
                            *sidebar_width = self.mouse_pos.0 as u16;
                        }
                    }
                }
                iced::mouse::Event::ButtonPressed(button) => {
                    if let (State::Launch(menu), iced::mouse::Button::Left) =
                        (&mut self.state, button)
                    {
                        let difference = self.mouse_pos.0 - f32::from(menu.sidebar_width);
                        if difference > 0.0 && difference < SIDEBAR_DRAG_LEEWAY {
                            menu.sidebar_dragging = true;
                        }
                    }
                    if let iced::event::Status::Ignored = status {
                        self.hide_submenu();
                    }
                }
                iced::mouse::Event::ButtonReleased(button) => {
                    if let (State::Launch(menu), iced::mouse::Button::Left) =
                        (&mut self.state, button)
                    {
                        menu.sidebar_dragging = false;
                    }
                }
                iced::mouse::Event::WheelScrolled { /*delta*/ .. } => {
                    /*if let iced::event::Status::Ignored = status {
                        if self.keys_pressed.contains(&Key::Named(Named::Control)) {
                            match delta {
                                iced::mouse::ScrollDelta::Lines { y, .. }
                                | iced::mouse::ScrollDelta::Pixels { y, .. } => {
                                    let new_scale =
                                        self.config.ui_scale.unwrap_or(1.0) + (f64::from(y) / 5.0);
                                    let new_scale = new_scale.clamp(0.5, 2.0);
                                    self.config.ui_scale = Some(new_scale);
                                    if let State::LauncherSettings(menu) = &mut self.state {
                                        menu.temp_scale = new_scale;
                                    }
                                }
                            }
                        }
                    }*/
                }
                iced::mouse::Event::CursorEntered | iced::mouse::Event::CursorLeft => {}
            },
            iced::Event::Touch(_) => {}
        }
        Task::none()
    }

    fn drag_and_drop(&mut self, path: &Path, extension: OsString, filename: &str) -> Task<Message> {
        if let State::EditMods(_) = &self.state {
            if extension == "jar" || extension == "disabled" {
                self.load_jar_from_path(path, filename);
                Task::none()
            } else if extension == "qmp" {
                self.load_qmp_from_path(path)
            } else if extension == "zip" || extension == "mrpack" {
                self.load_modpack_from_path(path.to_owned())
            } else {
                Task::none()
            }
        } else if let State::ManagePresets(_) = &self.state {
            if extension == "qmp" {
                self.load_qmp_from_path(path)
            } else if extension == "zip" || extension == "mrpack" {
                self.load_modpack_from_path(path.to_owned())
            } else {
                Task::none()
            }
        } else if let State::EditJarMods(MenuEditJarMods {
            jarmods: Some(jarmods),
            ..
        }) = &mut self.state
        {
            if extension == "jar" || extension == "zip" {
                Self::load_jarmods_from_path(
                    self.selected_instance.as_ref().unwrap(),
                    path,
                    filename,
                    jarmods,
                );
            }
            Task::none()
        } else if let State::InstallOptifine(MenuInstallOptifine::Choosing { .. }) = &mut self.state
        {
            if extension == "jar" || extension == "zip" {
                self.install_optifine_confirm(path)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    fn validate_sidebar_width(&mut self) {
        if let State::Launch(MenuLaunch { sidebar_width, .. }) = &mut self.state {
            self.config.sidebar_width = Some(u32::from(*sidebar_width));
            let window_width = self.window_size.0;

            if window_width > f32::from(SIDEBAR_LIMIT_RIGHT)
                && *sidebar_width > window_width as u16 - SIDEBAR_LIMIT_RIGHT
            {
                *sidebar_width = window_width as u16 - SIDEBAR_LIMIT_RIGHT;
            }

            if window_width > SIDEBAR_LIMIT_LEFT && *sidebar_width < SIDEBAR_LIMIT_LEFT as u16 {
                *sidebar_width = SIDEBAR_LIMIT_LEFT as u16;
            }
        }
    }

    fn load_jarmods_from_path(
        selected_instance: &InstanceSelection,
        path: &Path,
        filename: &str,
        jarmods: &mut JarMods,
    ) {
        let new_path = selected_instance
            .get_instance_path()
            .join("jarmods")
            .join(filename);
        if path != new_path {
            if let Err(err) = std::fs::copy(path, &new_path) {
                err!("Couldn't drag and drop mod file in: {err}");
            } else if !jarmods.mods.iter().any(|n| n.filename == filename) {
                jarmods.mods.push(JarMod {
                    filename: filename.to_owned(),
                    enabled: true,
                });
            }
        }
    }

    fn key_escape_back(&mut self, affect: bool) -> (bool, Task<Message>) {
        let mut should_return_to_main_screen = false;
        let mut should_return_to_mods_screen = false;
        let mut should_return_to_download_screen = false;

        match &self.state {
            State::ChangeLog
            | State::EditMods(MenuEditMods {
                mod_update_progress: None,
                ..
            })
            | State::Create(
                MenuCreateInstance::LoadingList { .. } | MenuCreateInstance::Choosing { .. },
            )
            | State::ServerCreate(
                MenuServerCreate::LoadingList | MenuServerCreate::Loaded { .. },
            )
            | State::Error { .. }
            | State::UpdateFound(MenuLauncherUpdate { progress: None, .. })
            | State::LauncherSettings(_)
            | State::LoginMS(MenuLoginMS { .. })
            | State::AccountLogin
            | State::ExportInstance(MenuExportInstance { progress: None, .. })
            | State::LoginAlternate(MenuLoginAlternate {
                is_loading: false, ..
            })
            | State::Welcome(_) => {
                should_return_to_main_screen = true;
            }
            State::License(_) => {
                if affect {
                    if let State::LauncherSettings(_) = &self.state {
                    } else {
                        self.state = State::LauncherSettings(MenuLauncherSettings {
                            temp_scale: self.config.ui_scale.unwrap_or(1.0),
                            selected_tab: LauncherSettingsTab::About,
                        });
                    }
                }
                return (true, Task::none());
            }
            State::ConfirmAction { no, .. } => {
                if affect {
                    return (true, self.update(no.clone()));
                }
            }
            State::InstallOptifine(MenuInstallOptifine::Choosing { .. })
            | State::InstallFabric(MenuInstallFabric::Loaded { progress: None, .. })
            | State::EditJarMods(_)
            | State::ExportMods(_)
            | State::ManagePresets(MenuEditPresets {
                is_building: false,
                progress: None,
                ..
            })
            | State::RecommendedMods(
                MenuRecommendedMods::Loaded { .. }
                | MenuRecommendedMods::InstallALoader
                | MenuRecommendedMods::NotSupported,
            ) => {
                should_return_to_mods_screen = true;
            }
            State::ModsDownload(menu) if menu.opened_mod.is_some() => {
                should_return_to_download_screen = true;
            }
            State::ModsDownload(menu) if menu.mods_download_in_progress.is_empty() => {
                should_return_to_mods_screen = true;
            }
            State::InstallPaper
            | State::ExportInstance(_)
            | State::InstallForge(_)
            | State::InstallJava
            | State::InstallOptifine(_)
            | State::UpdateFound(_)
            | State::InstallFabric(_)
            | State::EditMods(_)
            | State::Create(_)
            | State::ManagePresets(_)
            | State::ModsDownload(_)
            | State::ServerCreate(_)
            | State::GenericMessage(_)
            | State::AccountLoginProgress(_)
            | State::ImportModpack(_)
            | State::CurseforgeManualDownload(_)
            | State::LoginAlternate(_)
            | State::LogUploadResult { .. }
            | State::RecommendedMods(MenuRecommendedMods::Loading { .. })
            | State::Launch(_) => {}
        }

        if affect {
            if should_return_to_main_screen {
                return (true, self.go_to_launch_screen::<String>(None));
            }
            if should_return_to_mods_screen {
                return (true, self.go_to_edit_mods_menu(false));
            }
            if should_return_to_download_screen {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                    return (
                        true,
                        iced::widget::scrollable::scroll_to(
                            iced::widget::scrollable::Id::new("MenuModsDownload:main:mods_list"),
                            menu.scroll_offset,
                        ),
                    );
                }
            }
        }

        (
            should_return_to_main_screen
                | should_return_to_mods_screen
                | should_return_to_download_screen,
            Task::none(),
        )
    }

    fn hide_submenu(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            menu.submenu1_shown = false;
        }
    }

    fn key_change_selected_instance(&mut self, down: bool) -> Task<Message> {
        let (is_viewing_server, sidebar_height) = {
            let State::Launch(menu) = &self.state else {
                return Task::none();
            };
            (menu.is_viewing_server, menu.sidebar_height)
        };
        let list = if is_viewing_server {
            self.server_list.clone()
        } else {
            self.client_list.clone()
        };

        let Some(list) = list else {
            return Task::none();
        };

        // If the user actually switched instances,
        // and not hitting top/bottom of the list.
        let mut did_scroll = false;

        let idx = if let Some(selected_instance) = &mut self.selected_instance {
            if let Some(idx) = list
                .iter()
                .enumerate()
                .find_map(|(i, n)| (n == selected_instance.get_name()).then_some(i))
            {
                if down {
                    if idx + 1 < list.len() {
                        did_scroll = true;
                        *selected_instance =
                            InstanceSelection::new(list.get(idx + 1).unwrap(), is_viewing_server);
                        idx + 1
                    } else {
                        idx
                    }
                } else if idx > 0 {
                    did_scroll = true;
                    *selected_instance =
                        InstanceSelection::new(list.get(idx - 1).unwrap(), is_viewing_server);
                    idx - 1
                } else {
                    idx
                }
            } else {
                debug_assert!(
                    false,
                    "Selected instance {selected_instance:?}, not found in list?"
                );
                0
            }
        } else {
            did_scroll = true;
            self.selected_instance = list
                .first()
                .map(|n| InstanceSelection::new(n, is_viewing_server));
            0
        };

        if did_scroll {
            self.load_edit_instance(None);
        }

        let scroll_pos = idx as f32 / (list.len() as f32 - 1.0);
        let scroll_pos = scroll_pos * sidebar_height;
        iced::widget::scrollable::scroll_to(
            iced::widget::scrollable::Id::new("MenuLaunch:sidebar"),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: scroll_pos,
            },
        )
    }
}
