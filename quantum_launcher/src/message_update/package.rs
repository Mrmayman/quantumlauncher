use iced::Task;
use ql_core::{file_utils::DirItem, IntoIoError, IntoStringError};

use crate::state::{
    Launcher, MenuExportInstance, Message, PackageInstanceMessage, ProgressBar, State,
};

/// Files/folders in `.minecraft` that are commonly large and
/// not needed to be copied/exported.
const INSTANCE_EXCEPTIONS: &[&str] = &[
    ".fabric",
    "logs",
    "command_history.txt",
    "realms_persistence.json",
    "debug",
    ".cache",
    // Common mods...
    "authlib-injector.log",
    "easy_npc",
    "CustomSkinLoader",
    ".bobby",
];

impl Launcher {
    pub fn update_package(&mut self, msg: PackageInstanceMessage) -> Task<Message> {
        match msg {
            PackageInstanceMessage::ToggleItem(idx, t) => {
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
            PackageInstanceMessage::Start => {
                if let State::ExportInstance(MenuExportInstance {
                    entries: Some(entries),
                    progress,
                    is_exporting,
                }) = &mut self.state
                {
                    let (send, recv) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv(recv));

                    let exceptions = entries
                        .iter()
                        .filter_map(|(n, b)| (!b).then_some(format!(".minecraft/{}", n.name)))
                        .collect();

                    return if *is_exporting {
                        Task::perform(
                            ql_packager::export_instance(
                                self.selected_instance.clone().unwrap(),
                                exceptions,
                                Some(send),
                            ),
                            |n| PackageInstanceMessage::ExportFinished(n.strerr()).into(),
                        )
                    } else {
                        Task::perform(
                            ql_instances::clone_instance(
                                self.selected_instance.clone().unwrap(),
                                exceptions,
                            ),
                            |n| PackageInstanceMessage::CloneFinished(n.strerr()).into(),
                        )
                    };
                }
            }
            PackageInstanceMessage::ListLoaded(res) => {
                let mut entries: Vec<(DirItem, bool)> = match res {
                    Ok(n) => n
                        .into_iter()
                        .map(|n| {
                            let enabled = !(INSTANCE_EXCEPTIONS.iter().any(|m| n.name == *m));
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
            PackageInstanceMessage::ExportOpen => {
                self.state = State::ExportInstance(MenuExportInstance {
                    entries: None,
                    progress: None,
                    is_exporting: true,
                });
                return Task::perform(
                    ql_core::file_utils::read_filenames_from_dir(
                        self.selected_instance
                            .clone()
                            .unwrap()
                            .get_dot_minecraft_path(),
                    ),
                    |n| PackageInstanceMessage::ListLoaded(n.strerr()).into(),
                );
            }
            PackageInstanceMessage::CloneOpen => {
                self.state = State::ExportInstance(MenuExportInstance {
                    entries: None,
                    progress: None,
                    is_exporting: false,
                });
                return Task::perform(
                    ql_core::file_utils::read_filenames_from_dir(
                        self.selected_instance
                            .clone()
                            .unwrap()
                            .get_dot_minecraft_path(),
                    ),
                    |n| PackageInstanceMessage::ListLoaded(n.strerr()).into(),
                );
            }
            PackageInstanceMessage::ExportFinished(res) => match res {
                Ok(bytes) => {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        if let Err(err) = std::fs::write(&path, bytes).path(path) {
                            self.set_error(err);
                        } else {
                            return self.go_to_main_menu_with_message(None::<String>);
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
            PackageInstanceMessage::CloneFinished(res) => match res {
                Ok(instance) => {
                    self.selected_instance = Some(instance);
                    return self.go_to_main_menu_with_message(Some("Instance has been copied!"));
                }
                Err(e) => self.set_error(e),
            },
        }
        Task::none()
    }
}
