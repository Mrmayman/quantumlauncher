use iced::Task;
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::SidebarSelection,
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    state::{
        AutoSaveKind, LaunchModal, LaunchTab, Launcher, MainMenuMessage, MenuLaunch, Message,
        SidebarMessage, State,
    },
};

impl Launcher {
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
                            self.selected_instance =
                                Some(InstanceSelection::new(name, kind.is_server()));
                        }
                    }
                    *modal = None;
                }

                self.load_edit_instance(Some(tab));
                if let (LaunchTab::Log, Some(instance)) = (tab, self.selected_instance.clone()) {
                    self.load_logs(instance);
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
                return self.on_instance_selected();
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
            SidebarMessage::Scroll {
                total,
                offset,
                bounds,
            } => {
                if let State::Launch(MenuLaunch {
                    sidebar_scroll_total,
                    sidebar_scroll_offset,
                    sidebar_scroll_bounds,
                    ..
                }) = &mut self.state
                {
                    *sidebar_scroll_total = total;
                    *sidebar_scroll_offset = offset;
                    *sidebar_scroll_bounds = Some(bounds);
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
                    self.config.c_sidebar().rename_folder(*id, name);
                    self.hide_submenu();
                }
                self.sidebar_update_state();
            }
        }
        Task::none()
    }
}
