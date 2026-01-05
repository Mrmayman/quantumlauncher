use crate::message_update::MSG_RESIZE;
use crate::state::{
    AutoSaveKind, CreateInstanceMessage, LaunchTabId, Launcher, LauncherSettingsMessage,
    LauncherSettingsTab, MenuCreateInstance, MenuCreateInstanceChoosing, MenuEditMods,
    MenuEditPresets, MenuExportInstance, MenuInstallFabric, MenuInstallOptifine, MenuInstallPaper,
    MenuLauncherSettings, MenuLauncherUpdate, MenuLoginAlternate, MenuLoginMS, MenuRecommendedMods,
    Message, State,
};
use iced::{
    keyboard::{self, key::Named, Key},
    Task,
};
use ql_core::jarmod::JarMods;
use ql_core::{err, jarmod::JarMod, pt_no_log, InstanceSelection};
use std::ffi::OsStr;
use std::path::Path;

impl Launcher {
    pub fn iced_event(&mut self, event: iced::Event, status: iced::event::Status) -> Task<Message> {
        match event {
            iced::Event::Window(event) => match event {
                iced::window::Event::CloseRequested => {
                    pt_no_log!("Closing...");
                    std::process::exit(0);
                }
                iced::window::Event::Resized(size) => {
                    self.window_state.size = (size.width, size.height);

                    // Remember window height
                    let window = self.config.window.get_or_insert_with(Default::default);
                    window.width = Some(size.width);
                    window.height = Some(size.height);
                    if window.save_window_size {
                        self.autosave.remove(&AutoSaveKind::LauncherConfig);
                    }

                    // Clear the "Resize the window to apply changes"
                    // after changing UI scale
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
                        return self.drag_and_drop(&path, &extension, filename);
                    }
                }
                iced::window::Event::Closed
                | iced::window::Event::RedrawRequested(_)
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
                    return self.handle_key_press(key, modifiers, status);
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
                    let pos = (position.x, position.y);
                    self.window_state.mouse_pos = pos;
                }
                iced::mouse::Event::ButtonPressed(_) => {
                    if let iced::event::Status::Ignored = status {
                        self.hide_submenu();
                    }
                }
                _ => {}
            },
            iced::Event::Touch(_) => {}
        }
        Task::none()
    }

    fn handle_key_press(
        &mut self,
        key: Key,
        modifiers: keyboard::Modifiers,
        status: iced::event::Status,
    ) -> Task<Message> {
        let ignored = matches!(status, iced::event::Status::Ignored);
        if let (Key::Named(Named::Escape), true) = (key.clone(), ignored) {
            return self.key_escape_back(true).1;
        }

        if let Key::Character(ch) = &key {
            let msg = match (
                ch.as_str(),
                modifiers.command(),
                modifiers.alt(),
                ignored,
                &self.state,
            ) {
                ("q", true, _, true, _) => Message::CoreTryQuit,

                // ========
                // MANAGE MODS MENU
                // ========
                ("a", true, _, true, State::EditMods(_)) => {
                    Message::ManageMods(crate::state::ManageModsMessage::SelectAll)
                }
                // Ctrl-F search in mods list (with toggling)
                #[rustfmt::skip]
                ("f", true, _, _, State::EditMods(MenuEditMods { search: Some(_), .. })) => {
                    Message::ManageMods(crate::state::ManageModsMessage::SetSearch(None))
                },
                ("f", true, _, _, State::EditMods(_)) => Message::Multiple(vec![
                    Message::ManageMods(crate::state::ManageModsMessage::SetSearch(Some(
                        String::new(),
                    ))),
                    Message::CoreFocusNext,
                ]),

                // Search Action (general)
                ("f", true, _, _, State::Create(MenuCreateInstance::Choosing { .. }))
                | ("/", _, _, true, State::Create(MenuCreateInstance::Choosing { .. }))
                | ("f", true, _, _, State::ModsDownload(_))
                | ("/", _, _, true, State::ModsDownload(_)) => Message::CoreFocusNext,

                // Misc
                ("a", true, _, true, State::EditJarMods(_)) => {
                    Message::ManageJarMods(crate::state::ManageJarModsMessage::SelectAll)
                }

                // ========
                // MAIN MENU
                // ========
                ("n", true, _, _, State::Launch(n)) => {
                    Message::CreateInstance(CreateInstanceMessage::ScreenOpen {
                        is_server: n.is_viewing_server,
                    })
                }
                ("1", ctrl, alt, _, State::Launch(_)) if ctrl | alt => {
                    Message::LaunchChangeTab(LaunchTabId::Buttons)
                }
                ("2", ctrl, alt, _, State::Launch(_)) if ctrl | alt => {
                    Message::LaunchChangeTab(LaunchTabId::Edit)
                }
                ("3", ctrl, alt, _, State::Launch(_)) if ctrl | alt => {
                    Message::LaunchChangeTab(LaunchTabId::Saves)
                }
                ("4", ctrl, alt, _, State::Launch(_)) if ctrl | alt => {
                    Message::LaunchChangeTab(LaunchTabId::Log)
                }
                (",", true, _, _, State::Launch(_)) => {
                    Message::LauncherSettings(LauncherSettingsMessage::Open)
                }
                _ => Message::Nothing,
            };
            return Task::done(msg);
        } else if let State::LauncherSettings(menu) = &mut self.state {
            if let Key::Named(Named::ArrowUp) = key {
                return Task::done(Message::LauncherSettings(
                    LauncherSettingsMessage::ChangeTab(menu.selected_tab.prev()),
                ));
            } else if let Key::Named(Named::ArrowDown) = key {
                return Task::done(Message::LauncherSettings(
                    LauncherSettingsMessage::ChangeTab(menu.selected_tab.next()),
                ));
            }
        } else if let State::License(menu) = &mut self.state {
            if let Key::Named(Named::ArrowUp) = key {
                return Task::done(Message::LicenseChangeTab(menu.selected_tab.prev()));
            } else if let Key::Named(Named::ArrowDown) = key {
                return Task::done(Message::LicenseChangeTab(menu.selected_tab.next()));
            }
        } else if let (State::Launch(_), true) = (&self.state, ignored) {
            if let Key::Named(Named::ArrowUp) = key {
                return self.key_change_selected_instance(false);
            } else if let Key::Named(Named::ArrowDown) = key {
                return self.key_change_selected_instance(true);
            } else if let Key::Named(Named::Enter) = key {
                if modifiers.command() {
                    return self.launch_start();
                }
            } else if let Key::Named(Named::Backspace) = key {
                if modifiers.command() {
                    return Task::done(Message::LaunchKill);
                }
            }
        } else if let State::Create(MenuCreateInstance::Choosing(MenuCreateInstanceChoosing {
            list: Some(_),
            ..
        })) = &self.state
        {
            if let Key::Named(Named::Enter) = key {
                if modifiers.command() {
                    return Task::done(Message::CreateInstance(CreateInstanceMessage::Start));
                }
            }
        }
        self.keys_pressed.insert(key);

        Task::none()
    }

    fn drag_and_drop(&mut self, path: &Path, extension: &OsStr, filename: &str) -> Task<Message> {
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
        } else if let State::EditJarMods(menu) = &mut self.state {
            if extension == "jar" || extension == "zip" {
                Self::load_jarmods_from_path(
                    self.selected_instance.as_ref().unwrap(),
                    path,
                    filename,
                    &mut menu.jarmods,
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

    pub fn key_escape_back(&mut self, affect: bool) -> (bool, Task<Message>) {
        let mut ret_to_main_screen = false;
        let mut ret_to_mods = false;
        let mut ret_to_mod_store = false;

        if affect && self.hide_submenu() {
            return (true, Task::none());
        }

        match &self.state {
            State::ChangeLog
            | State::EditMods(MenuEditMods {
                mod_update_progress: None,
                ..
            })
            | State::Create(MenuCreateInstance::Choosing { .. })
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
                ret_to_main_screen = true;
            }
            State::License(_) => {
                if affect {
                    if let State::LauncherSettings(_) = &self.state {
                    } else {
                        self.state = State::LauncherSettings(MenuLauncherSettings {
                            temp_scale: self.config.ui_scale.unwrap_or(1.0),
                            selected_tab: LauncherSettingsTab::About,
                            arg_split_by_space: true,
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
            | State::InstallFabric(MenuInstallFabric::Loading { .. })
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
            )
            | State::InstallPaper(
                MenuInstallPaper::Loading { .. } | MenuInstallPaper::Loaded { .. },
            ) => {
                ret_to_mods = true;
            }
            State::ModsDownload(menu) if menu.opened_mod.is_some() => {
                ret_to_mod_store = true;
            }
            State::ModsDownload(menu) if menu.mods_download_in_progress.is_empty() => {
                ret_to_mods = true;
            }
            State::InstallPaper(_)
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
            if ret_to_main_screen {
                return (true, self.go_to_main_menu_with_message(None::<String>));
            }
            if ret_to_mods {
                return (true, self.go_to_edit_mods_menu(false));
            }
            if ret_to_mod_store {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                    menu.description = None;
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
            ret_to_main_screen | ret_to_mods | ret_to_mod_store,
            Task::none(),
        )
    }

    fn hide_submenu(&mut self) -> bool {
        if let State::EditMods(menu) = &mut self.state {
            if menu.modal.is_some() {
                menu.modal = None;
                return true;
            }
            if menu.search.is_some() {
                menu.search = None;
                return true;
            }
        } else if let State::Create(MenuCreateInstance::Choosing(MenuCreateInstanceChoosing {
            show_category_dropdown,
            ..
        })) = &mut self.state
        {
            if *show_category_dropdown {
                *show_category_dropdown = false;
                return true;
            }
        }
        false
    }

    fn key_change_selected_instance(&mut self, down: bool) -> Task<Message> {
        let (is_viewing_server, sidebar_height) = {
            let State::Launch(menu) = &self.state else {
                return Task::none();
            };
            (menu.is_viewing_server, menu.sidebar_scrolled)
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

        let scroll_pos = idx as f32 / (list.len() as f32 - 1.0);
        let scroll_pos = scroll_pos * sidebar_height;
        let scroll_task = iced::widget::scrollable::scroll_to(
            iced::widget::scrollable::Id::new("MenuLaunch:sidebar"),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: scroll_pos,
            },
        );

        if did_scroll {
            Task::batch([scroll_task, self.on_instance_selected()])
        } else {
            scroll_task
        }
    }
}
