use iced::Task;
use ql_core::{
    DownloadProgress, InstanceSelection, IntoStringError, JsonDownloadError, ListEntry,
    ListEntryKind,
};

use crate::state::{
    CreateInstanceMessage, Launcher, MenuCreateInstance, Message, ProgressBar, State,
};

macro_rules! iflet {
    ($self:ident, $( $field:ident ),* ; $block:block) => {
        if let State::Create(MenuCreateInstance::Choosing {
            $( $field, )* ..
        }) = &mut $self.state {
            $block
        }
    };
}

impl Launcher {
    pub fn update_create_instance(&mut self, message: CreateInstanceMessage) -> Task<Message> {
        match message {
            CreateInstanceMessage::ScreenOpen { is_server } => {
                return self.go_to_create_screen(is_server)
            }
            CreateInstanceMessage::VersionsLoaded(result, is_server) => {
                self.create_instance_finish_loading_versions_list(result, is_server);
            }
            CreateInstanceMessage::VersionSelected(ver) => {
                iflet!(self, selected_version; {
                    *selected_version = ver;
                });
            }
            CreateInstanceMessage::SearchInput(t) => {
                iflet!(self, search_box; {
                    *search_box = t;
                });
            }
            CreateInstanceMessage::SearchSubmit => {
                iflet!(self, search_box, selected_version, is_server, selected_categories; {
                    if let Some(sel) = self.version_list_cache.list
                        .as_deref()
                        .unwrap()
                        .iter()
                        .filter(|n| n.supports_server || !*is_server)
                        .filter(|n| selected_categories.contains(&n.kind))
                        .find(|n|
                            search_box.trim().is_empty()
                            || n.name.trim().to_lowercase().contains(&search_box.trim().to_lowercase())
                        ) {
                        *selected_version = sel.clone();
                    }
                })
            }
            CreateInstanceMessage::ContextMenuToggle => {
                iflet!(self, show_category_dropdown; {
                    *show_category_dropdown = !*show_category_dropdown;
                })
            }
            CreateInstanceMessage::CategoryToggle(kind) => {
                iflet!(self, selected_categories; {
                    if selected_categories.contains(&kind) {
                        // Don't allow removing the last category
                        if selected_categories.len() > 1 {
                            selected_categories.remove(&kind);
                        }
                    } else {
                        selected_categories.insert(kind);
                    }
                })
            }
            CreateInstanceMessage::NameInput(name) => self.update_created_instance_name(name),
            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(result) => match result {
                Ok(instance) => {
                    let is_server = instance.is_server();
                    self.selected_instance = Some(instance);
                    return if is_server {
                        self.go_to_server_manage_menu(Some("Created Server".to_owned()))
                    } else {
                        self.go_to_launch_screen(Some("Created Instance"))
                    };
                }
                Err(n) => self.set_error(n),
            },
            CreateInstanceMessage::ChangeAssetToggle(t) => {
                if let State::Create(MenuCreateInstance::Choosing {
                    download_assets, ..
                }) = &mut self.state
                {
                    *download_assets = t;
                }
            }
            CreateInstanceMessage::Cancel => {
                return self.go_to_main_menu_with_message(None::<String>)
            }
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
                        |n| {
                            Message::CreateInstance(CreateInstanceMessage::ImportResult(n.strerr()))
                        },
                    );
                }
            }
            CreateInstanceMessage::ImportResult(res) => match res {
                Ok(instance) => {
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
                Err(err) => self.set_error(err),
            },
        }
        Task::none()
    }

    fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<(Vec<ListEntry>, String), String>,
        is_server: bool,
    ) {
        match result {
            Ok((versions, latest)) => {
                self.version_list_cache.list = Some(versions.clone());
                self.version_list_cache.latest_stable = Some(latest.clone());

                if let State::Create(MenuCreateInstance::LoadingList { .. }) = &self.state {
                    let mut offset = 0.0;
                    let len = versions.len();

                    self.state = State::Create(MenuCreateInstance::Choosing {
                        instance_name: String::new(),
                        selected_version: versions
                            .iter()
                            .enumerate()
                            .filter(|n| n.1.kind != ListEntryKind::Snapshot)
                            .find(|n| n.1.name == latest)
                            .map(|n| {
                                offset = n.0 as f32 / len as f32;
                                n.1.clone()
                            })
                            .unwrap_or_else(|| ListEntry::new(latest)),
                        download_assets: true,
                        search_box: String::new(),
                        show_category_dropdown: false,
                        selected_categories: ListEntryKind::default_selected(),
                        is_server,
                    });
                }
            }
            Err(n) => self.set_error(n),
        }
    }

    pub fn go_to_create_screen(&mut self, is_server: bool) -> Task<Message> {
        if let Some(list) = &self.version_list_cache.list {
            self.state = State::Create(MenuCreateInstance::Choosing {
                instance_name: String::new(),
                selected_version: self
                    .version_list_cache
                    .latest_stable
                    .as_ref()
                    .map(|latest| {
                        list.iter()
                            .find(|n| n.name == *latest)
                            .cloned()
                            .unwrap_or_else(|| ListEntry::new(latest.to_owned()))
                    })
                    .or_else(|| list.first().cloned())
                    .unwrap(),
                download_assets: true,
                search_box: String::new(),
                show_category_dropdown: false,
                selected_categories: ListEntryKind::default_selected(),
                is_server,
            });
            Task::none()
        } else {
            let msg = move |n: Result<(Vec<ListEntry>, String), JsonDownloadError>| {
                Message::CreateInstance(CreateInstanceMessage::VersionsLoaded(
                    n.strerr(),
                    is_server,
                ))
            };
            let (task, handle) = Task::perform(ql_instances::list_versions(), msg).abortable();

            self.state = State::Create(MenuCreateInstance::LoadingList {
                _handle: handle.abort_on_drop(),
            });

            task
        }
    }

    fn update_created_instance_name(&mut self, name: String) {
        if let State::Create(MenuCreateInstance::Choosing { instance_name, .. }) = &mut self.state {
            *instance_name = name;
        }
    }

    fn create_instance(&mut self) -> Task<Message> {
        if let State::Create(MenuCreateInstance::Choosing {
            instance_name,
            download_assets,
            selected_version,
            is_server,
            ..
        }) = &mut self.state
        {
            let is_server = *is_server;
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

            self.state = State::Create(MenuCreateInstance::DownloadingInstance(progress));

            return if is_server {
                Task::perform(
                    async move {
                        let sender = sender;
                        ql_servers::create_server(instance_name.clone(), version, Some(&sender))
                            .await
                            .strerr()
                            .map(InstanceSelection::Server)
                    },
                    |n| Message::CreateInstance(CreateInstanceMessage::End(n)),
                )
            } else {
                Task::perform(
                    ql_instances::create_instance(
                        instance_name.clone(),
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
        }
        Task::none()
    }
}
