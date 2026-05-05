use crate::{
    Launcher, Message,
    menu_renderer::back_to_launch_screen,
    state::{
        AutoSaveKind, EditPresetsMessage, InfoMessage, LaunchTab, LogState, ManageModsMessage,
        MenuEditMods, MenuInstallForge, MenuInstallOptifine, ProgressBar, SelectedState, State,
    },
    tick::sort_dependencies,
};
use iced::{Task, futures::executor::block_on, widget::scrollable::AbsoluteOffset};
use ql_core::{
    GenericProgress, Instance, IntoIoError, IntoStringError, JsonFileError, err,
    file_utils::exists,
    json::{VersionDetails, instance_config::InstanceConfigJson},
};
use ql_mod_manager::{loaders, store::ModIndex};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender},
};

pub const SIDEBAR_LIMIT_RIGHT: f32 = 140.0;
pub const SIDEBAR_LIMIT_LEFT: f32 = 135.0;

mod arrow_keys;
mod iced_event;

impl Launcher {
    pub fn on_selecting_instance(&mut self) -> Task<Message> {
        self.load_edit_instance(None);
        let Some(instance) = self.selected_instance.clone() else {
            return Task::none();
        };

        let persistent = self.config.c_persistent();
        persistent.selected_instance = Some(instance.name.clone());
        self.autosave.remove(&AutoSaveKind::LauncherConfig);

        self.load_logs();
        if let State::Launch(menu) = &mut self.state {
            menu.modal = None;
            menu.reload_notes(instance.clone())
        } else {
            Task::none()
        }
    }

    pub fn close_launcher(&mut self) -> ! {
        self.uninitialize_presence();
        std::process::exit(0);
    }

    pub fn load_logs(&mut self) {
        let State::Launch(menu) = &mut self.state else {
            return;
        };
        let Some(instance) = self.selected_instance.as_ref() else {
            menu.log_state = None;
            return;
        };
        if let (Some(logs), LaunchTab::Log) = (self.logs.get(instance), menu.tab) {
            menu.log_state = Some(LogState {
                content: iced::widget::text_editor::Content::with_text(&logs.log.join("\n")),
            });
        } else {
            menu.log_state = None;
        }
    }

    pub fn delete_instance_confirm(&mut self) -> Task<Message> {
        let State::ConfirmAction { .. } = &self.state else {
            return Task::none();
        };

        let selected_instance = self.instance();
        let is_server = selected_instance.is_server();
        let deleted_instance_dir = selected_instance.get_instance_path();

        if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
            self.set_error(err);
            return Task::none();
        }

