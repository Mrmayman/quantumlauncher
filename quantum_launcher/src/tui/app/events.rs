// Centralized AuthEvent handler implementation for App

use crate::tui::app::state::{Account, Instance};
use crate::tui::app::{AddAccountFieldFocus, App};
use std::collections::HashSet;

impl App {
    /// Handle authentication and lifecycle events from async operations
    pub fn handle_auth_event(&mut self, event: crate::tui::AuthEvent) {
        match event {
            crate::tui::AuthEvent::LoginStarted => {
                self.status_message = "🔐 Authenticating with ElyBy...".to_string();
                self.is_loading = true;
            }
            crate::tui::AuthEvent::LoginSuccess { account_data } => {
                self.is_loading = false;
                if let Err(err) = self.save_authenticated_account(&account_data) {
                    self.login_error = Some(format!("Failed to save account: {}", err));
                    self.status_message =
                        format!("❌ Authentication succeeded but failed to save: {}", err);
                    return;
                }
                let account = Account {
                    username: account_data.nice_username.clone(),
                    account_type: account_data.account_type.to_string(),
                    uuid: account_data.uuid.clone(),
                    is_logged_in: true,
                };
                self.accounts.push(account);
                self.current_account = Some(account_data.get_username_modified());
                self.status_message = format!(
                    "✅ Successfully authenticated and saved as {} (set as default)",
                    account_data.nice_username
                );
                self.toggle_add_account_mode();
            }
            crate::tui::AuthEvent::LoginNeedsOtp => {
                self.is_loading = false;
                self.needs_otp = true;
                self.new_account_otp = Some(String::new());
                self.add_account_field_focus = AddAccountFieldFocus::Otp;
                self.status_message =
                    "📱 Two-factor authentication required. Enter your OTP code.".to_string();
            }
            crate::tui::AuthEvent::LoginError { error } => {
                self.is_loading = false;
                self.login_error = Some(error.clone());
                self.status_message = format!("❌ Authentication failed: {}", error);
            }
            crate::tui::AuthEvent::LaunchStarted(instance_name) => {
                self.status_message = format!("LAUNCHING: {}...", instance_name);
                self.is_loading = true;
                self.needs_forced_refresh = true;
                self.add_log(format!(
                    "[{}] Launch started for instance: {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    instance_name
                ));
            }
            crate::tui::AuthEvent::LaunchSuccess(instance_name, child) => {
                self.is_loading = false;
                self.needs_forced_refresh = true;
                let pid = child.lock().map(|c| c.id()).unwrap_or(None);
                let msg = match pid {
                    Some(pid) => format!(
                        "✅ Successfully launched {}! Game is running with PID: {}",
                        instance_name, pid
                    ),
                    None => format!(
                        "✅ Successfully launched {}! Game is running in background.",
                        instance_name
                    ),
                };
                self.status_message = msg.clone();
                self.add_log(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));

                // Store the process for kill functionality
                let client_process = crate::state::ClientProcess {
                    child: child.clone(),
                    receiver: None,
                };
                self.client_processes
                    .insert(instance_name.clone(), client_process);

                // Stream game's stdout/stderr into per-instance buffer via events
                {
                    let (stdout, stderr) = {
                        let mut guard = child.lock().unwrap();
                        (guard.stdout.take(), guard.stderr.take())
                    };
                    if let Some(mut out) = stdout {
                        let inst_name = instance_name.clone();
                        let ui_tx = self.auth_sender.clone();
                        tokio::spawn(async move {
                            use tokio::io::{AsyncBufReadExt, BufReader};
                            let mut lines = BufReader::new(&mut out).lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                let mut s = line;
                                if !s.ends_with('\n') {
                                    s.push('\n');
                                }
                                if let Some(tx) = &ui_tx {
                                    let _ = tx.send(crate::tui::AuthEvent::InstanceLogLine {
                                        instance_name: inst_name.clone(),
                                        line: s,
                                    });
                                }
                            }
                        });
                    }
                    if let Some(mut err) = stderr {
                        let inst_name = instance_name.clone();
                        let ui_tx = self.auth_sender.clone();
                        tokio::spawn(async move {
                            use tokio::io::{AsyncBufReadExt, BufReader};
                            let mut lines = BufReader::new(&mut err).lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                let mut s = line;
                                if !s.ends_with('\n') {
                                    s.push('\n');
                                }
                                if let Some(tx) = &ui_tx {
                                    let _ = tx.send(crate::tui::AuthEvent::InstanceLogLine {
                                        instance_name: inst_name.clone(),
                                        line: s,
                                    });
                                }
                            }
                        });
                    }
                }

