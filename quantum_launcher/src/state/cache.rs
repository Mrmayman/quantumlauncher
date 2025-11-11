use std::{
    collections::HashMap,
    path::Path,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
};

use dashmap::DashMap;
use iced::Task;
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, Watcher,
};
use ql_core::{
    err_no_log,
    json::{InstanceConfigJson, VersionDetails},
    InstanceSelection, IntoStringError, LAUNCHER_DIR,
};

use crate::state::{get_entries, CacheMessage, Message};

pub struct PathWatcher {
    receiver: Receiver<notify::Event>,
    _watcher: notify::RecommendedWatcher,
}

impl PathWatcher {
    pub fn new<P: AsRef<Path>>(path: P, is_dir: bool) -> notify::Result<Self> {
        let path = path.as_ref();
        let (sender, receiver) = mpsc::channel();

        if is_dir && !path.exists() {
            _ = std::fs::create_dir_all(path);
        }

        let mut watcher: notify::RecommendedWatcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                _ = sender.send(event);
            }
        })?;
        watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

        Ok(Self {
            receiver,
            _watcher: watcher,
        })
    }

    pub fn tick(&self) -> bool {
        self.receiver
            .try_recv()
            .is_ok_and(|e| !matches!(e.kind, EventKind::Access(AccessKind::Open(AccessMode::Any))))
    }
}

pub struct InstanceInfoWatcher {
    details: PathWatcher,
    config: PathWatcher,
}

impl std::fmt::Debug for InstanceInfoWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailsAndConfig").finish()
    }
}

#[derive(Default)]
pub struct InstanceCache {
    pub client_list: Option<Vec<String>>,
    pub server_list: Option<Vec<String>>,
    watch_clients: Option<PathWatcher>,
    watch_servers: Option<PathWatcher>,
    pub watch_info: HashMap<InstanceSelection, Arc<Mutex<InstanceInfoWatcher>>>,

    pub config: DashMap<InstanceSelection, InstanceConfigJson>,
    pub details: DashMap<InstanceSelection, VersionDetails>,
}

impl InstanceCache {
    pub fn new() -> (Self, Task<Message>) {
        let watch_clients = match PathWatcher::new(LAUNCHER_DIR.join("instances"), true) {
            Ok(n) => Some(n),
            Err(err) => {
                err_no_log!("While watching ./instances/ dir: {err}");
                None
            }
        };

        let servers = LAUNCHER_DIR.join("servers");
        let watch_servers = match PathWatcher::new(&servers, true) {
            Ok(n) => Some(n),
            Err(err) => {
                err_no_log!("While watching ./servers/ dir: {err}");
                None
            }
        };

        (
            Self {
                watch_clients,
                watch_servers,
                ..Default::default()
            },
            Task::batch([
                Task::perform(get_entries(false), |n| {
                    Message::CoreCache(CacheMessage::List(n))
                }),
                Task::perform(get_entries(true), |n| {
                    Message::CoreCache(CacheMessage::List(n))
                }),
            ]),
        )
    }

    pub fn get_list(&self, is_server: bool) -> Option<&Vec<String>> {
        if is_server {
            self.server_list.as_ref()
        } else {
            self.client_list.as_ref()
        }
    }

    pub fn set_list(&mut self, list: Vec<String>, is_server: bool) -> Task<Message> {
        self.watch_info.clear();
        let mut tasks = Vec::new();

        for item in &list {
            let instance = InstanceSelection::new(item, is_server);
            let i2 = instance.clone();

            tasks.push(Task::perform(
                async move { load_info_watchers(&i2).await },
                move |n| {
                    n.map(|(watcher, config, details)| {
                        Message::Multiple(vec![
                            Message::CoreCache(CacheMessage::DetailsAndConfigWatcher(
                                instance.clone(),
                                Arc::new(Mutex::new(watcher)),
                            )),
                            Message::CoreCache(CacheMessage::Config(
                                instance.clone(),
                                Ok(Box::new(config)),
                            )),
                            Message::CoreCache(CacheMessage::Details(
                                instance.clone(),
                                Ok(Box::new(details)),
                            )),
                        ])
                    })
                    .unwrap_or_else(|err| Message::CoreCache(CacheMessage::PopupError(err)))
                },
            ));
        }

        if is_server {
            self.server_list = Some(list);
        } else {
            self.client_list = Some(list);
        }

        Task::batch(tasks)
    }

    pub fn force_update_list(&mut self) -> Task<Message> {
        self.watch_info.clear();
        Task::batch([
            Task::perform(get_entries(false), |n| {
                Message::CoreCache(CacheMessage::List(n))
            }),
            Task::perform(get_entries(true), |n| {
                Message::CoreCache(CacheMessage::List(n))
            }),
        ])
    }

    pub fn update(&mut self, update_config: bool) -> Task<Message> {
        let mut tasks = Vec::new();

        if let Some(w) = &self.watch_clients {
            if w.tick() {
                self.watch_info.clear();
                tasks.push(Task::perform(get_entries(false), |n| {
                    Message::CoreCache(CacheMessage::List(n))
                }));
            }
        }
        if let Some(w) = &self.watch_servers {
            if w.tick() {
                self.watch_info.clear();
                tasks.push(Task::perform(get_entries(true), |n| {
                    Message::CoreCache(CacheMessage::List(n))
                }));
            }
        }

        for (instance, w) in &self.watch_info {
            let watcher = w.lock().unwrap();
            if watcher.details.tick() {
                let instance = instance.clone();
                let i2 = instance.clone();
                tasks.push(Task::perform(
                    async move { VersionDetails::load(&i2).await },
                    move |n| {
                        Message::CoreCache(CacheMessage::Details(
                            instance.clone(),
                            n.strerr().map(Box::new),
                        ))
                    },
                ));
            }
            if update_config && watcher.config.tick() {
                let instance = instance.clone();
                let i2 = instance.clone();
                tasks.push(Task::perform(
                    async move { InstanceConfigJson::load(&i2).await },
                    move |n| {
                        Message::CoreCache(CacheMessage::Config(
                            instance.clone(),
                            n.strerr().map(Box::new),
                        ))
                    },
                ));
            }
        }

        Task::batch(tasks)
    }
}

async fn load_info_watchers(
    instance: &InstanceSelection,
) -> Result<(InstanceInfoWatcher, InstanceConfigJson, VersionDetails), String> {
    let idir = instance.get_instance_path();
    let cf = idir.join("config.json");
    let dt = idir.join("details.json");
    let (wconfig, wdetails, config, details) = tokio::join!(
        tokio::task::spawn_blocking(move || PathWatcher::new(cf, false)),
        tokio::task::spawn_blocking(move || PathWatcher::new(dt, false)),
        InstanceConfigJson::load(instance),
        VersionDetails::load(instance)
    );
    Ok((
        InstanceInfoWatcher {
            config: wconfig.strerr()?.strerr()?,
            details: wdetails.strerr()?.strerr()?,
        },
        config.strerr()?,
        details.strerr()?,
    ))
}
