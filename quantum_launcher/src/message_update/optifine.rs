use std::path::Path;

use iced::{Task, futures::executor::block_on};
use ql_core::{IntoStringError, JsonFileError, Loader, OptifineUniqueVersion};
use ql_mod_manager::loaders;

use crate::state::{
    InfoMessage, InstallOptifineMessage, Launcher, MenuInstallOptifine, Message, ProgressBar, State,
};

impl Launcher {
    pub fn update_install_optifine(&mut self, message: InstallOptifineMessage) -> Task<Message> {
        match message {
            InstallOptifineMessage::ScreenOpen => {
                let (optifine_unique_version, version) = match self.optifine_get_version() {
                    Ok(n) => n,
                    Err(err) => {
                        self.set_error(err);
                        return Task::none();
                    }
                };

                if let Some(version @ OptifineUniqueVersion::B1_7_3) = optifine_unique_version {
                    self.state = State::InstallOptifine(MenuInstallOptifine::InstallingB173);

                    let selected_instance = self.selected_instance.clone().unwrap();
                    let url = version.get_url().0;
                    return Task::perform(
                        loaders::optifine::install_b173(selected_instance, url),
                        |n| InstallOptifineMessage::End(n.strerr()).into(),
                    );
                }

                self.state = State::InstallOptifine(MenuInstallOptifine::Choosing {
                    optifine_unique_version,
                    version,
                    delete_installer: true,
                    drag_and_drop_hovered: false,
                });
            }
            InstallOptifineMessage::DeleteInstallerToggle(t) => {
                if let State::InstallOptifine(MenuInstallOptifine::Choosing {
                    delete_installer,
                    ..
                }) = &mut self.state
                {
                    *delete_installer = t;
                }
            }
            InstallOptifineMessage::SelectInstallerStart => {
                let f = rfd::AsyncFileDialog::new()
                    .add_filter("jar/zip", &["jar", "zip"])
                    .set_title("Select OptiFine Installer")
                    .pick_file();
                return Task::perform(f, |p| {
                    if let Some(p) = p {
                        InstallOptifineMessage::SelectInstaller(p.path().to_owned()).into()
                    } else {
                        Message::Nothing
                    }
                });
            }
            InstallOptifineMessage::SelectInstaller(path) => {
                return self.install_optifine_confirm(&path);
            }
            InstallOptifineMessage::End(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self
                        .go_to_edit_mods_menu(Some(InfoMessage::success("Installed Optifine")));
                }
            }
        }
        Task::none()
    }

    fn optifine_get_version(
        &mut self,
    ) -> Result<(Option<OptifineUniqueVersion>, String), JsonFileError> {
        if let State::InstallOptifine(MenuInstallOptifine::Choosing {
            optifine_unique_version,
            version,
            ..
        }) = &self.state
        {
            return Ok((*optifine_unique_version, version.clone()));
        } else if let State::EditMods(menu) = &self.state {
            if menu.config.mod_type == Loader::Forge {
                return Ok((
                    Some(OptifineUniqueVersion::Forge),
                    menu.version_json.get_id().to_owned(),
                ));
            }
        };
        block_on(OptifineUniqueVersion::get(self.instance()))
    }

    pub fn install_optifine_confirm(&mut self, installer_path: &Path) -> Task<Message> {
        let (p_sender, p_recv) = std::sync::mpsc::channel();
        let (j_sender, j_recv) = std::sync::mpsc::channel();

        let instance = self.instance().clone();
        debug_assert!(!instance.is_server());

        let optifine_unique_version = match self.optifine_get_version() {
            Ok(n) => n.0,
            Err(err) => {
                self.set_error(err);
                return Task::none();
            }
        };

        let delete_installer = if let State::InstallOptifine(MenuInstallOptifine::Choosing {
            delete_installer,
            ..
        }) = &self.state
        {
            *delete_installer
        } else {
            false
        };

        self.state = State::InstallOptifine(MenuInstallOptifine::Installing {
            optifine_install_progress: ProgressBar::with_recv(p_recv),
            java_install_progress: Some(ProgressBar::with_recv(j_recv)),
            is_java_being_installed: false,
        });

        let installer_path = installer_path.to_owned();
        Task::perform(
            // OptiFine does not support servers
            // so it's safe to assume we've selected an instance.
            loaders::optifine::install(
                instance,
                installer_path.clone(),
                Some(p_sender),
                Some(j_sender),
                optifine_unique_version,
            ),
            |n| InstallOptifineMessage::End(n.strerr()).into(),
        )
        .chain(Task::perform(
            async move {
                if delete_installer
                    && installer_path.extension().is_some_and(|n| {
                        let n = n.to_ascii_lowercase();
                        n == "jar" || n == "zip"
                    })
                {
                    _ = tokio::fs::remove_file(installer_path).await;
                }
            },
            |()| Message::Nothing,
        ))
    }
}
