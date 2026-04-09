use std::{collections::HashMap, time::Instant};

use frostmark::MarkState;
use iced::{Task, futures::executor::block_on, widget::scrollable::AbsoluteOffset};
use ql_core::{
    InstanceConfigJson, InstanceKind, IntoStringError, JsonFileError, err, json::VersionDetails,
};
use ql_mod_manager::store::{self, ModIndex, Query, QueryType, StoreBackendType, get_description};

use crate::state::{
    ImageState, InstallModsMessage, Launcher, MenuCurseforgeManualDownload, MenuModsDownload,
    Message, ModOperation, ModsDownloadSearch, ProgressBar, State,
};

impl Launcher {
    pub fn update_install_mods(&mut self, message: InstallModsMessage) -> Task<Message> {
        let is_server = matches!(
            self.selected_instance.as_ref().map(|n| n.kind),
            Some(InstanceKind::Server)
        );

        match message {
            InstallModsMessage::LoadedDescription(Err(err))
            | InstallModsMessage::LoadedExtendedInfo(Err(err))
            | InstallModsMessage::DownloadComplete(Err(err))
            | InstallModsMessage::SearchResult(Err(err))
            | InstallModsMessage::IndexUpdated(Err(err))
            | InstallModsMessage::UninstallComplete(Err(err)) => {
                self.set_error(err);
            }

            InstallModsMessage::SearchResult(Ok(search)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.continuation_is_loading = false;
                    menu.continuation_has_ended = search.reached_end;

                    if search.start_time > menu.latest_load && menu.search.backend == search.backend
                    {
                        menu.latest_load = search.start_time;

                        if let (Some(results), true) = (&mut menu.results, search.offset > 0) {
                            results.mods.extend(search.mods);
                        } else {
                            menu.results = Some(search);
                            menu.scroll_offset = AbsoluteOffset::default();
                            return iced::widget::scrollable::scroll_to(
                                iced::widget::scrollable::Id::new(
                                    "MenuModsDownload:main:mods_list",
                                ),
                                AbsoluteOffset::default(),
                            );
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
                        menu.continuation_has_ended = false;
                    }

                    menu.scroll_offset = absolute_offset;
                    if (scroll_px > total_height)
                        && !menu.continuation_is_loading
                        && !menu.continuation_has_ended
                    {
                        menu.continuation_is_loading = true;

                        let offset = if let Some(results) = &menu.results {
                            results.offset + results.mods.len()
                        } else {
                            0
                        };
                        return menu.search_store(is_server, offset);
                    }
                }
            }
            InstallModsMessage::Open => match block_on(self.open_mods_store()) {
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
                    menu.search.term = input;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::Click(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    menu.reload_description(&mut self.images); // in case already cached

                    if let Some(task) = menu.fetch_description_cached(i) {
                        return task;
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
            InstallModsMessage::LoadedDescription(Ok((id, description))) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_descriptions.insert(id, description);
                    menu.reload_description(&mut self.images);
                }
            }
            InstallModsMessage::LoadedExtendedInfo(Ok((id, info))) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    if let Some(res) = &mut menu.results {
                        for m in &mut res.mods {
                            // Fill in that mod's entry with extended info
                            if m.get_id() == id {
                                *m = info;
                                break;
                            }
                        }
                    }
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
                    match block_on(self.open_mods_store()) {
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
                    not_allowed,
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
                    menu.search.backend = backend;
                    menu.search.categories.reset();
                    menu.search.sort_by = store::SearchSortBy::default_option(backend);

                    return Task::batch([
                        menu.search_store(is_server, 0),
                        menu.search.load_categories(),
                    ]);
                }
            }
            InstallModsMessage::ChangeQueryType(query) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.query_type = query;
                    menu.search.categories.reset();

                    return Task::batch([
                        menu.search_store(is_server, 0),
                        menu.search.load_categories(),
                    ]);
                }
            }
            InstallModsMessage::ChangeSortBy(s) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.sort_by = s;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ChangeSortAscending(asc) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.sort_ascending = asc;
                    return menu.search_store(is_server, 0);
                }
            }

            InstallModsMessage::CategoriesLoaded(res) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.categories.categories = res;
                }
            }
            InstallModsMessage::CategoriesToggle(slug) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.categories.toggle(&slug);
                    return menu.search_store(is_server, 0);
                }
            }

            InstallModsMessage::CategoriesUseAll(b) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.categories.use_all = b;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ForceOpenSource(b) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.search.force_open_source = b;
                    return menu.search_store(is_server, 0);
                }
            }

            InstallModsMessage::InstallModpack(id) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

                let selected_instance = self.selected_instance.clone().unwrap();

                return Task::perform(
                    async move {
                        store::download_mod(&id, &selected_instance, Some(sender))
                            .await
                            .map(|not_allowed| (id, not_allowed))
                    },
                    |n| InstallModsMessage::DownloadComplete(n.strerr()).into(),
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

                let mod_id = hit.get_id();
                mods_download_in_progress
                    .insert(mod_id.clone(), (hit.title.clone(), ModOperation::Deleting));
                let selected_instance = self.instance().clone();

                return Task::perform(store::delete_mods(vec![mod_id], selected_instance), |n| {
                    InstallModsMessage::UninstallComplete(n.strerr()).into()
                });
            }
            InstallModsMessage::UninstallComplete(Ok(ids)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    for id in ids {
                        menu.mods_download_in_progress.remove(&id);
                        menu.mod_index.mods.remove(&id);
                    }
                }
            }
        }
        Task::none()
    }

    async fn open_mods_store(&mut self) -> Result<Task<Message>, JsonFileError> {
        let instance = self.instance();

        let config = InstanceConfigJson::read(instance).await?;
        let version_json = if let State::EditMods(menu) = &self.state {
            menu.version_json.clone()
        } else {
            Box::new(VersionDetails::load(instance).await?)
        };
        let mod_index = ModIndex::load(instance).await?;

        let mut menu = MenuModsDownload {
            search: ModsDownloadSearch::default(),
            scroll_offset: AbsoluteOffset::default(),
            latest_load: Instant::now(),
            results: None,
            opened_mod: None,
            mod_descriptions: HashMap::new(),
            mods_download_in_progress: HashMap::new(),
            continuation_is_loading: false,
            continuation_has_ended: false,
            description: None,

            config,
            version_json,
            mod_index,
        };
        let command = Task::batch([
            menu.search_store(instance.is_server(), 0),
            menu.search.load_categories(),
        ]);
        self.state = State::ModsDownload(menu);
        Ok(command)
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

        menu.mods_download_in_progress
            .insert(hit.get_id(), (hit.title.clone(), ModOperation::Downloading));
        let id = hit.get_id();

        if let QueryType::ModPacks = menu.search.query_type {
            self.state = State::ConfirmAction {
                msg1: format!("install the modpack: {}", hit.title),
                msg2: "This might take a while, install many files, and use a lot of network..."
                    .to_owned(),
                yes: InstallModsMessage::InstallModpack(id).into(),
                no: InstallModsMessage::Open.into(),
            };
            Task::none()
        } else {
            Task::perform(
                async move {
                    store::download_mod(&id, &selected_instance, None)
                        .await
                        .map(|not_allowed| (id, not_allowed))
                },
                |n| InstallModsMessage::DownloadComplete(n.strerr()).into(),
            )
        }
    }
}