                if let Some(instance) = self.instances.iter_mut().find(|i| i.name == instance_name)
                {
                    instance.is_running = true;
                }
                if let Some(sender) = &self.auth_sender {
                    let sender_clone = sender.clone();
                    let instance_name_clone = instance_name.clone();
                    let child_clone = child.clone();
                    tokio::spawn(async move {
                        loop {
                            {
                                let mut child_guard = child_clone.lock().unwrap();
                                if let Ok(Some(_)) = child_guard.try_wait() {
                                    break;
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        let _ = sender_clone
                            .send(crate::tui::AuthEvent::LaunchEnded(instance_name_clone));
                    });
                }
            }
            crate::tui::AuthEvent::LaunchError(instance_name, error) => {
                self.is_loading = false;
                self.needs_forced_refresh = true;
                let msg = format!("❌ Failed to launch {}: {}", instance_name, error);
                self.status_message = msg.clone();
                self.add_log(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
            }
            crate::tui::AuthEvent::LaunchEnded(instance_name) => {
                self.client_processes.remove(&instance_name);
                if let Some(instance) = self.instances.iter_mut().find(|i| i.name == instance_name)
                {
                    instance.is_running = false;
                }
                let msg = format!("🛑 {} has stopped", instance_name);
                self.status_message = msg.clone();
                self.add_log(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
            }
            crate::tui::AuthEvent::InstanceLogLine {
                instance_name,
                line,
            } => {
                let buf = self.instance_logs.entry(instance_name).or_default();
                let ln = line.trim_end_matches('\n').to_string();
                buf.push(ln);
                if buf.len() > 2000 {
                    let excess = buf.len() - 2000;
                    buf.drain(0..excess);
                }
                if self.current_tab == crate::tui::app::TabId::InstanceSettings
                    && self.instance_settings_tab == crate::tui::app::InstanceSettingsTab::Logs
                    && self.instance_logs_auto_follow
                {
                    self.instance_logs_offset = 0;
                }
            }
            crate::tui::AuthEvent::InstanceCreateStarted(instance_name) => {
                self.status_message = format!(
                    "🔨 Creating instance '{}'... This may take a while",
                    instance_name
                );
                self.is_loading = true;
                self.add_log(format!(
                    "[{}] Started creating instance: {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    instance_name
                ));
            }
            crate::tui::AuthEvent::InstanceCreateProgress {
                instance_name,
                message,
            } => {
                self.status_message = format!("🔨 Creating '{}': {}", instance_name, message);
                self.add_log(format!(
                    "[{}] Instance '{}': {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    instance_name,
                    message
                ));
            }
            crate::tui::AuthEvent::InstanceCreateSuccess { instance_name } => {
                self.is_loading = false;
                let msg = format!("✅ Successfully created instance '{}'", instance_name);
                self.status_message = msg.clone();
                self.add_log(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
                self.current_tab = crate::tui::app::TabId::Instances;
                self.status_message = format!(
                    "✅ Successfully created instance '{}'. Press F5 to refresh instances list.",
                    instance_name
                );
            }
            crate::tui::AuthEvent::InstanceCreateError {
                instance_name,
                error,
            } => {
                self.is_loading = false;
                let msg = format!(
                    "❌ Failed to create instance '{}': {}",
                    instance_name, error
                );
                self.status_message = msg.clone();
                self.add_log(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
            }
            crate::tui::AuthEvent::RefreshStarted => {
                self.status_message = "🔄 Refreshing instances...".to_string();
                self.is_loading = true;
                self.add_log(format!(
                    "[{}] Started refreshing instances",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            crate::tui::AuthEvent::RefreshCompleted => {
                self.is_loading = false;
                self.status_message = "✅ Instances refreshed successfully".to_string();
                self.add_log(format!(
                    "[{}] Instances refresh completed",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            crate::tui::AuthEvent::RefreshData { instances } => {
                let running_instances: HashSet<String> = self
                    .instances
                    .iter()
                    .filter(|i| i.is_running)
                    .map(|i| i.name.clone())
                    .collect();
                self.instances.clear();
                for (name, version, loader) in instances {
                    self.instances.push(Instance {
                        name: name.clone(),
                        version,
                        loader,
                        is_running: running_instances.contains(&name),
                    });
                }
                if self.selected_instance >= self.instances.len() && !self.instances.is_empty() {
                    self.selected_instance = self.instances.len() - 1;
                } else if self.instances.is_empty() {
                    self.selected_instance = 0;
                }
            }
        }
    }
}
