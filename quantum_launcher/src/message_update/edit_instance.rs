use iced::Task;
use ql_core::{
    err,
    json::{instance_config::CustomJarConfig, GlobalSettings, InstanceConfigJson},
    IntoIoError, IntoStringError, LAUNCHER_DIR,
};

use crate::{
    message_handler::format_memory,
    state::{
        get_entries, EditInstanceMessage, Launcher, MenuEditInstance, MenuLaunch, Message, State,
        ADD_JAR_NAME, NONE_JAR_NAME, REMOVE_JAR_NAME,
    },
};

use super::add_to_arguments_list;

macro_rules! iflet_config {
    // Match pattern with one field (e.g. java_args: Some(args))
    ($state:expr, $field:ident : $pat:pat, $body:block) => {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(MenuEditInstance {
                config: InstanceConfigJson {
                    $field: $pat,
                    ..
                },
                ..
            }),
            ..
        }) = $state
        $body
    };

    ($state:expr, $field:ident, $body:block) => {
        iflet_config!($state, $field : $field, $body);
    };

    ($state:expr, get, $field:ident, $body:block) => {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(MenuEditInstance {
                config: InstanceConfigJson {
                    $field,
                    ..
                },
                ..
            }),
            ..
        }) = $state
        {
            let $field = $field.get_or_insert_with(Default::default);
            $body
        }
    };

    ($state:expr, prefix, |$prefix:ident| $body:block) => {
        iflet_config!($state, global_settings: global_settings, {
            let global_settings =
                global_settings.get_or_insert_with(GlobalSettings::default);
            let $prefix =
                global_settings.pre_launch_prefix.get_or_insert_with(Vec::new);
            $body
        });
    };
}

