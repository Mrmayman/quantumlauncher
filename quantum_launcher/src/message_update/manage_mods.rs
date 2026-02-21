use iced::{futures::executor::block_on, keyboard::Modifiers};
use iced::{widget, Task};
use ql_core::{
    err, jarmod::JarMods, InstanceSelection, IntoIoError, IntoStringError, ModId, SelectedMod,
};
use ql_mod_manager::store::ModIndex;
use std::{collections::HashSet, path::PathBuf};

use crate::state::{
    AutoSaveKind, ExportModsMessage, Launcher, ManageJarModsMessage, ManageModsMessage,
    MenuCurseforgeManualDownload, MenuEditJarMods, MenuEditMods, MenuEditModsModal, Message,
    ProgressBar, SelectedState, State,
};

impl Launcher {
    pub fn update_manage_mods(&mut self, msg: ManageModsMessage) -> Task<Message> {
        match msg {
            ManageModsMessage::ScreenOpen => return self.go_to_edit_mods_menu(true),
            ManageModsMessage::ScreenOpenWithoutUpdate => {
                return self.go_to_edit_mods_menu(false);
            }

            ManageModsMessage::AddFileDone(Err(err))
            | ManageModsMessage::DeleteFinished(Err(err))
            | ManageModsMessage::LocalDeleteFinished(Err(err))
            | ManageModsMessage::ToggleFinished(Err(err))
            | ManageModsMessage::UpdateModsFinished(Err(err)) => self.set_error(err),

            ManageModsMessage::ListScrolled(offset) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.list_scroll = offset;
                }
            }
            ManageModsMessage::SelectEnsure(name, id) => {
                let State::EditMods(menu) = &mut self.state else {
                    return Task::none();
                };
                let selected_mod = SelectedMod::from_pair(name, id);
                menu.list_shift_index = Some(menu.index(&selected_mod));
                menu.shift_selected_mods.clear();
                menu.selected_mods.clear();
                menu.selected_mods.insert(selected_mod);
                menu.update_selected_state();
                return menu.scroll_fix();
            }
            ManageModsMessage::SelectMod(name, id) => {
                let State::EditMods(menu) = &mut self.state else {
                    return Task::none();
                };

                let selected_mod = SelectedMod::from_pair(name, id);

                let pressed_ctrl = self.modifiers_pressed.contains(Modifiers::COMMAND);
                let pressed_shift = self.modifiers_pressed.contains(Modifiers::SHIFT);

                menu.select_mod(selected_mod, pressed_ctrl, pressed_shift);
                menu.update_selected_state();
                return menu.scroll_fix();
            }
            ManageModsMessage::AddFile(delete_file) => {
                return self.add_file_select(delete_file);
            }
            ManageModsMessage::AddFileDone(Ok(unsupported)) => {
                if !unsupported.is_empty() {
                    self.state = State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                        unsupported,
                        is_store: false,
                        delete_mods: true,
                    });
                }
                return self.go_to_edit_mods_menu(false);
            }
            ManageModsMessage::DeleteSelected => {
                if let State::EditMods(menu) = &mut self.state {
                    let selected_instance = self.selected_instance.clone().unwrap();
                    let mods_dir = selected_instance.get_dot_minecraft_path().join("mods");
                    let command = Self::get_delete_mods_command(selected_instance, menu);

                    let local_mods_paths: Vec<&String> = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Local { file_name } = s_mod {
                                Some(file_name)
                            } else {
                                None
                            }
                        })
                        .collect();
                    for path in &local_mods_paths {
                        menu.locally_installed_mods.remove(*path);
                    }
                    let delete_local_command = Task::batch(
                        local_mods_paths
                            .into_iter()
                            .map(|n| mods_dir.join(n))
                            .map(delete_file_wrapper)
                            .map(|n| {
                                Task::perform(n, |n| {
                                    Message::ManageMods(ManageModsMessage::LocalDeleteFinished(n))
                                })
                            }),
                    );

                    return Task::batch([command, delete_local_command]);
                }
            }
            ManageModsMessage::DeleteOptiforge(name) => {
                let mods_dir = self.get_selected_dot_minecraft_dir().unwrap().join("mods");
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods.remove(&name);
                    if let Some(mod_info) = &mut menu.config.mod_type_info {
                        if mod_info.optifine_jar.as_ref().is_some_and(|n| n == &name) {
                            mod_info.optifine_jar = None;
                            if let Err(err) =
                                block_on(menu.config.save(self.selected_instance.as_ref().unwrap()))
                            {
                                self.set_error(err);
                            }
                        }
                    }
                }
                return Task::perform(delete_file_wrapper(mods_dir.join(&name)), |n| {
                    Message::ManageMods(ManageModsMessage::LocalDeleteFinished(n))
                });
            }
            ManageModsMessage::DeleteFinished(Ok(_)) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.selected_mods.clear();
                }
                self.update_mod_index();
            }
            ManageModsMessage::LocalDeleteFinished(Ok(())) => {}
            ManageModsMessage::LocalIndexLoaded(hash_set) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods = hash_set;
                }
            }
            ManageModsMessage::ToggleSelected => return self.manage_mods_toggle_selected(),

            ManageModsMessage::ToggleFinished(Ok(())) => self.update_mod_index(),
            ManageModsMessage::UpdateMods => return self.update_mods(),
            ManageModsMessage::UpdateModsFinished(Ok(())) => {
                self.mod_updates_checked
                    .insert(self.selected_instance.clone().unwrap(), Vec::new());
                self.update_mod_index();
                if let State::EditMods(menu) = &mut self.state {
                    menu.available_updates.clear();
                }
            }
            ManageModsMessage::UpdateCheckResult(updates) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.update_check_handle = None;
                    match updates {
                        Ok(updates) => {
                            let available_updates: Vec<(ModId, String, bool)> =
                                updates.into_iter().map(|(a, b)| (a, b, true)).collect();
                            self.mod_updates_checked.insert(
                                self.selected_instance.clone().unwrap(),
                                available_updates.clone(),
                            );
                            menu.available_updates = available_updates;
                        }
                        Err(err) => {
                            err!(no_log, "Could not check for updates: {err}");
                        }
                    }
                }
            }
            ManageModsMessage::UpdateCheckToggle(idx, t) => {
                if let State::EditMods(MenuEditMods {
                    available_updates, ..
                }) = &mut self.state
                {
                    if let Some((_, _, b)) = available_updates.get_mut(idx) {
                        *b = t;
                    }
                }
            }
            ManageModsMessage::SelectAll => {
                if let State::EditMods(menu) = &mut self.state {
                    match menu.selected_state {
                        SelectedState::All => {
                            menu.selected_mods.clear();
                            menu.selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            menu.selected_mods = menu
                                .mods
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info
                                        .manually_installed
                                        .then_some(SelectedMod::Downloaded {
                                            name: mod_info.name.clone(),
                                            id: ModId::from_index_str(id),
                                        })
                                })
                                .chain(menu.locally_installed_mods.iter().map(|n| {
                                    SelectedMod::Local {
                                        file_name: n.clone(),
                                    }
                                }))
                                .collect();
                            menu.selected_state = SelectedState::All;
                        }
                    }
                }
            }
            ManageModsMessage::ExportMenuOpen => {
                if let State::EditMods(menu) = &mut self.state {
                    // Navigate to the export menu with the current selection and mod data
                    use crate::state::MenuExportMods;

                    self.state = State::ExportMods(MenuExportMods {
                        selected_mods: if menu.selected_mods.is_empty() {
                            menu.mods
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info
                                        .manually_installed
                                        .then_some(SelectedMod::Downloaded {
                                            name: mod_info.name.clone(),
                                            id: ModId::from_index_str(id),
                                        })
                                })
                                .chain(menu.locally_installed_mods.iter().map(|n| {
                                    SelectedMod::Local {
                                        file_name: n.clone(),
                                    }
                                }))
                                .collect()
                        } else {
                            menu.selected_mods.clone()
                        },
                    });
                }
            }
            ManageModsMessage::SetModal(modal) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.modal = modal;
                }
            }
            ManageModsMessage::SetSearch(search) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.search = search;
                }
            }
            ManageModsMessage::CurseforgeManualToggleDelete(t) => {
                if let State::CurseforgeManualDownload(menu) = &mut self.state {
                    menu.delete_mods = t;
                }
            }
            ManageModsMessage::RightClick(clicked_id) => {
                if let State::EditMods(menu) = &mut self.state {
                    if let Some(MenuEditModsModal::RightClick(old_id, _)) = &menu.modal {
                        if *old_id == clicked_id {
                            menu.modal = None;
                        } else {
                            menu.modal = Some(MenuEditModsModal::RightClick(
                                clicked_id,
                                self.window_state.mouse_pos,
                            ));
                        }
                    } else {
                        menu.modal = Some(MenuEditModsModal::RightClick(
                            clicked_id,
                            self.window_state.mouse_pos,
                        ));
                    }
                    return menu.scroll_fix();
                }
            }
        }
        Task::none()
    }

    fn manage_mods_toggle_selected(&mut self) -> Task<Message> {
        let State::EditMods(menu) = &mut self.state else {
            return Task::none();
        };
        let (ids_downloaded, ids_local) = menu.get_kinds_of_ids();
        let instance_name = self.selected_instance.clone().unwrap();

        // menu.selected_mods.clear();
        // menu.selected_state = SelectedState::None;

        menu.selected_mods.retain(|n| {
            if let SelectedMod::Local { file_name } = n {
                !ids_local.contains(file_name)
            } else {
                true
            }
        });
        menu.selected_mods
            .extend(ids_local.iter().map(|n| SelectedMod::Local {
                file_name: ql_mod_manager::store::flip_filename(n),
            }));

        let toggle_downloaded = Task::perform(
            ql_mod_manager::store::toggle_mods(ids_downloaded.clone(), instance_name.clone()),
            |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
        );
        let toggle_local = Task::perform(
            ql_mod_manager::store::toggle_mods_local(ids_local, instance_name.clone()),
            |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
        )
        .chain(MenuEditMods::update_locally_installed_mods(
            &menu.mods,
            &instance_name,
        ));

        Task::batch([toggle_downloaded, toggle_local])
    }

    fn add_file_select(&mut self, delete_file: bool) -> Task<Message> {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter("Mod/Modpack", &["jar", "zip", "mrpack", "qmp"])
            .set_title("Add Mod, Modpack or Preset")
            .pick_files()
        else {
            return Task::none();
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        let selected_instance = self.selected_instance.as_ref().unwrap();

        self.state = State::ImportModpack(ProgressBar::with_recv(receiver));
        self.mod_updates_checked.remove(selected_instance);

        // Modpacks being imported
        if paths
            .iter()
            .filter_map(|n| n.extension())
            .any(|n| !n.eq_ignore_ascii_case("jar"))
        {
            self.mod_updates_checked.remove(selected_instance);
        }

        let files_task = Task::perform(
            ql_mod_manager::add_files(
                self.selected_instance.clone().unwrap(),
                paths.clone(),
                Some(sender),
            ),
            move |n| Message::ManageMods(ManageModsMessage::AddFileDone(n.strerr())),
        );
        if delete_file {
            files_task.chain(Task::perform(
                async move {
                    for path in paths {
                        _ = tokio::fs::remove_file(&path).await;
                    }
                },
                |()| Message::Nothing,
            ))
        } else {
            files_task
        }
    }

    fn get_delete_mods_command(
        selected_instance: InstanceSelection,
        menu: &MenuEditMods,
    ) -> Task<Message> {
        let ids: Vec<ModId> = menu
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        Task::perform(
            ql_mod_manager::store::delete_mods(ids, selected_instance),
            |n| Message::ManageMods(ManageModsMessage::DeleteFinished(n.strerr())),
        )
    }

    fn update_mod_index(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            match block_on(ModIndex::load(self.selected_instance.as_ref().unwrap())).strerr() {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
            }
        }
    }

    pub fn update_manage_jar_mods(&mut self, msg: ManageJarModsMessage) -> Task<Message> {
        match msg {
            ManageJarModsMessage::Open => match block_on(JarMods::read(self.instance())) {
                Ok(jarmods) => {
                    self.state = State::EditJarMods(MenuEditJarMods {
                        jarmods,
                        selected_state: SelectedState::None,
                        selected_mods: HashSet::new(),
                        drag_and_drop_hovered: false,
                    });
                    self.autosave.remove(&AutoSaveKind::Jarmods);
                }
                Err(err) => self.set_error(err),
            },
            ManageJarModsMessage::AddFile => {
                self.manage_jarmods_add_file_from_picker();
            }
            ManageJarModsMessage::ToggleCheckbox(name, enable) => {
                self.manage_jarmods_toggle_checkbox(name, enable);
            }
            ManageJarModsMessage::DeleteSelected => {
                self.manage_jarmods_delete_selected();
            }
            ManageJarModsMessage::ToggleSelected => {
                self.manage_jarmods_toggle_selected();
            }
            ManageJarModsMessage::SelectAll => {
                self.manage_jarmods_select_all();
            }
            ManageJarModsMessage::AutosaveFinished((res, jarmods)) => {
                if let Err(err) = res {
                    self.set_error(format!("While autosaving jarmods index: {err}"));
                } else if let State::EditJarMods(menu) = &mut self.state {
                    // Some cleanup of jarmods state may happen during autosave
                    menu.jarmods = jarmods;
                    self.autosave.remove(&AutoSaveKind::Jarmods);
                }
            }

            ManageJarModsMessage::MoveUp | ManageJarModsMessage::MoveDown => {
                self.manage_jarmods_move_up_or_down(&msg);
            }
        }
        Task::none()
    }

    fn manage_jarmods_move_up_or_down(&mut self, msg: &ManageJarModsMessage) {
        if let State::EditJarMods(menu) = &mut self.state {
            let mut selected: Vec<usize> = menu
                .selected_mods
                .iter()
                .filter_map(|selected_name| {
                    menu.jarmods
                        .mods
                        .iter()
                        .enumerate()
                        .find_map(|(i, n)| (n.filename == *selected_name).then_some(i))
                })
                .collect();
            selected.sort_unstable();
            if let ManageJarModsMessage::MoveDown = msg {
                selected.reverse();
            }

            for i in selected {
                if i < menu.jarmods.mods.len() {
                    match msg {
                        ManageJarModsMessage::MoveUp => {
                            if i > 0 {
                                let removed = menu.jarmods.mods.remove(i);
                                menu.jarmods.mods.insert(i - 1, removed);
                            }
                        }
                        ManageJarModsMessage::MoveDown => {
                            if i + 1 < menu.jarmods.mods.len() {
                                let removed = menu.jarmods.mods.remove(i);
                                menu.jarmods.mods.insert(i + 1, removed);
                            }
                        }
                        _ => {}
                    }
                } else {
                    err!(
                        "Out of bounds in jarmods move up/down: !({i} < len:{})",
                        menu.jarmods.mods.len()
                    );
                }
            }
        }
    }

    fn manage_jarmods_select_all(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            match menu.selected_state {
                SelectedState::All => {
                    menu.selected_mods.clear();
                    menu.selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    menu.selected_mods = menu
                        .jarmods
                        .mods
                        .iter()
                        .map(|mod_info| mod_info.filename.clone())
                        .collect();
                    menu.selected_state = SelectedState::All;
                }
            }
        }
    }

    fn manage_jarmods_toggle_selected(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            for selected in menu.selected_mods.iter() {
                if let Some(jarmod) = menu
                    .jarmods
                    .mods
                    .iter_mut()
                    .find(|n| n.filename == *selected)
                {
                    jarmod.enabled = !jarmod.enabled;
                }
            }
        }
    }

    fn manage_jarmods_delete_selected(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            let jarmods_path = self
                .selected_instance
                .as_ref()
                .unwrap()
                .get_instance_path()
                .join("jarmods");

            for selected in menu.selected_mods.iter() {
                if let Some(n) = menu
                    .jarmods
                    .mods
                    .iter()
                    .enumerate()
                    .find_map(|(i, n)| (n.filename == *selected).then_some(i))
                {
                    menu.jarmods.mods.remove(n);
                }

                let path = jarmods_path.join(selected);
                if path.is_file() {
                    _ = std::fs::remove_file(&path);
                }
            }

            menu.selected_mods.clear();
        }
    }

    fn manage_jarmods_toggle_checkbox(&mut self, name: String, enable: bool) {
        if let State::EditJarMods(menu) = &mut self.state {
            if enable {
                menu.selected_mods.insert(name);
                menu.selected_state = SelectedState::Some;
            } else {
                menu.selected_mods.remove(&name);
                menu.selected_state = if menu.selected_mods.is_empty() {
                    SelectedState::None
                } else {
                    SelectedState::Some
                };
            }
        }
    }

    fn export_mods_markdown(selected_mods: &HashSet<SelectedMod>) -> String {
        let mut markdown_lines = Vec::new();

        for selected_mod in selected_mods {
            match selected_mod {
                SelectedMod::Downloaded { name, id } => {
                    let url = match id {
                        ModId::Modrinth(mod_id) => {
                            format!("https://modrinth.com/mod/{mod_id}")
                        }
                        ModId::Curseforge(mod_id) => {
                            format!("https://www.curseforge.com/projects/{mod_id}")
                        }
                    };
                    markdown_lines.push(format!("- [{name}]({url})"));
                }
                SelectedMod::Local { file_name } => {
                    let display_name = file_name
                        .strip_suffix(".jar")
                        .or_else(|| file_name.strip_suffix(".zip"))
                        .unwrap_or(file_name);
                    markdown_lines.push(display_name.to_string());
                }
            }
        }

        markdown_lines.join("\n")
    }

    fn export_to_file(content: String) -> Task<Message> {
        // Use a file dialog to save the exported content
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save exported mod list")
            .add_filter("Text files", &["txt"])
            .add_filter("Markdown files", &["md"])
            .save_file()
        {
            match std::fs::write(&path, content) {
                Ok(()) => {
                    // Optionally, we could show a success message
                    Task::none()
                }
                Err(_err) => {
                    // Handle the error by setting an error message
                    Task::none() // For now, just return none
                }
            }
        } else {
            Task::none()
        }
    }

    fn manage_jarmods_add_file_from_picker(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("jar/zip", &["jar", "zip"])
            .set_title("Pick a Jar Mod Patch (.jar/.zip)")
            .pick_file()
        {
            if let Some(filename) = path.file_name() {
                let dest = self
                    .instance()
                    .get_instance_path()
                    .join("jarmods")
                    .join(filename);
                if let Err(err) = std::fs::copy(&path, dest) {
                    self.set_error(format!("While picking jar mod to be added: {err}"));
                }
            }
        }
    }

    pub fn update_export_mods(&mut self, msg: ExportModsMessage) -> Task<Message> {
        match msg {
            ExportModsMessage::ExportAsPlainText => {
                if let State::ExportMods(menu) = &self.state {
                    return Self::export_to_file(Self::export_mods_plain_text(&menu.selected_mods));
                }
            }
            ExportModsMessage::ExportAsMarkdown => {
                if let State::ExportMods(menu) = &self.state {
                    return Self::export_to_file(Self::export_mods_markdown(&menu.selected_mods));
                }
            }
            ExportModsMessage::CopyMarkdownToClipboard => {
                if let State::ExportMods(menu) = &self.state {
                    return iced::clipboard::write(Self::export_mods_markdown(&menu.selected_mods));
                }
            }
            ExportModsMessage::CopyPlainTextToClipboard => {
                if let State::ExportMods(menu) = &self.state {
                    return iced::clipboard::write(Self::export_mods_plain_text(
                        &menu.selected_mods,
                    ));
                }
            }
        }
        Task::none()
    }

    fn export_mods_plain_text(selected_mods: &HashSet<SelectedMod>) -> String {
        let mut lines = Vec::new();

        for selected_mod in selected_mods {
            match selected_mod {
                SelectedMod::Downloaded { name, .. } => {
                    lines.push(name.clone());
                }
                SelectedMod::Local { file_name } => {
                    // Remove file extension for cleaner display
                    let display_name = file_name
                        .strip_suffix(".jar")
                        .or_else(|| file_name.strip_suffix(".zip"))
                        .unwrap_or(file_name);
                    lines.push(display_name.to_string());
                }
            }
        }
        lines.join("\n")
    }
}

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path).await.path(path).strerr()
}

