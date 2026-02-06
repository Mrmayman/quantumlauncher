use iced::Task;
use ql_core::{
    err,
    json::{
        instance_config::{CustomJarConfig, MainClassMode},
        GlobalSettings, InstanceConfigJson,
    },
    IntoIoError, IntoStringError, LAUNCHER_DIR,
};

use crate::{
    message_handler::format_memory,
    state::{
        dir_watch, get_entries, CustomJarState, EditInstanceMessage, LaunchTab, Launcher,
        MainMenuMessage, MenuCreateInstance, MenuEditInstance, MenuLaunch, Message, ProgressBar,
        State, ADD_JAR_NAME, NONE_JAR_NAME, OPEN_FOLDER_JAR_NAME, REMOVE_JAR_NAME,
    },
};

macro_rules! iflet_config {
    ($state:expr, $config:ident <- $body:block) => {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(MenuEditInstance {
                config: $config, ..
            }),
            ..
        }) = $state
        $body
    };

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

    ($state:expr, prefix, |$prefix:ident| $body:block) => {
        iflet_config!($state, global_settings: global_settings, {
            let global_settings =
                global_settings.get_or_insert_with(GlobalSettings::default);
            let $prefix =
                &mut global_settings.pre_launch_prefix;
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
            EditInstanceMessage::ToggleSplitArg(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.arg_split_by_space = t;
                } else if let State::LauncherSettings(menu) = &mut self.state {
                    menu.arg_split_by_space = t;
                }
            }
            EditInstanceMessage::JavaOverride(n) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.java_override = Some(n);
                }
            }
            EditInstanceMessage::BrowseJavaOverride => {
                if let Some(file) = rfd::FileDialog::new()
                    .set_title("Select Java Executable (./bin/java)")
                    .pick_file()
                {
                    if let State::Launch(MenuLaunch {
                        edit_instance: Some(menu),
                        ..
                    }) = &mut self.state
                    {
                        menu.config.java_override = Some(file.to_string_lossy().to_string());
                    }
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
            EditInstanceMessage::JavaArgsModeChanged(mode) => {
                iflet_config!(&mut self.state, global_java_args_enable, {
                    *global_java_args_enable = Some(mode);
                });
            }
            EditInstanceMessage::JavaArgs(msg) => {
                let split = self.should_split_args();
                iflet_config!(&mut self.state, java_args, {
                    msg.apply(java_args.get_or_insert_with(Vec::new), split);
                });
            }
            EditInstanceMessage::GameArgs(msg) => {
                let split = self.should_split_args();
                iflet_config!(&mut self.state, game_args, {
                    msg.apply(game_args.get_or_insert_with(Vec::new), split);
                });
            }
            EditInstanceMessage::PreLaunchPrefix(msg) => {
                let split = self.should_split_args();
                iflet_config!(&mut self.state, prefix, |pre_launch_prefix| {
                    msg.apply(pre_launch_prefix.get_or_insert_with(Vec::new), split);
                });
            }
            EditInstanceMessage::PreLaunchPrefixModeChanged(mode) => {
                iflet_config!(&mut self.state, pre_launch_prefix_mode, {
                    *pre_launch_prefix_mode = Some(mode);
                });
            }
            EditInstanceMessage::RenameToggle => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.instance_name = self
                        .selected_instance
                        .as_ref()
                        .unwrap()
                        .get_name()
                        .to_owned();
                    menu.is_editing_name = !menu.is_editing_name;
                }
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
                iflet_config!(&mut self.state, config <- {
                    config.c_global_settings().window_width = width.parse::<u32>().ok();
                });
            }
            EditInstanceMessage::WindowHeightChanged(height) => {
                iflet_config!(&mut self.state, config <- {
                    config.c_global_settings().window_height = height.parse::<u32>().ok();
                });
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
                            (&menu.config.custom_jar, &mut self.custom_jar)
                        {
                            list.choices.retain(|n| *n != jar.name);
                            let name = jar.name.clone();
                            menu.config.custom_jar = None;
                            return Ok(Task::perform(
                                tokio::fs::remove_file(LAUNCHER_DIR.join("custom_jars").join(name)),
                                |_| Message::Nothing,
                            ));
                        }
                    } else if path == NONE_JAR_NAME {
                        menu.config.custom_jar = None;
                    } else if path == OPEN_FOLDER_JAR_NAME {
                        return Ok(Task::done(Message::CoreOpenPath(
                            LAUNCHER_DIR.join("custom_jars"),
                        )));
                    } else {
                        menu.config
                            .custom_jar
                            .get_or_insert_with(CustomJarConfig::default)
                            .name = path
                    }
                }
            }
            EditInstanceMessage::CustomJarLoaded(items) => match items {
                Ok(items) => {
                    return Ok(self.loaded_custom_jar(items));
                }
                Err(err) => err!("Couldn't load list of custom jars (1)! {err}"),
            },
            EditInstanceMessage::SetMainClass(t, cls) => {
                if let State::Launch(MenuLaunch {
                    edit_instance:
                        Some(MenuEditInstance {
                            config,
                            main_class_mode,
                            ..
                        }),
                    ..
                }) = &mut self.state
                {
                    *main_class_mode = t;
                    let (name, autos) = match t {
                        Some(MainClassMode::Custom) => (cls, false),
                        Some(MainClassMode::SafeFallback) => (None, true),
                        None => (None, false),
                    };
                    config.main_class_override = name;
                    if let Some(c) = &mut config.custom_jar {
                        c.autoset_main_class = autos;
                    }
                };
            }
            EditInstanceMessage::ReinstallLibraries => {
                return Ok(self.instance_redownload_stage(
                    ql_core::DownloadProgress::DownloadingLibraries {
                        progress: 0,
                        out_of: 0,
                    },
                ));
            }
            EditInstanceMessage::UpdateAssets => {
                return Ok(self.instance_redownload_stage(
                    ql_core::DownloadProgress::DownloadingAssets {
                        progress: 0,
                        out_of: 0,
                    },
                ));
            }
        }
        Ok(Task::none())
    }

    fn instance_redownload_stage(&mut self, stage: ql_core::DownloadProgress) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let bar = ProgressBar::with_recv(receiver);
        self.state = State::Create(MenuCreateInstance::DownloadingInstance(bar));

        Task::perform(
            ql_instances::repeat_stage(self.instance().clone(), stage, Some(sender)),
            |t| {
                if let Err(err) = t {
                    Message::Error(err)
                } else {
                    MainMenuMessage::ChangeTab(LaunchTab::Edit).into()
                }
            },
        )
    }

    fn loaded_custom_jar(&mut self, items: Vec<String>) -> Task<Message> {
        match &mut self.custom_jar {
            Some(cx) => {
                cx.choices = items.clone();
            }
            None => {
                let (recv, watcher) = match dir_watch(LAUNCHER_DIR.join("custom_jars")) {
                    Ok(n) => n,
                    Err(err) => {
                        err!("Couldn't load list of custom jars (2)! {err}");
                        return Task::none();
                    }
                };
                self.custom_jar = Some(CustomJarState {
                    choices: items.clone(),
                    recv,
                    _watcher: watcher,
                });
            }
        }
        // If the currently selected jar got deleted/renamed
        // then unselect it
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            if let Some(jar) = &menu.config.custom_jar {
                if !items.contains(&jar.name) {
                    menu.config.custom_jar = None;
                }
            }
        }
        Task::none()
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
            &mut self.custom_jar,
            &mut self.state,
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
                get_entries(self.instance().is_server()),
                Message::CoreListLoaded,
            ))
        }
    }
}