impl Launcher {
    pub fn update_edit_instance(
        &mut self,
        message: EditInstanceMessage,
    ) -> Result<Task<Message>, String> {
        match message {
            EditInstanceMessage::JavaOverride(n) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.java_override = Some(n);
                }
            }
            EditInstanceMessage::MemoryChanged(new_slider_value) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.slider_value = new_slider_value;
                    menu.config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    menu.slider_text = format_memory(menu.config.ram_in_mb);
                }
            }
            EditInstanceMessage::LoggingToggle(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.enable_logger = Some(t);
                }
            }
            EditInstanceMessage::CloseLauncherToggle(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.close_on_start = Some(t);
                }
            }
            EditInstanceMessage::JavaArgsAdd => {
                iflet_config!(&mut self.state, get, java_args, {
                    java_args.push(String::new());
                });
            }
            EditInstanceMessage::JavaArgEdit(msg, idx) => {
                self.e_java_arg_edit(msg, idx);
            }
            EditInstanceMessage::JavaArgDelete(idx) => {
                self.e_java_arg_delete(idx);
            }
            EditInstanceMessage::JavaArgsModeChanged(mode) => {
                iflet_config!(&mut self.state, java_args_mode, {
                    *java_args_mode = Some(mode);
                });
            }
            EditInstanceMessage::GameArgsAdd => {
                iflet_config!(&mut self.state, get, game_args, {
                    game_args.push(String::new());
                });
            }
            EditInstanceMessage::GameArgEdit(msg, idx) => {
                self.e_game_arg_edit(msg, idx);
            }
            EditInstanceMessage::GameArgDelete(idx) => {
                self.e_game_arg_delete(idx);
            }
            EditInstanceMessage::JavaArgShiftUp(idx) => {
                iflet_config!(&mut self.state, java_args: Some(args), {
                    Self::e_list_shift_up(idx, args);
                });
            }
            EditInstanceMessage::JavaArgShiftDown(idx) => {
                iflet_config!(&mut self.state, java_args: Some(args), {
                    Self::e_list_shift_down(idx, args);
                });
            }
            EditInstanceMessage::GameArgShiftUp(idx) => {
                iflet_config!(&mut self.state, game_args: Some(args), {
                    Self::e_list_shift_up(idx, args);
                });
            }
            EditInstanceMessage::GameArgShiftDown(idx) => {
                iflet_config!(&mut self.state, game_args: Some(args), {
                    Self::e_list_shift_down(idx, args);
                });
            }
            EditInstanceMessage::PreLaunchPrefixAdd => {
                iflet_config!(&mut self.state, prefix, |pre_launch_prefix| {
                    pre_launch_prefix.push(String::new());
                });
            }
            EditInstanceMessage::PreLaunchPrefixEdit(msg, idx) => {
                self.e_pre_launch_prefix_edit(msg, idx);
            }
            EditInstanceMessage::PreLaunchPrefixDelete(idx) => {
                self.e_pre_launch_prefix_delete(idx);
            }
            EditInstanceMessage::PreLaunchPrefixShiftUp(idx) => {
                iflet_config!(&mut self.state, prefix, |pre_launch_prefix| {
                    Self::e_list_shift_up(idx, pre_launch_prefix);
                });
            }
            EditInstanceMessage::PreLaunchPrefixShiftDown(idx) => {
                iflet_config!(&mut self.state, prefix, |pre_launch_prefix| {
                    Self::e_list_shift_down(idx, pre_launch_prefix);
                });
            }
            EditInstanceMessage::PreLaunchPrefixModeChanged(mode) => {
                iflet_config!(&mut self.state, pre_launch_prefix_mode, {
                    *pre_launch_prefix_mode = Some(mode);
                });
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
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config
                        .global_settings
                        .get_or_insert_with(Default::default)
                        .window_width = if width.is_empty() {
                        None
                    } else {
                        // TODO: Error handling
                        width.parse::<u32>().ok()
                    }
                }
            }
            EditInstanceMessage::WindowHeightChanged(height) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config
                        .global_settings
                        .get_or_insert_with(Default::default)
                        .window_height = if height.is_empty() {
                        None
                    } else {
                        // TODO: Error handling
                        height.parse::<u32>().ok()
                    }
                }
            }
            EditInstanceMessage::CustomJarPathChanged(path) => {
                if path == ADD_JAR_NAME {
                    return Ok(self.add_custom_jar());
                } else if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    if path == REMOVE_JAR_NAME {
                        if let (Some(jar), Some(list)) =
                            (&menu.config.custom_jar, &mut self.custom_jar_choices)
                        {
                            list.retain(|n| *n != jar.name);
                            let name = jar.name.clone();
                            menu.config.custom_jar = None;
                            return Ok(Task::perform(
                                tokio::fs::remove_file(LAUNCHER_DIR.join("custom_jars").join(name)),
                                |_| Message::Nothing,
                            ));
                        }
                    } else if path == NONE_JAR_NAME {
                        menu.config.custom_jar = None;
                    } else {
                        menu.config
                            .custom_jar
                            .get_or_insert_with(CustomJarConfig::default)
                            .name = path
                    }
                }
            }
            EditInstanceMessage::CustomJarLoaded(items) => match items {
                Ok(items) => self.custom_jar_choices = Some(items),
                Err(err) => err!("Couldn't load list of custom jars! {err}"),
            },
            EditInstanceMessage::AutoSetMainClassToggle(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance:
                        Some(MenuEditInstance {
                            config:
                                InstanceConfigJson {
                                    custom_jar: Some(custom_jar),
                                    ..
                                },
                            ..
                        }),
                    ..
                }) = &mut self.state
                {
                    custom_jar.autoset_main_class = t;
                }
            }
        }
        Ok(Task::none())
    }

    fn add_custom_jar(&mut self) -> Task<Message> {
        if let (
            Some(custom_jars),
            State::Launch(MenuLaunch {
                edit_instance: Some(menu),
                ..
            }),
            Some((path, file_name)),
        ) = (
            &mut self.custom_jar_choices,
            &mut self.state,
            rfd::FileDialog::new()
                .set_title("Select Custom Minecraft JAR")
                .add_filter("Java Archive", &["jar"])
                .pick_file()
                .and_then(|n| n.file_name().map(|f| (n.clone(), f.to_owned()))),
        ) {
            let file_name = file_name.to_string_lossy().to_string();
            if !custom_jars.contains(&file_name) {
                custom_jars.insert(1, file_name.clone());
            }

            *menu
                .config
                .custom_jar
                .get_or_insert_with(CustomJarConfig::default) = CustomJarConfig {
                name: file_name.clone(),
                autoset_main_class: false,
            };

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

            Ok(Task::perform(
                get_entries(self.selected_instance.as_ref().unwrap().is_server()),
                Message::CoreListLoaded,
            ))
        }
    }

    fn e_java_arg_edit(&mut self, msg: String, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        let Some(args) = menu.config.java_args.as_mut() else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_java_arg_delete(&mut self, idx: usize) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            if let Some(args) = &mut menu.config.java_args {
                args.remove(idx);
            }
        }
    }

    fn e_game_arg_edit(&mut self, msg: String, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_game_arg_delete(&mut self, idx: usize) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            if let Some(args) = &mut menu.config.game_args {
                args.remove(idx);
            }
        }
    }

    fn e_list_shift_up(idx: usize, args: &mut Vec<String>) {
        if idx > 0 {
            args.swap(idx, idx - 1);
        }
    }

    fn e_list_shift_down(idx: usize, args: &mut [String]) {
        if idx + 1 < args.len() {
            args.swap(idx, idx + 1);
        }
    }

    fn e_pre_launch_prefix_edit(&mut self, msg: String, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        add_to_arguments_list(msg, menu.config.get_launch_prefix(), idx);
    }

    fn e_pre_launch_prefix_delete(&mut self, idx: usize) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            menu.config.get_launch_prefix().remove(idx);
        }
    }
}