        self.unselect_instance();
        self.go_to_main_menu(Some(InfoMessage::success(format!(
            "Deleted {}",
            if is_server { "Server" } else { "Instance" }
        ))))
    }

    pub fn unselect_instance(&mut self) {
        self.selected_instance = None;
        let p = self.config.c_persistent();
        p.selected_instance = None;
        p.selected_instance_kind = None;
        self.autosave.remove(&AutoSaveKind::LauncherConfig);
    }

    pub fn go_to_edit_mods_menu(&mut self, msg: Option<InfoMessage>) -> Task<Message> {
        async fn inner(
            this: &mut Launcher,
            info_message: Option<InfoMessage>,
        ) -> Result<Task<Message>, JsonFileError> {
            let instance = this.selected_instance.as_ref().unwrap();

            let config_json = InstanceConfigJson::read(instance).await?;
            let version_json = Box::new(VersionDetails::load(instance).await?);

            let mods = ModIndex::load(instance).await?;
            let update_local_mods_task =
                MenuEditMods::update_locally_installed_mods(&mods, instance);

            let locally_installed_mods = HashSet::new();
            let sorted_mods_list = sort_dependencies(&mods.mods, &locally_installed_mods);

            this.state = State::EditMods(MenuEditMods {
                config: config_json,
                mods,
                selected_mods: HashSet::new(),
                shift_selected_mods: HashSet::new(),
                sorted_mods_list,
                selected_state: SelectedState::None,
                available_updates: Vec::new(),
                mod_update_progress: None,
                locally_installed_mods,
                drag_and_drop_hovered: false,
                update_check_handle: None,
                version_json,
                modal: None,
                search: None,
                // If you wanna test stuff out...
                // info_message: Some(crate::state::ModInfoMessage {
                //     text: "Hello, World!".to_owned(),
                //     kind: crate::state::InfoMessageKind::AtPath(PathBuf::from("/home/mrmayman")),
                // }),
                // info_message: Some(crate::state::ModInfoMessage {
                //     text: "Hello, World!".to_owned(),
                //     kind: crate::state::InfoMessageKind::Success,
                // }),
                info_message,
                width_name: 220.0,
                list_shift_index: None,
                list_scroll: AbsoluteOffset::default(),
            });

            Ok(Task::batch([update_local_mods_task]))
        }
        match block_on(inner(self, msg)) {
            Ok(n) => n,
            Err(err) => {
                self.set_error(format!("While opening Mods screen:\n{err}"));
                Task::none()
            }
        }
    }

    pub fn install_forge(&mut self, kind: ForgeKind) -> Task<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver): (Sender<GenericProgress>, Receiver<GenericProgress>) =
            std::sync::mpsc::channel();

        let instance_selection = self.selected_instance.clone().unwrap();
        let instance_selection2 = instance_selection.clone();

        let command = Task::perform(
            async move {
                if matches!(kind, ForgeKind::NeoForge) {
                    // TODO: Add UI to specify NeoForge version
                    loaders::neoforge::install(
                        None,
                        instance_selection2,
                        Some(f_sender),
                        Some(j_sender),
                    )
                    .await
                } else {
                    loaders::forge::install(
                        None,
                        instance_selection2,
                        Some(f_sender),
                        Some(j_sender),
                    )
                    .await
                }
                .strerr()?;
                if matches!(kind, ForgeKind::OptiFine) {
                    copy_optifine_over(&instance_selection)
                        .await
                        .map_err(|n| format!("Couldn't install OptiFine with Forge:\n{n}"))?;
                    loaders::optifine::uninstall(instance_selection.get_name().to_owned(), false)
                        .await
                        .strerr()?;
                }
                Ok(())
            },
            Message::InstallForgeEnd,
        );

        self.state = State::InstallForge(MenuInstallForge {
            forge_progress: ProgressBar::with_recv(f_receiver),
            java_progress: ProgressBar::with_recv(j_receiver),
            is_java_getting_installed: false,
        });
        command
    }

    fn load_modpack_from_path(&mut self, path: PathBuf) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

        Task::perform(
            ql_mod_manager::add_files(
                self.selected_instance.clone().unwrap(),
                vec![path],
                Some(sender),
            ),
            |n| ManageModsMessage::AddFileDone(n.strerr()).into(),
        )
    }

    fn load_jar_from_path(&mut self, path: &Path, filename: &str) {
        let selected_instance = self.instance();
        let new_path = selected_instance
            .get_dot_minecraft_path()
            .join("mods")
            .join(filename);
        if *path != new_path {
            if let Err(err) = std::fs::copy(path, &new_path) {
                err!("Couldn't drag and drop mod file in: {err}");
            }
        }
    }

    fn load_qmp_from_path(&mut self, path: &Path) -> Task<Message> {
        let file = match std::fs::read(path) {
            Ok(n) => n,
            Err(err) => {
                err!("Couldn't drag and drop preset file: {err}");
                return Task::none();
            }
        };
        match tokio::runtime::Handle::current().block_on(ql_mod_manager::Preset::load(
            self.selected_instance.clone().unwrap(),
            file,
            true,
        )) {
            Ok(mods) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::EditMods(_) = &self.state {
                    self.go_to_edit_presets_menu();
                }
                if let State::ManagePresets(menu) = &mut self.state {
                    menu.progress = Some(ProgressBar::with_recv(receiver));
                }
                let instance_name = self.selected_instance.clone().unwrap();
                Task::perform(
                    ql_mod_manager::store::download_mods_bulk(
                        mods.to_install,
                        instance_name,
                        Some(sender),
                    ),
                    |n| EditPresetsMessage::LoadComplete(n.strerr()).into(),
                )
            }
            Err(err) => {
                self.set_error(err);
                Task::none()
            }
        }
    }

    fn set_drag_and_drop_hover(&mut self, is_hovered: bool) {
        if let State::EditMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::ManagePresets(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::EditJarMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::InstallOptifine(MenuInstallOptifine::Choosing {
            drag_and_drop_hovered,
            ..
        }) = &mut self.state
        {
            *drag_and_drop_hovered = is_hovered;
        }
    }
    #[cfg(feature = "auto_update")]
    pub fn update_download_start(&mut self) -> Task<Message> {
        if let State::UpdateFound(crate::state::MenuLauncherUpdate { url, progress, .. }) =
            &mut self.state
        {
            let (sender, update_receiver) = std::sync::mpsc::channel();
            *progress = Some(ProgressBar::with_recv_and_msg(
                update_receiver,
                "Starting Update".to_owned(),
            ));

            let url = url.clone();

            Task::perform(
                async move { crate::launcher_update::install(url, sender).await.strerr() },
                Message::UpdateDownloadEnd,
            )
        } else {
            Task::none()
        }
    }

    pub fn go_to_delete_instance_menu(&mut self) {
        let instance = self.instance();
        self.state = State::ConfirmAction {
            msg1: format!(
                "delete the {} {}",
                if instance.is_server() {
                    "server"
                } else {
                    "instance"
                },
                instance.get_name()
            ),
            msg2: "All your data, including worlds, will be lost".to_owned(),
            yes: Message::DeleteInstance,
            no: back_to_launch_screen(None),
        };
    }
}

pub async fn get_locally_installed_mods(
    selected_instance: PathBuf,
    blacklist: Vec<String>,
) -> HashSet<String> {
    let mods_dir_path = selected_instance.join("mods");

    let Ok(mut dir) = tokio::fs::read_dir(&mods_dir_path).await else {
        err!("Error reading mods directory");
        return HashSet::new();
    };
    let mut set = HashSet::new();
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if blacklist.contains(&file_name.to_owned()) {
            continue;
        }
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension == "jar" || extension == "disabled" {
            set.insert(file_name.to_owned());
        }
    }
    set
}

#[derive(Debug, Clone, Copy)]
pub enum ForgeKind {
    Normal,
    NeoForge,
    OptiFine,
}

async fn copy_optifine_over(instance: &Instance) -> Result<(), String> {
    let instance_dir = instance.get_instance_path();
    let installer_path = instance_dir.join("optifine/OptiFine.jar");
    let mods_dir = instance_dir.join(".minecraft/mods");

    if !exists(&installer_path).await {
        return Ok(());
    }
    if !exists(&mods_dir).await {
        tokio::fs::create_dir_all(&mods_dir)
            .await
            .path(&mods_dir)
            .strerr()?;
    }
    let new_path = mods_dir.join("optifine.jar");
    tokio::fs::copy(&installer_path, &new_path).await.strerr()?;

    let mut config = InstanceConfigJson::read(instance).await.strerr()?;
    config.mod_type_info.get_or_insert_default().optifine_jar = Some("optifine.jar".to_owned());
    config.save(instance).await.strerr()?;

    Ok(())
}
