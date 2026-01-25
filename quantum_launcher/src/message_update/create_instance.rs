use iced::{widget::pane_grid, Task};
use ql_core::{InstanceSelection, IntoStringError, ListEntry, ListEntryKind};

use crate::{
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    sip,
    state::{
        AutoSaveKind, CreateInstanceMessage, Launcher, MenuCreateInstance,
        MenuCreateInstanceChoosing, Message, ProgressBar, State,
    },
};

macro_rules! iflet {
    ($self:ident, $( $field:ident ),* ; $block:block) => {
        if let State::Create(MenuCreateInstance::Choosing(MenuCreateInstanceChoosing {
            $( $field, )* ..
        })) = &mut $self.state {
            $block
        }
    };
}

impl Launcher {
    pub fn update_create_instance(&mut self, message: CreateInstanceMessage) -> Task<Message> {
        match message {
            CreateInstanceMessage::End(Err(err))
            | CreateInstanceMessage::VersionsLoaded(Err(err))
            | CreateInstanceMessage::ImportResult(Err(err)) => {
                self.set_error(err);
            }
            CreateInstanceMessage::ScreenOpen { is_server } => {
                return self.go_to_create_screen(is_server)
            }
            CreateInstanceMessage::VersionsLoaded(Ok((versions, latest))) => {
                self.create_instance_finish_loading_versions_list(versions, latest);
            }
            CreateInstanceMessage::VersionSelected(ver) => iflet!(self, selected_version; {
                *selected_version = ver;
            }),

            CreateInstanceMessage::SearchInput(t) => iflet!(self, search_box; {
                *search_box = t;
            }),
            CreateInstanceMessage::SearchSubmit => {
                iflet!(self, search_box, selected_version, is_server, selected_categories, list; {
                    let iter = || list
                        .iter()
                        .flatten()
                        .filter(|n| n.supports_server || !*is_server)
                        .filter(|n| selected_categories.contains(&n.kind))
                        .filter(|n|
                            search_box.trim().is_empty()
                            || n.name.trim().to_lowercase().contains(&search_box.trim().to_lowercase())
                        );

                    // Search priority order
                    // - Exact name match
                    // - Name contains search term
                    // - Special lwjgl3 "ports" of normal versions (de-prioritized)
                    if let Some(sel) = list.iter().flatten().find(|n| n.name == search_box.trim())
                        .or(iter().find(|n| !n.name.ends_with("-lwjgl3"))
                        .or(iter().next())) {
                        *selected_version = sel.clone();
                    }
                })
            }
            CreateInstanceMessage::SidebarResize(ratio) => {
                let window_width = self.window_state.size.0;
                let ratio = ratio * window_width;
                iflet!(self, sidebar_split, sidebar_grid_state; {
                    if let Some(split) = *sidebar_split {
                        sidebar_grid_state.resize(
                            split,
                            ratio.clamp(SIDEBAR_LIMIT_LEFT, window_width - SIDEBAR_LIMIT_RIGHT) / window_width
                        );
                    }
                });
            }
            CreateInstanceMessage::ContextMenuToggle => iflet!(self, show_category_dropdown; {
                *show_category_dropdown = !*show_category_dropdown;
            }),
            CreateInstanceMessage::CategoryToggle(kind) => iflet!(self, selected_categories; {
                if selected_categories.contains(&kind) {
                    // Don't allow removing the last category
                    if selected_categories.len() > 1 {
                        selected_categories.remove(&kind);
                    }
                } else {
                    selected_categories.insert(kind);
                }

                self.config
                    .c_persistent().create_instance_filters = Some(selected_categories.clone());
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
            }),
            CreateInstanceMessage::NameInput(name) => iflet!(self, instance_name; {
                *instance_name = name;
            }),
            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(Ok(instance)) => {
                let is_server = instance.is_server();
                self.selected_instance = Some(instance);
                return if is_server {
                    self.go_to_server_manage_menu(Some("Created Server".to_owned()))
                } else {
                    self.go_to_launch_screen(Some("Created Instance"))
                };
            }
            CreateInstanceMessage::ChangeAssetToggle(t) => iflet!(self, download_assets; {
                *download_assets = t;
            }),
            CreateInstanceMessage::Import => {
                if let Some(file) = rfd::FileDialog::new()
                    .set_title("Select an instance...")
                    .pick_file()
                {
                    let progress = ProgressBar::new();
                    self.state = State::Create(MenuCreateInstance::ImportingInstance(progress));

                    return sip(
                        |send| ql_packager::import_instance(file, true, Some(send)),
                        |n| {
                            Message::CreateInstance(CreateInstanceMessage::ImportResult(n.strerr()))
                        },
                    );
                }
            }
            CreateInstanceMessage::ImportResult(Ok(instance)) => {
                let is_valid_modpack = instance.is_some();
                self.selected_instance = instance;
                if is_valid_modpack {
                    return self.go_to_main_menu_with_message(None::<String>);
                }
                self.set_error(
                    r#"the file you imported isn't a valid QuantumLauncher/MultiMC instance.

If you meant to import a Modrinth/Curseforge/Preset pack,
create a instance with the matching version,
then go to "Mods->Add File""#,
                );
            }
        }
        Task::none()
    }

