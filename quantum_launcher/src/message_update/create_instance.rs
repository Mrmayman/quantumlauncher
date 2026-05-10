use iced::{Task, widget::pane_grid};
use ql_core::{
    DownloadProgress, Instance, InstanceKind, IntoStringError, ListEntry, ListEntryKind,
};

use crate::{
    message_handler::{SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT},
    state::{
        AutoSaveKind, CreateInstanceMessage, InfoMessage, Launcher, MenuCreateInstance,
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
            | CreateInstanceMessage::ImportResult(Err(err)) => {
                self.set_error(err);
            }
            CreateInstanceMessage::ScreenOpen(kind) => {
                return self.go_to_create_screen(kind);
            }
            CreateInstanceMessage::VersionsLoaded(res) => {
                self.create_instance_finish_loading_versions_list(res);
            }
            CreateInstanceMessage::VersionSelected(ver) => {
                iflet!(self, selected_version, show_category_dropdown; {
                    *show_category_dropdown = false;
                    *selected_version = ver;
                });
            }

            CreateInstanceMessage::SearchInput(t) => iflet!(self, search_box; {
                *search_box = t;
            }),
            CreateInstanceMessage::SearchSubmit => {
                iflet!(self, search_box, selected_version, kind, selected_categories, list; {
                    let iter = || list.iter().flatten().flatten()
                        .filter(|n| n.supports_server || !matches!(kind, InstanceKind::Server))
                        .filter(|n| selected_categories.contains(&n.kind))
                        .filter(|n|
                            search_box.trim().is_empty()
                            || n.name.trim().to_lowercase().contains(&search_box.trim().to_lowercase())
                        );

                    // Search priority order
                    // - Exact name match
                    // - Name contains search term
                    // - Special lwjgl3 "ports" of normal versions (de-prioritized)
                    if let Some(sel) = list.iter().flatten().flatten().find(|n| n.name == search_box.trim())
                        .or(iter().find(|n| !n.name.ends_with("-lwjgl3"))
                        .or(iter().next())) {
                        *selected_version = sel.clone();
                    }
                });
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

            // Filters dropdown
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
            CreateInstanceMessage::ChangeKind(t) => iflet!(self, kind; {
                *kind = t;
            }),

            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(Ok(instance)) => {
                let is_server = instance.is_server();
                self.selected_instance = Some(instance);
                return self.go_to_main_menu(Some(InfoMessage::success(format!(
                    "Created {}",
                    if is_server { "Server" } else { "Instance" }
                ))));
            }
            CreateInstanceMessage::ChangeAssetToggle(t) => iflet!(self, download_assets; {
                *download_assets = t;
            }),
            CreateInstanceMessage::Import => {
                if let Some(file) = rfd::FileDialog::new()
                    .set_title("Select an instance...")
                    .pick_file()
                {
                    let (send, recv) = std::sync::mpsc::channel();
                    let progress = ProgressBar::with_recv(recv);

                    self.state = State::Create(MenuCreateInstance::ImportingInstance(progress));

                    return Task::perform(
                        ql_packager::import_instance(file.clone(), true, Some(send)),
                        |n| CreateInstanceMessage::ImportResult(n.strerr()).into(),
                    );
                }
            }
            CreateInstanceMessage::ImportResult(Ok(instance)) => {
                let is_valid_modpack = instance.is_some();
                self.selected_instance = instance;
                if is_valid_modpack {
                    return self.go_to_main_menu(None);
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
        res: Result<(Vec<ListEntry>, String), String>,
    ) {
        iflet!(self, selected_version, list; {
            let mut offset = 0.0;

            if let Ok((v, latest)) = &res {
                let len = v.len();
                *selected_version = v
                    .iter()
                    .enumerate()
                    .filter(|n| n.1.kind != ListEntryKind::Snapshot)
                    .find(|n| n.1.name == *latest)
                    .map_or_else(|| ListEntry::new(latest.clone()), |n| {
                        offset = n.0 as f32 / len as f32;
                        n.1.clone()
                    });
            }
            *list = res.map(|n| Some(n.0));
        });
    }

    fn go_to_create_screen(&mut self, kind: InstanceKind) -> Task<Message> {
        let (task, handle) = Task::perform(ql_instances::list_versions(), |n| {
            CreateInstanceMessage::VersionsLoaded(n.strerr()).into()
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
            list: Ok(None),
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
            kind,
            sidebar_grid_state,
            sidebar_split,
        }));

        task
    }

    fn create_instance(&mut self) -> Task<Message> {
        iflet!(self, instance_name, download_assets, selected_version, kind; {
            let already_exists = {
                let existing_instances = match kind {
                    InstanceKind::Client => self.client_list.as_ref(),
                    InstanceKind::Server => self.server_list.as_ref(),
                };
                existing_instances.is_some_and(|n| {
                    n.contains(instance_name)
                        || (instance_name.is_empty() && n.contains(&selected_version.name))
                })
            };

            if already_exists {
                return Task::none();
            }

            let (sender, receiver) = std::sync::mpsc::channel::<DownloadProgress>();
            let progress = ProgressBar {
                num: 0.0,
                message: Some("Started download".to_owned()),
                receiver,
                progress: DownloadProgress::DownloadingJsonManifest,
            };

            let version = selected_version.clone();
            let instance_name = if instance_name.trim().is_empty() {
                version.name.clone()
            } else {
                instance_name.clone()
            };
            let download_assets = *download_assets;
            let kind = *kind;

            self.state = State::Create(MenuCreateInstance::DownloadingInstance(progress));

            return match kind {
                InstanceKind::Server => Task::perform(
                    async move {
                        let sender = sender;
                        ql_servers::create_server(instance_name.clone(), version, Some(&sender))
                            .await
                            .strerr()
                            .map(|n| Instance::server(&n))
                    },
                    |n| CreateInstanceMessage::End(n).into(),
                ),
                InstanceKind::Client => Task::perform(
                    ql_instances::create_instance(
                        instance_name.clone(),
                        version,
                        Some(sender),
                        download_assets,
                    ),
                    |n| CreateInstanceMessage::End(
                        n.strerr().map(|n| Instance::client(&n)),
                    ).into(),
                )
            }
        });
        Task::none()
    }
}
