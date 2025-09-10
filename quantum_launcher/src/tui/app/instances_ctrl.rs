// Instances controls: refresh (sync and async)

use crate::tui::app::{App, ArgsEditKind, Instance};
use ql_core::file_utils;
use ql_core::json::InstanceConfigJson;
use std::collections::HashSet;

impl App {
    /// Preload current memory value from config without opening popup
    pub fn preload_memory_summary(&mut self) {
        let Some(idx) = self.instance_settings_instance else {
            return;
        };
        let instance_name = if let Some(i) = self.instances.get(idx) {
            i.name.clone()
        } else {
            return;
        };
        let (mb, _err) = match file_utils::get_launcher_dir() {
            Ok(dir) => {
                let mut p = dir;
                p.push("instances");
                p.push(&instance_name);
                match std::fs::read_to_string(p.join("config.json")) {
                    Ok(s) => match serde_json::from_str::<InstanceConfigJson>(&s) {
                        Ok(cfg) => (cfg.ram_in_mb, None),
                        Err(e) => (2048, Some(format!("parse error: {}", e))),
                    },
                    Err(e) => (2048, Some(format!("read error: {}", e))),
                }
            }
            Err(e) => (2048, Some(format!("dir error: {}", e))),
        };
        self.memory_edit_mb = mb;
        self.memory_edit_input = mb.to_string();
    }
    /// Open the memory edit popup for the current instance in InstanceSettings
    pub fn open_memory_edit(&mut self) {
        let Some(idx) = self.instance_settings_instance else {
            self.status_message = "No instance selected".to_string();
            return;
        };
        let instance_name = self.instances.get(idx).map(|i| i.name.clone());
        if instance_name.is_none() {
            return;
        }
        let instance_name = instance_name.unwrap();
        // Preload from disk
        let (last_mb, err) = match file_utils::get_launcher_dir() {
            Ok(dir) => {
                let mut p = dir;
                p.push("instances");
                p.push(&instance_name);
                match std::fs::read_to_string(p.join("config.json")) {
                    Ok(s) => match serde_json::from_str::<InstanceConfigJson>(&s) {
                        Ok(cfg) => (cfg.ram_in_mb, None),
                        Err(e) => (2048, Some(format!("parse error: {}", e))),
                    },
                    Err(e) => (2048, Some(format!("read error: {}", e))),
                }
            }
            Err(e) => (2048, Some(format!("dir error: {}", e))),
        };
        self.memory_edit_mb = last_mb;
        self.memory_edit_input = last_mb.to_string();
        self.is_editing_memory = true;
        self.status_message = match err {
            Some(e) => format!(
                "Editing memory (loaded default due to {}): {} MB",
                e, last_mb
            ),
            None => format!("Editing memory: {} MB", last_mb),
        };
    }

    /// Apply memory edit: validate, save to config.json, and close popup
    pub fn apply_memory_edit(&mut self) {
        if !self.is_editing_memory {
            return;
        }
        let Some(idx) = self.instance_settings_instance else {
            return;
        };
        let Some(inst) = self.instances.get(idx) else {
            return;
        };
        let instance_name = inst.name.clone();

        // Parse input; allow suffixes like G/GB
        let txt = self.memory_edit_input.trim();
        let parsed_mb = if txt.is_empty() {
            None
        } else {
            let lower = txt.to_ascii_lowercase();
            if let Some(stripped) = lower.strip_suffix("gb").or_else(|| lower.strip_suffix('g')) {
                stripped
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|g| (g * 1024.0).round() as usize)
            } else if let Some(stripped) =
                lower.strip_suffix("mb").or_else(|| lower.strip_suffix('m'))
            {
                stripped.trim().parse::<usize>().ok()
            } else {
                lower.parse::<usize>().ok()
            }
        };

        let Some(mb) = parsed_mb else {
            self.status_message = "❌ Invalid memory value. Examples: 2048, 2G, 4GB".to_string();
            return;
        };

        // Clamp sensible range: 256 MB .. 16384 MB
        let mb = mb.clamp(256, 16384);