impl MenuModsDownload {
    pub fn search_store(&mut self, server_side: bool, offset: usize) -> Task<Message> {
        if offset == 0 {
            self.results = None;
            self.scroll_offset = AbsoluteOffset::default();
        }

        let cats = &self.search.categories;

        let categories = cats
            .selected
            .iter()
            .filter_map(|slug| {
                cats.categories
                    .as_ref()
                    .ok()
<<<<<<< mod-store-improvements
                    .and_then(|cats| cats.iter().filter_map(|n| n.search_for_slug(slug)).next())
=======
                    .and_then(|categories| categories.iter().find_map(|n| n.search_for_slug(slug)))
>>>>>>> main
                    .cloned()
            })
            .collect();

        let s = &self.search;
        let version = self.version_json.get_id().to_owned();
        let loader = self.config.mod_type;
        let query = Query {
            name: s.term.clone(),
            kind: s.query_type,
            open_source: s.force_open_source,
            sort_by: s.sort_by,
            sort_ascending: s.sort_ascending,
            categories_use_all: s.categories.use_all,
            version,
            loader,
            server_side,
            categories,
        };
        Task::perform(store::search(query, offset, s.backend), |n| {
            InstallModsMessage::SearchResult(n.strerr()).into()
        })
    }

    pub fn fetch_description_cached(&self, index: usize) -> Option<Task<Message>> {
        let results = self.results.as_ref()?;
        let hit = results.mods.get(index).expect("index came from iterator");
        let id = hit.get_id();

        if self.mod_descriptions.contains_key(&id) {
            // Already fetched
            return None;
        }

        let t1 = Task::perform(get_description(id.clone()), |n| {
            InstallModsMessage::LoadedDescription(n.strerr()).into()
        });

        let id2 = id.clone();
        let t2 = Task::perform(async move { store::get_info(&id2).await }, move |n| {
            let id = id.clone();
            InstallModsMessage::LoadedExtendedInfo(n.strerr().map(move |n| (id, n))).into()
        });

        Some(Task::batch([t1, t2]))
    }

    pub fn reload_description(&mut self, images: &mut ImageState) {
        let (Some(selection), Some(results)) = (self.opened_mod, &self.results) else {
            return;
        };
        let Some(hit) = results.mods.get(selection) else {
            return;
        };
        let Some(info) = self.mod_descriptions.get(&hit.get_id()) else {
            return;
        };
        let description = match results.backend {
            StoreBackendType::Modrinth => MarkState::with_html_and_markdown(info),
            StoreBackendType::Curseforge => MarkState::with_html(info), // Optimization, curseforge only has HTML
        };
        let imgs = description.find_image_links();
        self.description = Some(description);

        for img in imgs {
            images.queue(&img, false);
        }
    }
}
