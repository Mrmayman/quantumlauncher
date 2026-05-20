use iced::Task;
use ql_core::{IntoIoError, IntoStringError};
use ql_mod_manager::store::{QueryType, SelectedMod};
use std::collections::HashSet;

use crate::state::{
    EditPresetsMessage, InfoMessage, Launcher, MenuCurseforgeManualDownload, MenuEditPresets,
    Message, SelectedState, State,
};

macro_rules! iflet_manage_preset {
    ($self:ident, $($field:ident),+, { $($code:tt)* }) => {
        if let State::ManagePresets(MenuEditPresets {
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
            EditPresetsMessage::Open => self.go_to_edit_presets_menu(),
            EditPresetsMessage::ToggleCheckbox(name, id, enable) => {
                iflet_manage_preset!(self, selected_mods, selected_state, {
                    if enable {
                        selected_mods.insert(SelectedMod::Downloaded { name, id });
                    } else {
                        selected_mods.remove(&SelectedMod::Downloaded { name, id });
                    }
                    *selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::ToggleCheckboxLocal(local, enable) => {
                iflet_manage_preset!(self, selected_mods, selected_state, {
                    let m = SelectedMod::Local(local);
                    if enable {
                        selected_mods.insert(m);
                    } else {
                        selected_mods.remove(&m);
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
            EditPresetsMessage::BuildYourOwn => {
                iflet_manage_preset!(self, selected_mods, is_building, include_config, {
                    *is_building = true;
                    let selected_instance = self.selected_instance.clone().unwrap();
                    let selected_mods = selected_mods.clone();
                    let include_config = *include_config;
                    return Task::perform(
                        ql_mod_manager::Preset::generate(
                            selected_instance,
                            selected_mods,
                            include_config,
                        ),
                        |n| EditPresetsMessage::BuildYourOwnEnd(n.strerr()).into(),
                    );
                });
            }
            EditPresetsMessage::BuildYourOwnEnd(result) => {
                match result.map(|n| self.build_end(n)) {
                    Ok(task) => return task,
                    Err(err) => self.set_error(err),
                }
            }
            EditPresetsMessage::LoadComplete(result) => {
                match result.map(|not_allowed| {
                    if not_allowed.is_empty() {
                        self.go_to_edit_mods_menu(Some(InfoMessage::success("Imported mod preset")))
                    } else {
                        self.state =
                            State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                                not_allowed,
                                delete_mods: true,
                            });
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
        if let State::ManagePresets(MenuEditPresets {
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

    pub fn go_to_edit_presets_menu(&mut self) {
        let State::EditMods(menu) = &self.state else {
            return;
        };

        // I'm anyway overhauling this thing in another PR
        let mods = || {
            menu.sorted_mods_list
                .iter()
                .filter(|n| n.project_type() == QueryType::Mods)
        };

        let selected_mods = mods()
            .filter_map(|n| n.is_manually_installed().then_some(n.clone().into()))
            .collect::<HashSet<_>>();

        let menu = MenuEditPresets {
            selected_mods,
            selected_state: SelectedState::All,
            is_building: false,
            include_config: true,
            progress: None,
            sorted_mods_list: mods().cloned().collect(),
            drag_and_drop_hovered: false,
        };

        self.state = State::ManagePresets(menu);
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
            self.go_to_edit_mods_menu(Some(InfoMessage::success("Created Preset")))
        } else {
            Task::none()
        }
    }
}
