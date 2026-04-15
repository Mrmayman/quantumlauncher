use iced::Task;
use ql_core::{IntoIoError, IntoStringError};
use ql_mod_manager::{presets::SOFT_EXCEPTIONS, store::SelectedMod};
use std::collections::HashSet;

use crate::state::{
    EditPresetsMessage, InfoMessage, Launcher, MenuEditPresets, Message, SelectedState, State,
};

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
            EditPresetsMessage::ModToggle((name, id), enable) => {
                iflet_manage_preset!(self, mods_selected, mods_selected_state, {
                    if enable {
                        mods_selected.insert(SelectedMod::Downloaded { name, id });
                    } else {
                        mods_selected.remove(&SelectedMod::Downloaded { name, id });
                    }
                    *mods_selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::ModToggleLocal(file_name, enable) => {
                iflet_manage_preset!(self, mods_entries, mods_selected, mods_selected_state, {
                    if enable {
                        mods_selected.insert(SelectedMod::Local { file_name });
                    } else {
                        mods_selected.remove(&SelectedMod::Local { file_name });
                    }
                    *mods_selected_state =
                        SelectedState::compute(mods_selected.len(), mods_entries.len());
                });
            }
            EditPresetsMessage::DirToggle(name, enable) => {
                iflet_manage_preset!(
                    self,
                    mc_dir_entries,
                    mc_dir_selected,
                    mc_dir_selected_state,
                    {
                        if enable {
                            mc_dir_selected.insert(name);
                        } else {
                            mc_dir_selected.remove(&name);
                        }
                        *mc_dir_selected_state =
                            SelectedState::compute(mc_dir_selected.len(), mc_dir_entries.len());
                    }
                );
            }
            EditPresetsMessage::DirSelectAll => self.preset_dir_select_all(),
            EditPresetsMessage::ModSelectAll => self.preset_mod_select_all(),

            EditPresetsMessage::ModIncludeConfig(enable) => {
                iflet_manage_preset!(self, include_config, {
                    *include_config = enable;
                });
            }
            EditPresetsMessage::LoadedDir(res) => match res {
                Ok(dir) => {
                    iflet_manage_preset!(
                        self,
                        mc_dir_entries,
                        mc_dir_selected,
                        mc_dir_selected_state,
                        {
                            *mc_dir_selected = dir
                                .iter()
                                .filter(|n| !SOFT_EXCEPTIONS.contains(&n.name.as_str()))
                                .map(|n| n.name.clone())
                                .collect();
                            *mc_dir_entries = dir;
                            *mc_dir_selected_state =
                                SelectedState::compute(mc_dir_selected.len(), mc_dir_entries.len());
                        }
                    )
                }
                Err(err) => self.set_error(err),
            },
            EditPresetsMessage::Generate => {
                iflet_manage_preset!(self, mods_selected, include_config, mc_dir_entries, {
                    let selected_instance = self.selected_instance.clone().unwrap();
                    let mods_selected = mods_selected.clone();
                    let include_config = *include_config;
                    let dir = mc_dir_entries.clone();

                    self.state =
                        State::ManagePresets(MenuEditPresets::Loading("Building Preset..."));

                    return Task::perform(
                        ql_mod_manager::presets::generate(
                            selected_instance,
                            mods_selected,
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
                        self.go_to_edit_mods_menu(Some(InfoMessage::success("Imported mod preset")))
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

    fn preset_mod_select_all(&mut self) {
        if let State::ManagePresets(MenuEditPresets::Selecting {
            mods_selected,
            mods_selected_state,
            mods_entries: mods_sorted_list,
            ..
        }) = &mut self.state
        {
            match mods_selected_state {
                SelectedState::All => {
                    mods_selected.clear();
                    *mods_selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    *mods_selected = mods_sorted_list
                        .iter()
                        .filter_map(|mod_info| {
                            mod_info
                                .is_manually_installed()
                                .then_some(mod_info.clone().into())
                        })
                        .collect();
                    *mods_selected_state = SelectedState::All;
                }
            }
        }
    }

    fn preset_dir_select_all(&mut self) {
        if let State::ManagePresets(MenuEditPresets::Selecting {
            mc_dir_selected,
            mc_dir_selected_state,
            mc_dir_entries,
            ..
        }) = &mut self.state
        {
            match mc_dir_selected_state {
                SelectedState::All => {
                    mc_dir_selected.clear();
                    *mc_dir_selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    *mc_dir_selected = mc_dir_entries.iter().map(|n| n.name.clone()).collect();
                    *mc_dir_selected_state = SelectedState::All;
                }
            }
        }
    }

    pub fn go_to_edit_presets_menu(&mut self) -> Task<Message> {
        let State::EditMods(menu) = &self.state else {
            return Task::none();
        };

        let mods_selected = menu
            .sorted_mods_list
            .iter()
            .filter_map(|n| n.is_manually_installed().then_some(n.clone().into()))
            .collect::<HashSet<_>>();

        let menu = MenuEditPresets::Selecting {
            mods_selected,
            mods_selected_state: SelectedState::All,
            include_config: true,
            mods_entries: menu.sorted_mods_list.clone(),
            drag_and_drop_hovered: false,
            mc_dir_entries: Vec::new(),
            mc_dir_selected: HashSet::new(),
            mc_dir_selected_state: SelectedState::None,
        };

        self.state = State::ManagePresets(menu);

        let instance = self.selected_instance.clone().unwrap();
        Task::perform(
            async move { ql_mod_manager::presets::get_mc_dir_contents(&instance).await },
            |n| EditPresetsMessage::LoadedDir(n.strerr()).into(),
        )
    }

    fn build_end(&mut self, preset: Vec<u8>) -> Task<Message> {
        let save = Task::perform(
            async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("QuantumLauncher Preset", &["qmp"])
                    .set_file_name("my_preset.qmp")
                    .set_title("Save your QuantumLauncher Preset")
                    .save_file()
                    .await
                {
                    _ = tokio::fs::write(&file.path(), preset).await;
                }
            },
            |_| Message::Nothing,
        );
        Task::batch([
            self.go_to_edit_mods_menu(Some(InfoMessage::success("Created Preset"))),
            save,
        ])
    }
}
