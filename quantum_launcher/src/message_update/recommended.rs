use iced::{futures::executor::block_on, Task};
use ql_core::{json::InstanceConfigJson, InstanceSelection, IntoStringError, JsonFileError, ModId};
use ql_mod_manager::store::{RecommendedMod, RECOMMENDED_MODS};

use crate::{
    sip,
    state::{Launcher, MenuRecommendedMods, Message, ProgressBar, RecommendedModMessage, State},
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
                    let ids: Vec<ModId> = mods
                        .iter()
                        .filter(|n| n.0)
                        .map(|n| ModId::from_pair(n.1.id, n.1.backend))
                        .collect();

                    self.state = State::RecommendedMods(MenuRecommendedMods::Loading {
                        progress: ProgressBar::new(),
                        config: config.clone(),
                    });

                    let instance = self.selected_instance.clone().unwrap();

                    return sip(
                        |sender| {
                            ql_mod_manager::store::download_mods_bulk(ids, instance, Some(sender))
                        },
                        |n| {
                            Message::RecommendedMods(RecommendedModMessage::DownloadEnd(n.strerr()))
                        },
                    );
                }
            }
            RecommendedModMessage::DownloadEnd(result) => {
                match result {
                    Ok(mods) => {
                        // If any restrictive mods ended up in our
                        // official download list, that would be a major
                        // skill issue from our end.
                        // No need for manual download UI, such mods
                        // don't deserve to be recommended anyway.
                        debug_assert!(mods.is_empty());
                        return self.go_to_edit_mods_menu(false);
                    }
                    Err(err) => self.set_error(err),
                }
            }
        }
        Task::none()
    }

    fn go_to_recommended_mods(&mut self) -> Task<Message> {
        let config = if let State::EditMods(menu) = &self.state {
            menu.config.clone()
        } else {
            match block_on(InstanceConfigJson::read(self.instance())) {
                Ok(n) => n,
                Err(err) => {
                    self.set_error(err);
                    return Task::none();
                }
            }
        };
        self.state = State::RecommendedMods(MenuRecommendedMods::Loading {
            progress: ProgressBar::new(),
            config: config.clone(),
        });
        let loader = config.mod_type;
        if loader.is_vanilla() {
            self.state = State::RecommendedMods(MenuRecommendedMods::InstallALoader);
            return Task::none();
        }
        let ids = RECOMMENDED_MODS.to_owned();
        let instance = self.selected_instance.clone().unwrap();
        sip(
            move |sender| RecommendedMod::get_compatible_mods(ids, instance, loader, sender),
            |n| Message::RecommendedMods(RecommendedModMessage::ModCheckResult(n.strerr())),
        )
    }
}

impl MenuRecommendedMods {
    pub fn get_config(
        &self,
        instance: &InstanceSelection,
    ) -> Result<InstanceConfigJson, JsonFileError> {
        if let MenuRecommendedMods::Loaded { config, .. }
        | MenuRecommendedMods::Loading { config, .. } = self
        {
            Ok(config.clone())
        } else {
            block_on(InstanceConfigJson::read(instance))
        }
    }
}
