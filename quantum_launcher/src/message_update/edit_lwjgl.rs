use iced::Task;

use crate::state::{ApplyLwjglResult, EditLwjglMessage, Launcher, MenuEditLwjgl, Message, State};

impl Launcher {
    pub fn update_edit_lwjgl(&mut self, message: EditLwjglMessage) -> Task<Message> {
        match message {
            EditLwjglMessage::VersionsLoaded(result) => {
                match result {
                    Ok(versions) => {
                        if let State::EditLwjgl(MenuEditLwjgl::Loading {
                            initial_version, ..
                        }) = &self.state
                        {
                            let initial_version = initial_version.clone();
                            let selected_version = initial_version.clone().unwrap_or_else(|| {
                                versions
                                    .lwjgl3
                                    .first()
                                    .or_else(|| versions.lwjgl2.first())
                                    .cloned()
                                    .unwrap_or_else(|| "default".to_string())
                            });

                            // Cache the versions for future use
                            self.lwjgl_versions_cache = Some(versions.clone());

                            self.state = State::EditLwjgl(MenuEditLwjgl::Loaded {
                                versions,
                                selected_version,
                                initial_version,
                                is_applying: false,
                                mismatch_confirm: None,
                            });
                        }
                    }
                    Err(err) => {
                        self.state = State::Error {
                            error: format!("Failed to load LWJGL versions: {err}"),
                        };
                    }
                }
                Task::none()
            }
            EditLwjglMessage::VersionSelected(version) => {
                if let State::EditLwjgl(MenuEditLwjgl::Loaded {
                    selected_version,
                    mismatch_confirm,
                    ..
                }) = &mut self.state
                {
                    if let Some(v) = version {
                        *selected_version = v;
                        *mismatch_confirm = None;
                    }
                }
                Task::none()
            }
            EditLwjglMessage::Apply => {
                let instance = self.instance().clone();

                if let State::EditLwjgl(MenuEditLwjgl::Loaded {
                    selected_version,
                    is_applying,
                    mismatch_confirm,
                    ..
                }) = &mut self.state
                {
                    *mismatch_confirm = None;
                    *is_applying = true;

                    let selected_version = selected_version.clone();
                    return Task::perform(
                        async move {
                            // "default" means no override
                            if selected_version == "default" {
                                let mut config =
                                    ql_core::json::InstanceConfigJson::read(&instance).await?;
                                config.lwjgl_version = None;
                                config.save(&instance).await?;
                                return Ok(ApplyLwjglResult::Saved);
                            }

                            // Detect which LWJGL major this instance uses by inspecting its libraries.
                            let details = ql_core::json::VersionDetails::load(&instance).await?;
                            let mut instance_is_lwjgl3: Option<bool> = None;
                            for lib in &details.libraries {
                                let Some(name) = &lib.name else {
                                    continue;
                                };
                                // groupId is before first ':'
                                if let Some((group, _rest)) = name.split_once(':') {
                                    if group == "org.lwjgl" {
                                        instance_is_lwjgl3 = Some(true);
                                        break;
                                    }
                                    if group == "org.lwjgl.lwjgl" {
                                        instance_is_lwjgl3 = Some(false);
                                        break;
                                    }
                                }
                            }

                            let override_is_lwjgl3 =
                                ql_core::json::lwjgl::is_lwjgl3(&selected_version);

                            if let Some(instance_is_lwjgl3) = instance_is_lwjgl3 {
                                if instance_is_lwjgl3 != override_is_lwjgl3 {
                                    let msg = format!(
                                        "This instance uses {} LWJGL libraries, but you selected LWJGL {}.\n\nMix-matching major versions often crashes and may cause 404 downloads.\n\nContinue?",
                                        if instance_is_lwjgl3 { "LWJGL 3.x" } else { "LWJGL 2.x" },
                                        selected_version
                                    );
                                    return Ok(ApplyLwjglResult::NeedsConfirmation(msg));
                                }
                            }

                            let mut config =
                                ql_core::json::InstanceConfigJson::read(&instance).await?;
                            config.lwjgl_version = Some(selected_version);
                            config.save(&instance).await?;
                            Ok(ApplyLwjglResult::Saved)
                        },
                        |result: Result<ApplyLwjglResult, ql_core::JsonFileError>| {
                            let result = result.map_err(|e| e.to_string());
                            Message::EditLwjgl(EditLwjglMessage::ApplyChecked(result))
                        },
                    );
                }
                Task::none()
            }
            EditLwjglMessage::ApplyChecked(result) => {
                if let State::EditLwjgl(MenuEditLwjgl::Loaded {
                    is_applying,
                    mismatch_confirm,
                    ..
                }) = &mut self.state
                {
                    *is_applying = false;
                    match result {
                        Ok(ApplyLwjglResult::Saved) => {
                            return Task::done(Message::EditLwjgl(EditLwjglMessage::Back))
                        }
                        Ok(ApplyLwjglResult::NeedsConfirmation(msg)) => {
                            *mismatch_confirm = Some(msg);
                        }
                        Err(err) => {
                            return Task::done(Message::Error(format!(
                                "Failed to apply LWJGL version: {err}"
                            )))
                        }
                    }
                }
                Task::none()
            }
            EditLwjglMessage::MismatchProceed => {
                let instance = self.instance().clone();
                if let State::EditLwjgl(MenuEditLwjgl::Loaded {
                    selected_version,
                    is_applying,
                    mismatch_confirm,
                    ..
                }) = &mut self.state
                {
                    *mismatch_confirm = None;
                    *is_applying = true;
                    let selected_version = selected_version.clone();
                    return Task::perform(
                        async move {
                            let mut config =
                                ql_core::json::InstanceConfigJson::read(&instance).await?;
                            config.lwjgl_version =
                                Some(selected_version).filter(|v| v != "default");
                            config.save(&instance).await?;
                            Ok(())
                        },
                        |result: Result<(), ql_core::JsonFileError>| match result {
                            Ok(()) => Message::EditLwjgl(EditLwjglMessage::Back),
                            Err(err) => {
                                Message::Error(format!("Failed to save LWJGL version: {err}"))
                            }
                        },
                    );
                }
                Task::none()
            }
            EditLwjglMessage::MismatchRevert => {
                if let State::EditLwjgl(MenuEditLwjgl::Loaded {
                    selected_version,
                    initial_version,
                    mismatch_confirm,
                    is_applying,
                    ..
                }) = &mut self.state
                {
                    *is_applying = false;
                    *mismatch_confirm = None;
                    *selected_version = initial_version
                        .clone()
                        .unwrap_or_else(|| "default".to_owned());
                }
                Task::none()
            }
            EditLwjglMessage::Back => {
                self.state = State::Launch(Default::default());
                Task::none()
            }
        }
    }
}
