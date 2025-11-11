use iced::Task;
use ql_core::{
    err, json::instance_config::CustomJarConfig, IntoIoError, IntoStringError, LAUNCHER_DIR,
};

use crate::{
    message_handler::format_memory,
    state::{
        CustomJarState, EditInstanceMessage, Launcher, MenuLaunch, Message, PathWatcher, State,
        ADD_JAR_NAME, NONE_JAR_NAME, OPEN_FOLDER_JAR_NAME, REMOVE_JAR_NAME,
    },
};

use super::add_to_arguments_list;

impl Launcher {
    pub fn update_edit_instance(
        &mut self,
        message: EditInstanceMessage,
    ) -> Result<Task<Message>, String> {
        match message {
            EditInstanceMessage::JavaOverride(n) => {
                self.i_config_mut().java_override = Some(n);
            }
            EditInstanceMessage::MemoryChanged(new_slider_value) => {
                let memory_mb = self.i_config_mut().ram_in_mb;
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.slider_value = new_slider_value;
                    menu.slider_text = format_memory(memory_mb);
                }
                self.i_config_mut().ram_in_mb = 2f32.powf(new_slider_value) as usize;
            }
            EditInstanceMessage::LoggingToggle(t) => {
                self.i_config_mut().enable_logger = Some(t);
            }
            EditInstanceMessage::CloseLauncherToggle(t) => {
                self.i_config_mut().close_on_start = Some(t);
            }
            EditInstanceMessage::JavaArgsAdd => {
                self.i_config_mut()
                    .java_args
                    .get_or_insert(Vec::new())
                    .push(String::new());
            }
            EditInstanceMessage::JavaArgEdit(msg, idx) => {
                self.e_java_arg_edit(msg, idx);
            }
            EditInstanceMessage::JavaArgDelete(idx) => {
                self.e_java_arg_delete(idx);
            }
            EditInstanceMessage::JavaArgsModeChanged(mode) => {
                self.i_config_mut().java_args_mode = Some(mode)
            }

            EditInstanceMessage::GameArgsAdd => {
                self.i_config_mut()
                    .game_args
                    .get_or_insert(Vec::new())
                    .push(String::new());
            }
            EditInstanceMessage::GameArgEdit(msg, idx) => {
                self.e_game_arg_edit(msg, idx);
            }
            EditInstanceMessage::GameArgDelete(idx) => {
                self.e_game_arg_delete(idx);
            }
            EditInstanceMessage::JavaArgShiftUp(idx) => {
                if let Some(args) = &mut self.i_config_mut().java_args {
                    Self::e_list_shift_up(idx, args);
                }
            }
            EditInstanceMessage::JavaArgShiftDown(idx) => {
                if let Some(args) = &mut self.i_config_mut().java_args {
                    Self::e_list_shift_down(idx, args);
                }
            }
            EditInstanceMessage::GameArgShiftUp(idx) => {
                if let Some(args) = &mut self.i_config_mut().game_args {
                    Self::e_list_shift_up(idx, args);
                }
            }
            EditInstanceMessage::GameArgShiftDown(idx) => {
                if let Some(args) = &mut self.i_config_mut().game_args {
                    Self::e_list_shift_down(idx, args);
                }
            }
            EditInstanceMessage::PreLaunchPrefixAdd => {
                self.i_config_mut().get_launch_prefix().push(String::new());
            }
            EditInstanceMessage::PreLaunchPrefixEdit(msg, idx) => {
                add_to_arguments_list(msg, self.i_config_mut().get_launch_prefix(), idx);
            }
            EditInstanceMessage::PreLaunchPrefixDelete(idx) => {
                self.i_config_mut().get_launch_prefix().remove(idx);
            }
            EditInstanceMessage::PreLaunchPrefixShiftUp(idx) => {
                Self::e_list_shift_up(idx, self.i_config_mut().get_launch_prefix());
            }
            EditInstanceMessage::PreLaunchPrefixShiftDown(idx) => {
                Self::e_list_shift_down(idx, self.i_config_mut().get_launch_prefix());
            }
            EditInstanceMessage::PreLaunchPrefixModeChanged(mode) => {
                self.i_config_mut().pre_launch_prefix_mode = Some(mode);
            }
            EditInstanceMessage::RenameEdit(n) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.instance_name = n;
                }
            }
            EditInstanceMessage::RenameApply => return self.rename_instance(),
            EditInstanceMessage::ConfigSaved(res) => res?,
            EditInstanceMessage::WindowWidthChanged(width) => {
                self.i_config_mut()
                    .global_settings
                    .get_or_insert_with(Default::default)
                    .window_width = if width.is_empty() {
                    None
                } else {
                    // TODO: Error handling
                    width.parse::<u32>().ok()
                }
            }
            EditInstanceMessage::WindowHeightChanged(height) => {
                self.i_config_mut()
                    .global_settings
                    .get_or_insert_with(Default::default)
                    .window_height = if height.is_empty() {
                    None
                } else {
                    // TODO: Error handling
                    height.parse::<u32>().ok()
                }
            }
            EditInstanceMessage::CustomJarPathChanged(path) => {
                if path == ADD_JAR_NAME {
                    return Ok(self.add_custom_jar());
                } else if path == REMOVE_JAR_NAME {
                    let jar_config = self.i_config().custom_jar.clone();
                    if let (Some(jar), Some(list)) = (jar_config, &mut self.custom_jar) {
                        list.choices.retain(|n| *n != jar.name);
                        let name = jar.name.clone();
                        self.i_config_mut().custom_jar = None;
                        return Ok(Task::perform(
                            tokio::fs::remove_file(LAUNCHER_DIR.join("custom_jars").join(name)),
                            |_| Message::Nothing,
                        ));
                    }
                } else if path == NONE_JAR_NAME {
                    self.i_config_mut().custom_jar = None;
                } else if path == OPEN_FOLDER_JAR_NAME {
                    return Ok(Task::done(Message::CoreOpenPath(
                        LAUNCHER_DIR.join("custom_jars"),
                    )));
                } else {
                    self.i_config_mut()
                        .custom_jar
                        .get_or_insert_with(CustomJarConfig::default)
                        .name = path
                }
            }
            EditInstanceMessage::CustomJarLoaded(items) => match items {
                Ok(items) => {
                    match &mut self.custom_jar {
                        Some(cx) => {
                            cx.choices = items.clone();
                        }
                        None => {
                            let watcher =
                                match PathWatcher::new(LAUNCHER_DIR.join("custom_jars"), true) {
                                    Ok(n) => n,
                                    Err(err) => {
                                        err!("Couldn't load list of custom jars (2)! {err}");
                                        return Ok(Task::none());
                                    }
                                };
                            self.custom_jar = Some(CustomJarState {
                                choices: items.clone(),
                                watcher,
                            })
                        }
                    }
                    // If the currently selected jar got deleted/renamed
                    // then unselect it
                    if self.selected_instance.is_some() {
                        if let Some(jar) = &self.i_config().custom_jar {
                            if !items.contains(&jar.name) {
                                self.i_config_mut().custom_jar = None;
                            }
                        }
                    }
                }
                Err(err) => err!("Couldn't load list of custom jars (1)! {err}"),
            },
            EditInstanceMessage::AutoSetMainClassToggle(t) => {
                if let Some(custom_jar) = &mut self.i_config_mut().custom_jar {
                    custom_jar.autoset_main_class = t;
                }
            }
        }
        Ok(Task::none())
    }

    fn add_custom_jar(&mut self) -> Task<Message> {
        if let (Some(custom_jars), Some((path, file_name))) = (
            &mut self.custom_jar,
            rfd::FileDialog::new()
                .set_title("Select Custom Minecraft JAR")
                .add_filter("Java Archive", &["jar"])
                .pick_file()
                .and_then(|n| n.file_name().map(|f| (n.clone(), f.to_owned()))),
        ) {
            let file_name = file_name.to_string_lossy().to_string();
            if !custom_jars.choices.contains(&file_name) {
                custom_jars.choices.insert(1, file_name.clone());
            }

            self.i_config_mut().custom_jar = Some(CustomJarConfig {
                name: file_name.clone(),
                autoset_main_class: false,
            });

            Task::perform(
                tokio::fs::copy(path, LAUNCHER_DIR.join("custom_jars").join(file_name)),
                |_| Message::Nothing,
            )
        } else {
            Task::none()
        }
    }

    fn rename_instance(&mut self) -> Result<Task<Message>, String> {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return Ok(Task::none());
        };

        let mut disallowed = vec![
            '/', '\\', ':', '*', '?', '"', '<', '>', '|', '\'', '\0', '\u{7F}',
        ];
        disallowed.extend('\u{1}'..='\u{1F}');

        // Remove disallowed characters

        let mut instance_name = menu.instance_name.clone();
        instance_name.retain(|c| !disallowed.contains(&c));
        let instance_name = instance_name.trim();

        if instance_name.is_empty() {
            err!("New name is empty or invalid");
            return Ok(Task::none());
        }

        if menu.old_instance_name == menu.instance_name {
            // Don't waste time talking to OS
            // and "renaming" instance if nothing has changed.
            Ok(Task::none())
        } else {
            let instances_dir =
                LAUNCHER_DIR.join(if self.selected_instance.as_ref().unwrap().is_server() {
                    "servers"
                } else {
                    "instances"
                });

            let old_path = instances_dir.join(&menu.old_instance_name);
            let new_path = instances_dir.join(&menu.instance_name);

            menu.old_instance_name = menu.instance_name.clone();
            if let Some(n) = &mut self.selected_instance {
                n.set_name(&menu.instance_name);
            }
            std::fs::rename(&old_path, &new_path)
                .path(&old_path)
                .strerr()?;

            Ok(self.cache.force_update_list())
        }
    }

    fn e_java_arg_edit(&mut self, msg: String, idx: usize) {
        let Some(args) = &mut self.i_config_mut().java_args else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_java_arg_delete(&mut self, idx: usize) {
        if let Some(args) = &mut self.i_config_mut().java_args {
            args.remove(idx);
        }
    }

    fn e_game_arg_edit(&mut self, msg: String, idx: usize) {
        let Some(args) = &mut self.i_config_mut().game_args else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_game_arg_delete(&mut self, idx: usize) {
        if let Some(args) = &mut self.i_config_mut().game_args {
            args.remove(idx);
        }
    }

    fn e_list_shift_up(idx: usize, args: &mut [String]) {
        if idx > 0 {
            args.swap(idx, idx - 1);
        }
    }

    fn e_list_shift_down(idx: usize, args: &mut [String]) {
        if idx + 1 < args.len() {
            args.swap(idx, idx + 1);
        }
    }
}
