use iced::{Task, widget};
use iced::{futures::executor::block_on, keyboard::Modifiers};
use ql_core::json::VersionDetails;
use ql_core::{Instance, IntoIoError, IntoStringError, err, jarmod::JarMods};
use ql_mod_manager::store::{DirStructure, LocalMod, ModId, QueryType, SelectedMod};
use std::{collections::HashSet, path::PathBuf};

use crate::state::{
    AutoSaveKind, ExportModsTextMessage, InfoMessage, InfoMessageKind, Launcher,
    ManageJarModsMessage, ManageModsMessage, MenuCurseforgeManualDownload, MenuEditJarMods,
    MenuEditMods, MenuEditModsModal, MenuExportModsText, Message, ProgressBar, SelectedState,
    State,
};

impl Launcher {
    pub fn update_manage_mods(&mut self, msg: ManageModsMessage) -> Task<Message> {
        match msg {
            ManageModsMessage::Open => return self.go_to_edit_mods_menu(None),

            ManageModsMessage::AddFileDone(Err(err))
            | ManageModsMessage::DeleteFinished(Err(err))
            | ManageModsMessage::IndexLoaded(Err(err))
            | ManageModsMessage::UpdatePerformDone(Err(err)) => self.set_error(err),

            ManageModsMessage::IndexLoaded(Ok(idx)) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.file_data.mod_index = idx;
                }
            }
            ManageModsMessage::ListScrolled(offset) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.ui_state.list_scroll = offset;
                }
            }
            ManageModsMessage::SelectEnsure(name, id, project_type) => {
                let State::EditMods(menu) = &mut self.state else {
                    return Task::none();
                };
                let selected_mod = SelectedMod::new(name, id, project_type);
                menu.selection.list_shift_index = Some(menu.index(&selected_mod));
                menu.selection.shift_selected_mods.clear();
                menu.selection.selected_mods.clear();
                menu.selection.selected_mods.insert(selected_mod);
                menu.update_selected_state();
                return menu.scroll_fix();
            }
            ManageModsMessage::SelectMod(name, id, project_type) => {
                let State::EditMods(menu) = &mut self.state else {
                    return Task::none();
                };

                let selected_mod = SelectedMod::new(name, id, project_type);

                let pressed_ctrl = self.modifiers_pressed.contains(Modifiers::COMMAND);
                let pressed_shift = self.modifiers_pressed.contains(Modifiers::SHIFT);

                menu.select_mod(selected_mod, pressed_ctrl, pressed_shift);
                menu.update_selected_state();
                return menu.scroll_fix();
            }
            ManageModsMessage::AddFile(delete_file, project_type) => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter(project_type.to_string(), project_type.get_extensions())
                        .set_title("Add Mod, Modpack or Preset")
                        .pick_files(),
                    move |r| {
                        ManageModsMessage::AddFileSelected(
                            delete_file,
                            r.into_iter()
                                .flatten()
                                .map(|n| n.path().to_owned())
                                .collect(),
                            project_type,
                        )
                        .into()
                    },
                );
            }
            ManageModsMessage::AddFileSelected(delete_files, paths, project_type) => {
                return self.add_file_selected(delete_files, paths, project_type);
            }
            ManageModsMessage::AddFileDone(Ok(not_allowed)) => {
                if !not_allowed.is_empty() {
                    self.state = State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                        not_allowed,
                        delete_mods: true,
                    });
                }
                return self.go_to_edit_mods_menu(None);
            }
            ManageModsMessage::DeleteSelected => {
                if let State::EditMods(menu) = &mut self.state {
                    let instance = self.selected_instance.clone().unwrap();
                    let delete_downloaded_command =
                        Self::get_delete_mods_command(instance.clone(), menu);

                    let local_mods: Vec<LocalMod> = menu
                        .selection
                        .selected_mods
                        .iter()
                        .filter_map(|m| m.local().cloned())
                        .collect();

                    for l in &local_mods {
                        menu.locally_installed_mods.remove(l);
                    }

                    let delete_local_command = Task::perform(
                        async move {
                            let json = VersionDetails::load(&instance).await.strerr()?;
                            let dirs = DirStructure::new(instance, &json).await.strerr()?;
                            for l in local_mods {
                                let Some(dir) = dirs.get(l.1) else { continue };
                                let path = dir.join(&*l.0);
                                delete_file_wrapper(path).await?;
                            }
                            Ok(())
                        },
                        Message::Done,
                    );

                    return Task::batch([delete_downloaded_command, delete_local_command]);
                }
            }
            ManageModsMessage::DeleteOptiforge(name) => {
                let mods_dir = self.instance().get_dot_minecraft_path().join("mods");
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods
                        .remove(&LocalMod(name.clone(), QueryType::Mods));
                    if let Some(mod_info) = &mut menu.file_data.config.mod_type_info {
                        if mod_info.optifine_jar.as_ref().is_some_and(|n| n == &name) {
                            mod_info.optifine_jar = None;
                            if let Err(err) = block_on(
                                menu.file_data
                                    .config
                                    .save(self.selected_instance.as_ref().unwrap()),
                            ) {
                                self.set_error(err);
                            }
                        }
                    }
                }
                return Task::perform(delete_file_wrapper(mods_dir.join(&*name)), Message::Done);
            }
            ManageModsMessage::DeleteFinished(Ok(_)) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.selection.selected_mods.clear();
                }
            }
            ManageModsMessage::LocalFilesLoaded(files, query_type) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods
                        .retain(|n| n.1 != query_type || files.contains(n));
                    menu.locally_installed_mods.extend(files);
                }
            }
            ManageModsMessage::ToggleSelected => return self.manage_mods_toggle_selected(),

            ManageModsMessage::UpdatePerform => return self.apply_mod_updates(),
            ManageModsMessage::UpdatePerformDone(Ok((file, should_write_changelog))) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.updates.available.clear();
                    menu.ui_state.info_message = if let Some(file) = file {
                        Some(InfoMessage {
                            text: format!("{} written to disk", file.filename),
                            kind: InfoMessageKind::AtPath(file.path),
                        })
                    } else {
                        should_write_changelog
                            .then(|| InfoMessage::error("Changelog was not written to disk"))
                    };
                }
            }

            ManageModsMessage::UpdateCheck => {
                let (task, handle) = Task::perform(
                    ql_mod_manager::store::check_for_updates(
                        self.selected_instance.clone().unwrap(),
                    ),
                    |n| ManageModsMessage::UpdateCheckResult(n.strerr()).into(),
                )
                .abortable();
                if let State::EditMods(menu) = &mut self.state {
                    menu.updates.check_handle = Some(handle.abort_on_drop());
                    menu.ui_state.modal = None;
                }
                return task;
            }
            ManageModsMessage::UpdateCheckResult(updates) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.updates.check_handle = None;
                    match updates {
                        Ok(updates) => {
                            if updates.is_empty() {
                                menu.ui_state.info_message = Some(InfoMessage {
                                    text: "No updates found".to_owned(),
                                    kind: InfoMessageKind::Success,
                                });
                            }

                            menu.updates.available = updates
                                .into_iter()
                                .map(|(id, title)| {
                                    let enabled = menu
                                        .file_data
                                        .mod_index
                                        .mods
                                        .get(&id)
                                        .is_none_or(|n| n.enabled);
                                    (id, title, enabled)
                                })
                                .collect();
                        }
                        Err(err) => {
                            err!(no_log, "Could not check for updates: {err}");
                        }
                    }
                }
            }
            ManageModsMessage::UpdateCheckToggle(idx, t) => {
                if let State::EditMods(menu) = &mut self.state {
                    if let Some((_, _, b)) = menu.updates.available.get_mut(idx) {
                        *b = t;
                    }
                }
            }
            ManageModsMessage::SetInfoMessage(message) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.ui_state.info_message = message;
                }
            }
            ManageModsMessage::SelectAll => {
                if let State::EditMods(menu) = &mut self.state {
                    match menu.selection.state {
                        SelectedState::All => {
                            menu.selection.selected_mods.clear();
                            menu.selection.state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            menu.selection.selected_mods = menu
                                .file_data
                                .mod_index
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info
                                        .manually_installed
                                        .then_some(SelectedMod::Downloaded {
                                            name: mod_info.name.clone(),
                                            id: id.clone(),
                                        })
                                })
                                .chain(
                                    menu.locally_installed_mods
                                        .iter()
                                        .map(|n| SelectedMod::Local(n.clone())),
                                )
                                .collect();
                            menu.selection.state = SelectedState::All;
                        }
                    }
                }
            }
            ManageModsMessage::SetModal(modal) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.ui_state.modal = modal;
                    return menu.scroll_fix();
                }
            }
            ManageModsMessage::SetSearch(search) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.ui_state.modal = None;
                    menu.search = search;
                    return menu.scroll_fix();
                }
            }
            ManageModsMessage::SetContentFilter(filter) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.content_filter = filter;
                    // Clear selection when changing filter
                    menu.selection.selected_mods.clear();
                    menu.selection.shift_selected_mods.clear();
                    menu.update_selected_state();
                }
            }
            ManageModsMessage::CurseforgeManualToggleDelete(t) => {
                if let State::CurseforgeManualDownload(menu) = &mut self.state {
                    menu.delete_mods = t;
                }
            }
            ManageModsMessage::RightClick(clicked_id) => {
                if let State::EditMods(menu) = &mut self.state {
                    if let Some(MenuEditModsModal::RightClick(old_id, _)) = &menu.ui_state.modal {
                        if *old_id == clicked_id {
                            menu.ui_state.modal = None;
                        } else {
                            menu.ui_state.modal = Some(MenuEditModsModal::RightClick(
                                clicked_id,
                                self.window_state.mouse_pos,
                            ));
                        }
                    } else {
                        menu.ui_state.modal = Some(MenuEditModsModal::RightClick(
                            clicked_id,
                            self.window_state.mouse_pos,
                        ));
                    }
                    return menu.scroll_fix();
                }
            }
            ManageModsMessage::ToggleOne(id) => {
                if let State::EditMods(menu) = &mut self.state {
                    if let Some(m) = menu.file_data.mod_index.mods.get_mut(&id) {
                        if m.project_type.is_toggleable() {
                            m.enabled = !m.enabled;
                        }
                    }
                }
                return Task::perform(
                    ql_mod_manager::store::toggle_mods(
                        vec![id],
                        self.selected_instance.clone().unwrap(),
                    ),
                    |n| Message::Done(n.strerr()),
                );
            }
            ManageModsMessage::ToggleOneLocal(local) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.toggle_local_mods_in_ui(std::slice::from_ref(&local));
                }
                return Task::perform(
                    ql_mod_manager::store::toggle_mods_local(
                        vec![local],
                        self.selected_instance.clone().unwrap(),
                    ),
                    |n| Message::Done(n.strerr()),
                );
            }
        }
        Task::none()
    }

    fn apply_mod_updates(&mut self) -> Task<Message> {
        if let State::EditMods(menu) = &mut self.state {
            let updates = menu
                .updates
                .available
                .clone()
                .into_iter()
                .map(|(id, version, _)| (id, version))
                .collect();
            let write_changelog = self.config.c_persistent().write_mod_update_changelog;
            let (sender, receiver) = std::sync::mpsc::channel();
            menu.updates.progress = Some(ProgressBar::with_recv_and_msg(
                receiver,
                "Deleting Mods".to_owned(),
            ));
            let selected_instance = self.selected_instance.clone().unwrap();
            Task::perform(
                ql_mod_manager::store::apply_updates(
                    selected_instance,
                    updates,
                    Some(sender),
                    write_changelog,
                ),
                move |n| {
                    ManageModsMessage::UpdatePerformDone(
                        n.strerr().map(|res| (res, write_changelog)),
                    )
                    .into()
                },
            )
        } else {
            Task::none()
        }
    }

    fn manage_mods_toggle_selected(&mut self) -> Task<Message> {
        let State::EditMods(menu) = &mut self.state else {
            return Task::none();
        };
        let (ids_downloaded, ids_local) = menu.get_kinds_of_ids();
        let instance_name = self.selected_instance.clone().unwrap();

        // Show change in UI beforehand, don't want for disk sync
        for m in &ids_downloaded {
            if let Some(m) = menu.file_data.mod_index.mods.get_mut(m) {
                if !m.project_type.is_toggleable() {
                    continue;
                }
                m.enabled = !m.enabled;
            }
        }

        // menu.selected_mods.clear();
        // menu.selected_state = SelectedState::None;

        menu.toggle_local_mods_in_ui(&ids_local);

        let toggle_downloaded = Task::perform(
            ql_mod_manager::store::toggle_mods(ids_downloaded.clone(), instance_name.clone()),
            |n| Message::Done(n.strerr()),
        );
        let toggle_local = Task::perform(
            ql_mod_manager::store::toggle_mods_local(ids_local, instance_name.clone()),
            |n| Message::Done(n.strerr()),
        );

        Task::batch([toggle_downloaded, toggle_local])
    }

    fn add_file_selected(
        &mut self,
        delete_file: bool,
        paths: Vec<PathBuf>,
        project_type: QueryType,
    ) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();
        if project_type == QueryType::ModPacks {
            self.state = State::ImportModpack(ProgressBar::with_recv(receiver));
        }

        let files_task = Task::perform(
            ql_mod_manager::add_files(
                self.selected_instance.clone().unwrap(),
                paths.clone(),
                Some(sender),
                project_type,
            ),
            move |n| ManageModsMessage::AddFileDone(n.strerr()).into(),
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

    fn get_delete_mods_command(selected_instance: Instance, menu: &MenuEditMods) -> Task<Message> {
        let ids: Vec<ModId> = menu
            .selection
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    if let Some(config) = menu.file_data.mod_index.mods.get(id) {
                        if config.manually_installed {
                            return Some(id.clone());
                        }
                    }
                }
                None
            })
            .collect();

        Task::perform(
            ql_mod_manager::store::delete_mods(ids, selected_instance),
            |n| ManageModsMessage::DeleteFinished(n.strerr()).into(),
        )
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
                        ManageJarModsMessage::MoveUp if i > 0 => {
                            let removed = menu.jarmods.mods.remove(i);
                            menu.jarmods.mods.insert(i - 1, removed);
                        }
                        ManageJarModsMessage::MoveDown if i + 1 < menu.jarmods.mods.len() => {
                            let removed = menu.jarmods.mods.remove(i);
                            menu.jarmods.mods.insert(i + 1, removed);
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
            for selected in &menu.selected_mods {
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

            for selected in &menu.selected_mods {
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
                SelectedMod::Local(l) => {
                    let display_name =
                        l.0.strip_suffix(".jar")
                            .or_else(|| l.0.strip_suffix(".zip"))
                            .unwrap_or(&*l.0);
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

    pub fn update_export_mods(&mut self, msg: ExportModsTextMessage) -> Task<Message> {
        match msg {
            ExportModsTextMessage::Open => {
                let State::EditMods(menu) = &mut self.state else {
                    return Task::none();
                };
                self.state = State::ExportModsText(MenuExportModsText {
                    selected_mods: if menu.selection.selected_mods.is_empty() {
                        menu.file_data
                            .mod_index
                            .mods
                            .iter()
                            .filter_map(|(id, mod_info)| {
                                mod_info
                                    .manually_installed
                                    .then_some(SelectedMod::Downloaded {
                                        name: mod_info.name.clone(),
                                        id: id.clone(),
                                    })
                            })
                            .chain(
                                menu.locally_installed_mods
                                    .iter()
                                    .map(|n| SelectedMod::Local(n.clone())),
                            )
                            .collect()
                    } else {
                        menu.selection.selected_mods.clone()
                    },
                });
            }
            ExportModsTextMessage::ExportAsPlainText => {
                if let State::ExportModsText(menu) = &self.state {
                    return Self::export_to_file(Self::export_mods_plain_text(&menu.selected_mods));
                }
            }
            ExportModsTextMessage::ExportAsMarkdown => {
                if let State::ExportModsText(menu) = &self.state {
                    return Self::export_to_file(Self::export_mods_markdown(&menu.selected_mods));
                }
            }
            ExportModsTextMessage::CopyMarkdownToClipboard => {
                if let State::ExportModsText(menu) = &self.state {
                    return iced::clipboard::write(Self::export_mods_markdown(&menu.selected_mods));
                }
            }
            ExportModsTextMessage::CopyPlainTextToClipboard => {
                if let State::ExportModsText(menu) = &self.state {
                    return iced::clipboard::write(Self::export_mods_plain_text(
                        &menu.selected_mods,
                    ));
                }
            }
        }
        Task::none()
    }

    fn export_mods_plain_text(selected_mods: &HashSet<SelectedMod>) -> String {
        let mut lines = String::new();

        for selected_mod in selected_mods {
            match selected_mod {
                SelectedMod::Downloaded { name, .. } => {
                    lines.push_str(name);
                }
                SelectedMod::Local(l) => {
                    if l.1 != QueryType::Mods {
                        continue;
                    }

                    // Remove file extension for cleaner display
                    let display_name =
                        l.0.strip_suffix(".jar")
                            .or_else(|| l.0.strip_suffix(".zip"))
                            .unwrap_or(&*l.0);
                    lines.push_str(display_name);
                }
            }
            lines.push('\n');
        }
        lines
    }
}

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    match tokio::fs::metadata(&path).await {
        Ok(n) if n.is_dir() => tokio::fs::remove_dir_all(&path).await,
        Ok(_) => tokio::fs::remove_file(&path).await,

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(err) => return Err(err.to_string()),
    }
    .path(path)
    .strerr()
}

impl MenuEditMods {
    fn select_mod(&mut self, selected_mod: SelectedMod, pressed_ctrl: bool, pressed_shift: bool) {
        self.ui_state.modal = None;

        match (pressed_ctrl, pressed_shift) {
            (true, _) => {
                self.selection.shift_selected_mods.clear();
            }
            (_, false) => {
                let single = if let Some(m) = self.selection.selected_mods.iter().next() {
                    selected_mod == *m && self.selection.selected_mods.len() == 1
                } else {
                    false
                };

                if !pressed_ctrl && !single {
                    self.selection.selected_mods.clear();
                }
            }
            _ => {}
        }

        let idx = self.index(&selected_mod);

        match (pressed_shift, self.selection.list_shift_index) {
            // Range selection, shift pressed
            (true, Some(shift_idx)) if shift_idx != idx => {
                self.selection
                    .selected_mods
                    .retain(|n| !self.selection.shift_selected_mods.contains(n));
                self.selection.shift_selected_mods.clear();

                let (idx, shift_idx) =
                    (std::cmp::min(idx, shift_idx), std::cmp::max(idx, shift_idx));

                for i in idx..=shift_idx {
                    let current_mod: SelectedMod = self.sorted_mods_list[i].clone().into();
                    if self.selection.selected_mods.insert(current_mod.clone()) {
                        self.selection.shift_selected_mods.insert(current_mod);
                    }
                }
            }

            // Normal selection
            _ => {
                self.selection.list_shift_index = Some(idx);
                if self.selection.selected_mods.contains(&selected_mod) {
                    self.selection.selected_mods.remove(&selected_mod);
                } else {
                    self.selection.selected_mods.insert(selected_mod);
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

    /// Workaround for annoying iced tree diffing bug
    /// that resets scroll position.
    pub fn scroll_fix(&self) -> Task<Message> {
        let id = widget::scrollable::Id::new("MenuEditMods:mods");
        widget::scrollable::scroll_to(id, self.ui_state.list_scroll)
    }
}

impl ManageModsMessage {
    pub fn edits_mod_list(&self) -> bool {
        match self {
            ManageModsMessage::Open
            | ManageModsMessage::IndexLoaded(_)
            | ManageModsMessage::DeleteOptiforge(_)
            | ManageModsMessage::DeleteSelected
            | ManageModsMessage::DeleteFinished(_)
            | ManageModsMessage::LocalFilesLoaded(_, _)
            | ManageModsMessage::UpdatePerformDone(_)
            | ManageModsMessage::AddFileDone(_)
            | ManageModsMessage::ToggleSelected
            | ManageModsMessage::ToggleOne(_)
            | ManageModsMessage::ToggleOneLocal(_) => true,

            ManageModsMessage::SelectEnsure(_, _, _)
            | ManageModsMessage::SelectMod(_, _, _)
            | ManageModsMessage::SelectAll
            | ManageModsMessage::ListScrolled(_)
            | ManageModsMessage::UpdateCheck
            | ManageModsMessage::UpdateCheckResult(_)
            | ManageModsMessage::UpdateCheckToggle(_, _)
            | ManageModsMessage::UpdatePerform
            | ManageModsMessage::AddFile(_, _)
            | ManageModsMessage::SetSearch(_)
            | ManageModsMessage::SetContentFilter(_)
            | ManageModsMessage::RightClick(_)
            | ManageModsMessage::SetModal(_)
            | ManageModsMessage::AddFileSelected(_, _, _)
            | ManageModsMessage::CurseforgeManualToggleDelete(_)
            | ManageModsMessage::SetInfoMessage(_) => false,
        }
    }
}