impl MenuEditMods {
    pub fn select_mod(
        &mut self,
        selected_mod: SelectedMod,
        pressed_ctrl: bool,
        pressed_shift: bool,
    ) {
        self.modal = None;

        match (pressed_ctrl, pressed_shift) {
            (true, _) => {
                self.shift_selected_mods.clear();
            }
            (_, false) => {
                let single = if let Some(m) = self.selected_mods.iter().next() {
                    selected_mod == *m && self.selected_mods.len() == 1
                } else {
                    false
                };

                if !pressed_ctrl && !single {
                    self.selected_mods.clear();
                }
            }
            _ => {}
        }

        let idx = self.index(&selected_mod);

        match (pressed_shift, self.list_shift_index) {
            // Range selection, shift pressed
            (true, Some(shift_idx)) if shift_idx != idx => {
                self.selected_mods
                    .retain(|n| !self.shift_selected_mods.contains(n));
                self.shift_selected_mods.clear();

                let (idx, shift_idx) =
                    (std::cmp::min(idx, shift_idx), std::cmp::max(idx, shift_idx));

                for i in idx..=shift_idx {
                    let current_mod: SelectedMod = self.sorted_mods_list[i].clone().into();
                    if self.selected_mods.insert(current_mod.clone()) {
                        self.shift_selected_mods.insert(current_mod);
                    }
                }
            }

            // Normal selection
            _ => {
                self.list_shift_index = Some(idx);
                if self.selected_mods.contains(&selected_mod) {
                    self.selected_mods.remove(&selected_mod);
                } else {
                    self.selected_mods.insert(selected_mod);
                }
            }
        }
    }

    fn index(&self, m: &SelectedMod) -> usize {
        if let Some(idx) = self.sorted_mods_list.iter().position(|n| m == n) {
            idx
        } else {
            debug_assert!(false, "couldn't find index of mod");
            0
        }
    }

    fn scroll_fix(&self) -> Task<Message> {
        let id = widget::scrollable::Id::new("MenuEditMods:mods");
        widget::scrollable::scroll_to(id, self.list_scroll)
    }
}