        // Load, update, save config
        match file_utils::get_launcher_dir() {
            Ok(dir) => {
                let path = dir
                    .join("instances")
                    .join(&instance_name)
                    .join("config.json");
                match std::fs::read_to_string(&path) {
                    Ok(s) => match serde_json::from_str::<InstanceConfigJson>(&s) {
                        Ok(mut cfg) => {
                            cfg.ram_in_mb = mb;
                            match serde_json::to_string_pretty(&cfg) {
                                Ok(new_s) => match std::fs::write(&path, new_s) {
                                    Ok(_) => {
                                        self.is_editing_memory = false;
                                        self.memory_edit_mb = mb;
                                        self.status_message = format!("✅ Saved memory: {} MB", mb);
                                    }
                                    Err(e) => {
                                        self.status_message =
                                            format!("❌ Failed to write config: {}", e);
                                    }
                                },
                                Err(e) => {
                                    self.status_message =
                                        format!("❌ Failed to serialize config: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("❌ Failed to parse config: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("❌ Failed to read config: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("❌ Failed to get launcher directory: {}", e);
            }
        }
    }

    /// Cancel memory edit
    pub fn cancel_memory_edit(&mut self) {
        self.is_editing_memory = false;
        self.status_message = "Cancelled memory edit".to_string();
    }

    /// Apply args edit (Java/Game) to config.json and close popup
    pub fn apply_args_edit(&mut self) {
        if !self.is_editing_args {
            return;
        }
        let Some(idx) = self.instance_settings_instance else {
            return;
        };
        let Some(inst) = self.instances.get(idx) else {
            return;
        };
        let instance_name = inst.name.clone();

        let parsed_args = parse_shell_like_args(self.args_edit_input.trim());

        match ql_core::file_utils::get_launcher_dir() {
            Ok(dir) => {
                let path = dir
                    .join("instances")
                    .join(&instance_name)
                    .join("config.json");
                match std::fs::read_to_string(&path) {
                    Ok(s) => match serde_json::from_str::<ql_core::json::InstanceConfigJson>(&s) {
                        Ok(mut cfg) => {
                            match self.args_edit_kind {
                                ArgsEditKind::Java => cfg.java_args = Some(parsed_args),
                                ArgsEditKind::Game => cfg.game_args = Some(parsed_args),
                                ArgsEditKind::GlobalJava => {}
                                ArgsEditKind::WindowSize => {}
                                ArgsEditKind::GlobalWindowSize => {}
                                ArgsEditKind::GlobalTuiRefreshInterval => {}
                                ArgsEditKind::PreLaunchPrefixInstance => {}
                                ArgsEditKind::PreLaunchPrefixGlobal => {}
                            }
                            match serde_json::to_string_pretty(&cfg) {
                                Ok(new_s) => match std::fs::write(&path, new_s) {
                                    Ok(_) => {
                                        self.is_editing_args = false;
                                        self.status_message = match self.args_edit_kind {
                                            ArgsEditKind::Java => "✅ Saved Java arguments",
                                            ArgsEditKind::Game => "✅ Saved Game arguments",
                                            ArgsEditKind::GlobalJava => "",
                                            ArgsEditKind::WindowSize => "",
                                            ArgsEditKind::GlobalWindowSize => "",
                                            ArgsEditKind::GlobalTuiRefreshInterval => "",
                                            ArgsEditKind::PreLaunchPrefixInstance => "",
                                            ArgsEditKind::PreLaunchPrefixGlobal => "",
                                        }
                                        .to_string();
                                    }
                                    Err(e) => {
                                        self.status_message =
                                            format!("❌ Failed to write config: {}", e);
                                    }
                                },
                                Err(e) => {
                                    self.status_message =
                                        format!("❌ Failed to serialize config: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("❌ Failed to parse config: {}", e);
                        }
                    },
                    Err(e) => {
                        self.status_message = format!("❌ Failed to read config: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("❌ Failed to get launcher directory: {}", e);
            }
        }
    }

    /// Cancel args edit popup
    pub fn cancel_args_edit(&mut self) {
        self.is_editing_args = false;
        self.status_message = "Cancelled arguments edit".to_string();
    }
    /// Start async refresh of instances
    pub fn start_refresh(&mut self) {
        if let Some(sender) = &self.auth_sender {
            let sender_clone = sender.clone();
            let _ = sender.send(crate::tui::AuthEvent::RefreshStarted);
            tokio::spawn(async move {
                match crate::state::get_entries("instances".to_owned(), false).await {
                    Ok((instance_names, _)) => {
                        let mut instance_data = Vec::new();
                        for name in instance_names {
                            let instance_dir = match ql_core::file_utils::get_launcher_dir() {
                                Ok(launcher_dir) => launcher_dir.join("instances").join(&name),
                                Err(_) => continue,
                            };
                            let loader = match ql_core::json::InstanceConfigJson::read_from_dir(
                                &instance_dir,
                            )
                            .await
                            {
                                Ok(cfg) => cfg.mod_type,
                                Err(_) => "Vanilla".to_string(),
                            };
                            let version =
                                match ql_core::json::VersionDetails::load_from_path(&instance_dir)
                                    .await
                                {
                                    Ok(details) => details.id,
                                    Err(_) => "Unknown".to_string(),
                                };
                            instance_data.push((name, version, loader));
                        }
                        let _ = sender_clone.send(crate::tui::AuthEvent::RefreshData {
                            instances: instance_data,
                        });
                        let _ = sender_clone.send(crate::tui::AuthEvent::RefreshCompleted);
                    }
                    Err(_) => {
                        let _ = sender_clone.send(crate::tui::AuthEvent::RefreshCompleted);
                    }
                }
            });
        }
    }

    /// Synchronous refresh used on startup
    pub fn refresh(&mut self) {
        use crate::state::get_entries;
        use ql_core::file_utils;
        use ql_core::json::{InstanceConfigJson, VersionDetails};
        use std::path::PathBuf;

        self.is_loading = true;
        self.status_message = "Refreshing...".to_string();

        let running_instances: HashSet<String> = self
            .instances
            .iter()
            .filter(|i| i.is_running)
            .map(|i| i.name.clone())
            .collect();

        match tokio::runtime::Runtime::new() {
            Ok(rt) => {
                match rt.block_on(get_entries("instances".to_owned(), false)) {
                    Ok((instance_names, _)) => {
                        self.instances.clear();
                        let launcher_dir = match file_utils::get_launcher_dir() {
                            Ok(dir) => dir,
                            Err(_) => PathBuf::from(".config/QuantumLauncher"),
                        };
                        let instances_dir = launcher_dir.join("instances");
                        for name in instance_names {
                            let instance_dir = instances_dir.join(&name);
                            let loader = match rt
                                .block_on(InstanceConfigJson::read_from_dir(&instance_dir))
                            {
                                Ok(cfg) => cfg.mod_type,
                                Err(_) => "Vanilla".to_string(),
                            };
                            let version =
                                match rt.block_on(VersionDetails::load_from_path(&instance_dir)) {
                                    Ok(details) => details.id,
                                    Err(_) => "Unknown".to_string(),
                                };
                            self.instances.push(Instance {
                                name: name.clone(),
                                version,
                                loader,
                                is_running: running_instances.contains(&name),
                            });
                        }
                        self.status_message = format!("Loaded {} instances", self.instances.len());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load instances: {}", e);
                    }
                }
                if self.available_versions.is_empty() {
                    match rt.block_on(ql_instances::list_versions()) {
                        Ok(versions) => {
                            self.available_versions = versions;
                            self.update_filtered_versions();
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load versions: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Failed to create runtime: {}", e);
            }
        }

        self.is_loading = false;
    }
}

/// Parse a shell-like string into arguments, supporting quotes and escapes
pub fn parse_shell_like_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut esc = false;
    for ch in input.chars() {
        if esc {
            cur.push(match ch {
                'n' => '\n',
                't' => '\t',
                '"' => '"',
                '\'' => '\'',
                '\\' => '\\',
                other => other,
            });
            esc = false;
            continue;
        }
        match ch {
            '\\' if in_single => cur.push('\\'),
            '\\' => {
                esc = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            // Treat comma like a separator (like in the Iced UI), but only when not inside quotes
            c if (c.is_whitespace() || c == ',') && !in_single && !in_double => {
                if !cur.is_empty() {
                    args.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        args.push(cur);
    }
    args
}
