use iced::{Task, futures::executor::block_on};
use ql_core::{Instance, IntoStringError, JsonFileError, json::InstanceConfigJson};
use ql_mod_manager::store::{ModId, RECOMMENDED_MODS, RecommendedMod};

use crate::state::{
    InfoMessage, Launcher, MenuCurseforgeManualDownload, MenuRecommendedMods, Message, ProgressBar,
    RecommendedModMessage, State,
};

impl Launcher {
    pub fn update_recommended_mods(&mut self, msg: RecommendedModMessage) -> Task<Message> {
        match msg {
            RecommendedModMessage::Open => {
                return self.go_to_recommended_mods();
            }
            RecommendedModMessage::ModCheckResult(res) => match res {
                Ok(mods) => {
                    let instance = self.instance();
                    let config = match if let State::RecommendedMods(menu) = &self.state {
                        menu.get_config(instance)
                    } else {
                        block_on(InstanceConfigJson::read(instance))
                    } {
                        Ok(c) => c,
                        Err(e) => {
                            self.set_error(e);
                            return Task::none();
                        }
                    };
                    self.state = State::RecommendedMods(if mods.is_empty() {
                        MenuRecommendedMods::NotSupported
                    } else {
                        MenuRecommendedMods::Loaded {
                            mods: mods
                                .into_iter()
                                .map(|n| (n.enabled_by_default, n))
                                .collect(),
                            config,
                        }
                    });
                }
                Err(err) => self.set_error(err),
            },
            RecommendedModMessage::Toggle(idx, toggle) => {
                if let State::RecommendedMods(MenuRecommendedMods::Loaded { mods, .. }) =
                    &mut self.state
                {
                    if let Some((t, _)) = mods.get_mut(idx) {
                        *t = toggle;
                    }
                }
            }
            RecommendedModMessage::Download => {
                if let State::RecommendedMods(MenuRecommendedMods::Loaded { mods, config }) =
                    &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    let ids: Vec<ModId> = mods
                        .iter()
                        .filter(|n| n.0)
                        .map(|n| ModId::from_pair(n.1.id, n.1.backend))
                        .collect();

                    self.state = State::RecommendedMods(MenuRecommendedMods::Loading {
                        progress: ProgressBar::with_recv(receiver),
                        config: config.clone(),
                    });

                    let instance = self.selected_instance.clone().unwrap();

                    return Task::perform(
                        ql_mod_manager::store::download_mods_bulk(ids, instance, Some(sender)),
                        |n| RecommendedModMessage::DownloadEnd(n.strerr()).into(),
                    );
                }
            }
            RecommendedModMessage::DownloadEnd(result) => match result {
                Ok(not_allowed) => {
                    if not_allowed.is_empty() {
                        return self.go_to_edit_mods_menu(Some(InfoMessage::success(
                            "Downloaded recommended mods",
                        )));
                    }
                    self.state = State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                        not_allowed,
                        delete_mods: true,
                    });
                }
                Err(err) => self.set_error(err),
            },
        }
        Task::none()
    }

    fn go_to_recommended_mods(&mut self) -> Task<Message> {
        let config = if let State::EditMods(menu) = &self.state {
            menu.file_data.config.clone()
        } else {
            match block_on(InstanceConfigJson::read(self.instance())) {
                Ok(n) => n,
                Err(err) => {
                    self.set_error(err);
                    return Task::none();
                }
            }
        };
        let (sender, recv) = std::sync::mpsc::channel();
        let progress = ProgressBar::with_recv(recv);
        self.state = State::RecommendedMods(MenuRecommendedMods::Loading {
            progress,
            config: config.clone(),
        });
        let loader = config.mod_type;
        if loader.is_vanilla() {
            self.state = State::RecommendedMods(MenuRecommendedMods::InstallALoader);
            return Task::none();
        }
        let ids = RECOMMENDED_MODS.to_owned();
        Task::perform(
            RecommendedMod::get_compatible_mods(
                ids,
                self.selected_instance.clone().unwrap(),
                loader,
                sender,
            ),
            |n| RecommendedModMessage::ModCheckResult(n.strerr()).into(),
        )
    }
}

impl MenuRecommendedMods {
    fn get_config(&self, instance: &Instance) -> Result<InstanceConfigJson, JsonFileError> {
        if let MenuRecommendedMods::Loaded { config, .. }
        | MenuRecommendedMods::Loading { config, .. } = self
        {
            Ok(config.clone())
        } else {
            block_on(InstanceConfigJson::read(instance))
        }
    }
}
