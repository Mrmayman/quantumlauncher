use iced::Task;
use ql_core::{IntoIoError, IntoStringError};
use ql_mod_manager::store::SelectedMod;
use std::collections::HashSet;

use crate::state::{EditPresetsMessage, Launcher, MenuEditPresets, Message, SelectedState, State};

macro_rules! iflet_manage_preset {
    ($self:ident, $($field:ident),+, { $($code:tt)* }) => {
        if let State::ManagePresets(MenuEditPresets::Selecting {
            $($field,)* ..
        }) = &mut $self.state
        {
            $($code)*
        }
    };
}

impl Launcher {
    pub fn update_edit_presets(&mut self, message: EditPresetsMessage) -> Task<Message> {
        match message {
            EditPresetsMessage::Open => return self.go_to_edit_presets_menu(),
            EditPresetsMessage::ToggleCheckbox((name, id), enable) => {
                iflet_manage_preset!(self, selected_mods, selected_state, {
                    if enable {
                        selected_mods.insert(SelectedMod::Downloaded { name, id });
                    } else {
                        selected_mods.remove(&SelectedMod::Downloaded { name, id });
                    }
                    *selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::ToggleCheckboxLocal(file_name, enable) => {
                iflet_manage_preset!(self, selected_mods, selected_state, {
                    if enable {
                        selected_mods.insert(SelectedMod::Local { file_name });
                    } else {
                        selected_mods.remove(&SelectedMod::Local { file_name });
                    }
                    *selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::SelectAll => {
                self.preset_select_all();
            }
            EditPresetsMessage::ToggleIncludeConfig(enable) => {
                iflet_manage_preset!(self, include_config, {
                    *include_config = enable;
                });
            }
            EditPresetsMessage::LoadedDir(res) => match res {
                Ok(dir) => iflet_manage_preset!(self, mc_dir_entries, {
                    *mc_dir_entries = dir;
                }),
                Err(err) => self.set_error(err),
            },
            EditPresetsMessage::Generate => {
                iflet_manage_preset!(self, selected_mods, include_config, mc_dir_entries, {
                    let selected_instance = self.selected_instance.clone().unwrap();
                    let selected_mods = selected_mods.clone();
                    let include_config = *include_config;
                    let dir = mc_dir_entries.clone();

                    self.state =
                        State::ManagePresets(MenuEditPresets::Loading("Building Preset..."));

                    return Task::perform(
                        ql_mod_manager::presets::generate(
                            selected_instance,
                            selected_mods,
                            dir,
                            include_config,
                        ),
                        |n| EditPresetsMessage::GenerateEnd(n.strerr()).into(),
                    );
                });
            }
            EditPresetsMessage::GenerateEnd(result) => match result.map(|n| self.build_end(n)) {
                Ok(task) => return task,
                Err(err) => self.set_error(err),
            },
            EditPresetsMessage::ImportComplete(result) => {
                match result.map(|not_allowed| {
                    if not_allowed.is_empty() {
                        self.go_to_edit_mods_menu()
                    } else {
                        self.state = State::curseforge_manual_download(not_allowed);
                        Task::none()
                    }
                }) {
                    Ok(n) => return n,
                    Err(err) => self.set_error(err),
                }
            }
        }
        Task::none()
    }

    fn preset_select_all(&mut self) {
        if let State::ManagePresets(MenuEditPresets::Selecting {
            selected_mods,
            selected_state,
            sorted_mods_list,
            ..
        }) = &mut self.state
        {
            match selected_state {
                SelectedState::All => {
                    selected_mods.clear();
                    *selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    *selected_mods = sorted_mods_list
                        .iter()
                        .filter_map(|mod_info| {
                            mod_info
                                .is_manually_installed()
                                .then_some(mod_info.clone().into())
                        })
                        .collect();
                    *selected_state = SelectedState::All;
                }
            }
        }
    }

    pub fn go_to_edit_presets_menu(&mut self) -> Task<Message> {
        let State::EditMods(menu) = &self.state else {
            return Task::none();
        };

        let selected_mods = menu
            .sorted_mods_list
            .iter()
            .filter_map(|n| n.is_manually_installed().then_some(n.clone().into()))
            .collect::<HashSet<_>>();

        let menu = MenuEditPresets::Selecting {
            selected_mods,
            selected_state: SelectedState::All,
            include_config: true,
            sorted_mods_list: menu.sorted_mods_list.clone(),
            drag_and_drop_hovered: false,
            mc_dir_entries: HashSet::new(),
        };

        self.state = State::ManagePresets(menu);

        let instance = self.selected_instance.clone().unwrap();
        Task::perform(
            async move { ql_mod_manager::presets::get_mc_dir_contents(&instance).await },
            |n| EditPresetsMessage::LoadedDir(n.strerr()).into(),
        )
    }

    fn build_end(&mut self, preset: Vec<u8>) -> Task<Message> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("QuantumLauncher Preset", &["qmp"])
            .set_file_name("my_preset.qmp")
            .set_title("Save your QuantumLauncher Preset")
            .save_file()
        {
            if let Err(err) = std::fs::write(&path, preset).path(&path) {
                self.set_error(err);
            }
            self.go_to_edit_mods_menu()
        } else {
            Task::none()
        }
    }
}
