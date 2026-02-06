use iced::Task;

use crate::{
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    state::{AutoSaveKind, LaunchModal, Launcher, MainMenuMessage, MenuLaunch, Message, State},
};

impl Launcher {
    pub fn update_main_menu(&mut self, msg: MainMenuMessage) -> Task<Message> {
        match msg {
            MainMenuMessage::ChangeTab(tab) => {
                self.load_edit_instance(Some(launch_tab_id));
                if let LaunchTab::Log = launch_tab_id {
                    self.load_logs(self.instance().clone());
                }
            }
            MainMenuMessage::Modal(modal) => {
                if let State::Launch(menu) = &mut self.state {
                    menu.modal = match (&modal, &menu.modal) {
                        // Unset if you click on it again
                        (
                            Some(LaunchModal::InstanceOptions),
                            Some(LaunchModal::InstanceOptions),
                        ) => None,
                        _ => modal.clone(),
                    }
                }
            }
            MainMenuMessage::DragHover { location, entered } => {
                if let State::Launch(MenuLaunch {
                    modal: Some(LaunchModal::Dragging { dragged_to, .. }),
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
            MainMenuMessage::DragDrop(location) => {
                if let State::Launch(MenuLaunch {
                    modal: Some(LaunchModal::Dragging { being_dragged, .. }),
                    ..
                }) = &mut self.state
                {
                    self.config.c_sidebar().drag_drop(being_dragged, location);
                    if let Some(sel) = &self.selected_instance {
                        if being_dragged != sel {
                            self.selected_instance = None;
                        }
                    }
                }
                self.sidebar_update_state();
            }
            MainMenuMessage::DeleteFolder(folder) => {
                self.config.c_sidebar().delete_folder(folder);
                self.sidebar_update_state();
            }
            MainMenuMessage::SidebarResize(ratio) => {
                if let State::Launch(menu) = &mut self.state {
                    // self.autosave.remove(&AutoSaveKind::LauncherConfig);
                    let window_width = self.window_state.size.0;
                    let ratio = ratio * window_width;
                    menu.resize_sidebar(
                        ratio.clamp(SIDEBAR_LIMIT_LEFT, window_width - SIDEBAR_LIMIT_RIGHT)
                            / window_width,
                    );
                }
            }
            MainMenuMessage::SidebarScroll(total) => {
                if let State::Launch(MenuLaunch {
                    sidebar_scrolled: sidebar_height,
                    ..
                }) = &mut self.state
                {
                    *sidebar_height = total;
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
            MainMenuMessage::NewFolder(at_position) => {
                self.config
                    .c_sidebar()
                    .new_folder_at(at_position, "New Folder");
                self.sidebar_update_state();
            }
            MainMenuMessage::ToggleFolderVisibility(id) => {
                let sidebar = self.config.c_sidebar();
                sidebar.toggle_visibility(id);
                self.sidebar_update_state();
            }
        }
        Task::none()
    }

    fn sidebar_update_state(&mut self) {
        self.hide_submenu();
        self.config.c_sidebar().fix();
        self.autosave.remove(&AutoSaveKind::LauncherConfig);
    }
}