    fn create_instance_finish_loading_versions_list(
        &mut self,
        versions: Vec<ListEntry>,
        latest: String,
    ) {
        iflet!(self, selected_version, list; {
            let mut offset = 0.0;
            let len = versions.len();

            *selected_version = versions
                .iter()
                .enumerate()
                .filter(|n| n.1.kind != ListEntryKind::Snapshot)
                .find(|n| n.1.name == latest)
                .map(|n| {
                    offset = n.0 as f32 / len as f32;
                    n.1.clone()
                })
                .unwrap_or_else(|| ListEntry::new(latest));
            *list = Some(versions);
        })
    }

    pub fn go_to_create_screen(&mut self, is_server: bool) -> Task<Message> {
        let (task, handle) = Task::perform(ql_instances::list_versions(), |n| {
            Message::CreateInstance(CreateInstanceMessage::VersionsLoaded(n.strerr()))
        })
        .abortable();

        let (mut sidebar_grid_state, pane) = pane_grid::State::new(true);
        let sidebar_split = if let Some((_, split)) =
            sidebar_grid_state.split(pane_grid::Axis::Vertical, pane, false)
        {
            sidebar_grid_state.resize(split, 0.33);
            Some(split)
        } else {
            None
        };

        self.state = State::Create(MenuCreateInstance::Choosing(MenuCreateInstanceChoosing {
            _loading_list_handle: handle.abort_on_drop(),
            list: None,
            selected_version: ListEntry {
                name: String::new(),
                supports_server: true,
                kind: ListEntryKind::Release,
            },
            instance_name: String::new(),
            download_assets: true,
            search_box: String::new(),
            show_category_dropdown: false,
            selected_categories: self.config.c_persistent().get_create_instance_filters(),
            is_server,
            sidebar_grid_state,
            sidebar_split,
        }));

        task
    }

    fn create_instance(&mut self) -> Task<Message> {
        iflet!(self, instance_name, download_assets, selected_version, is_server; {
            let is_server = *is_server;

            let already_exists = {
                let existing_instances = if is_server {
                    self.server_list.as_ref()
                } else {
                    self.client_list.as_ref()
                };
                existing_instances.is_some_and(|n| {
                    n.contains(instance_name)
                        || (instance_name.is_empty() && n.contains(&selected_version.name))
                })
            };

            if already_exists {
                return Task::none();
            }

            let version = selected_version.clone();
            let instance_name = if instance_name.trim().is_empty() {
                version.name.clone()
            } else {
                instance_name.clone()
            };
            let download_assets = *download_assets;

            self.state = State::Create(MenuCreateInstance::DownloadingInstance(ProgressBar::new()));

            return if is_server {
                sip(|sender|
                    ql_servers::create_server(instance_name, version, Some(sender)),
                    |n| Message::CreateInstance(CreateInstanceMessage::End(
                        n.strerr().map(InstanceSelection::Server)
                    )),
                )
            } else {
                sip(
                    move |sender| ql_instances::create_instance(
                        instance_name,
                        version,
                        Some(sender),
                        download_assets,
                    ),
                    |n| {
                        Message::CreateInstance(CreateInstanceMessage::End(
                            n.strerr().map(InstanceSelection::Instance),
                        ))
                    },
                )
            };
        });
        Task::none()
    }
}
