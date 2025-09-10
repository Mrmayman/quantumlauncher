// Launch helpers: account selection, username cleanup, and process launching

use crate::tui::app::App;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

impl App {
    /// Launch the currently selected instance (for Shift+Enter)
    pub fn launch_selected_instance(&mut self) {
        if let Some(instance) = self.instances.get(self.selected_instance) {
            let instance_name = instance.name.clone();
            self.launch_instance(&instance_name);
        }
    }

    pub(crate) fn launch_instance(&mut self, instance_name: &str) {
        self.status_message = format!("LAUNCHING: {}", instance_name);
        let account_data = self.get_account_data_for_launch();
        let (username, _display_name) = if let Some(acc) = &account_data {
            let clean_username = self.get_clean_username_for_launch(
                &acc.username,
                &acc.nice_username,
                &acc.account_type,
            );
            self.status_message =
                format!("LAUNCHING: {} ({})", acc.nice_username, acc.account_type);
            (clean_username, acc.nice_username.clone())
        } else if !self.accounts.is_empty() && self.selected_account < self.accounts.len() {
            let selected_account = &self.accounts[self.selected_account];
            if selected_account.account_type == "Offline" {
                let clean_username = self.get_clean_username_for_selected_account(selected_account);
                self.status_message = format!("LAUNCHING: offline account {}", clean_username);
                (clean_username, selected_account.username.clone())
            } else {
                let fallback_account_data = self.get_selected_account_data_fallback();
                if let Some(acc) = fallback_account_data {
                    let clean_username = self.get_clean_username_for_launch(
                        &acc.username,
                        &acc.nice_username,
                        &acc.account_type,
                    );
                    self.status_message = format!(
                        "LAUNCHING: {} ({})",
                        acc.nice_username, selected_account.account_type
                    );
                    (clean_username, acc.nice_username.clone())
                } else {
                    let clean_username =
                        self.get_clean_username_for_selected_account(selected_account);
                    self.status_message = format!(
                        "WARNING: Using {} ({}) offline - may need re-auth",
                        clean_username, selected_account.account_type
                    );
                    (clean_username, selected_account.username.clone())
                }
            }
        } else {
            self.status_message = "WARNING: No accounts available. Launching offline as 'Player'. Add account with 'a' then 'n'.".to_string();
            ("Player".to_string(), "Player".to_string())
        };

        if let Some(sender) = &self.auth_sender {
            let sender_clone = sender.clone();
            let instance_name = instance_name.to_string();
            let final_account_data = if account_data.is_some() {
                account_data
            } else if !self.accounts.is_empty() && self.selected_account < self.accounts.len() {
                let selected_account = &self.accounts[self.selected_account];
                if selected_account.account_type == "Offline" {
                    None
                } else {
                    self.get_selected_account_data_fallback()
                        .or_else(|| self.create_minimal_account_data_for_authlib(selected_account))
                }
            } else {
                None
            };
            let _ = sender.send(crate::tui::AuthEvent::LaunchStarted(instance_name.clone()));
            tokio::spawn(async move {
                let result = App::launch_with_suppressed_output(
                    instance_name.clone(),
                    username,
                    final_account_data,
                    sender_clone.clone(),
                )
                .await;
                match result {
                    Ok(child) => {
                        let _ = sender_clone
                            .send(crate::tui::AuthEvent::LaunchSuccess(instance_name, child));
                    }
                    Err(e) => {
                        let _ = sender_clone.send(crate::tui::AuthEvent::LaunchError(
                            instance_name,
                            e.to_string(),
                        ));
                    }
                }
            });
        } else {
            self.status_message = "❌ Authentication system not initialized".to_string();
        }
    }

    pub async fn launch_with_suppressed_output(
        instance_name: String,
        username: String,
        account_data: Option<ql_instances::auth::AccountData>,
        sender: tokio::sync::mpsc::UnboundedSender<crate::tui::AuthEvent>,
    ) -> Result<Arc<Mutex<tokio::process::Child>>, Box<dyn Error + Send + Sync>> {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let launch_mode = if let Some(ref acc) = account_data {
            if acc.access_token.is_some() {
                format!("with {} account: {}", acc.account_type, acc.nice_username)
            } else {
                format!("in offline mode as: {}", acc.nice_username)
            }
        } else {
            format!("in offline mode as: {}", username)
        };
        let _ = sender.send(crate::tui::AuthEvent::LaunchStarted(format!(
            "[{}] Preparing to launch {} {}",
            timestamp, instance_name, launch_mode
        )));
        std::env::set_var("QL_QUIET_LAUNCH", "true");
        match ql_instances::launch(
            instance_name.clone(),
            username.clone(),
            None,
            account_data,
            None,
            Vec::new(),
        )
        .await
        {
            Ok(child_arc) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let pid = child_arc.lock().map(|child| child.id()).unwrap_or(None);
                let timestamp = chrono::Local::now().format("%H:%M:%S");
                let _ = sender.send(crate::tui::AuthEvent::LaunchStarted(format!(
                    "[{}] Minecraft process started with PID: {:?}",
                    timestamp, pid
                )));
                let _ = sender.send(crate::tui::AuthEvent::LaunchStarted(format!(
                    "[{}] Launch completed successfully!",
                    timestamp
                )));
                Ok(child_arc)
            }
            Err(e) => {
                let timestamp = chrono::Local::now().format("%H:%M:%S");
                let _ = sender.send(crate::tui::AuthEvent::LaunchStarted(format!(
                    "[{}] Launch failed: {}",
                    timestamp, e
                )));
                Err(Box::new(e) as Box<dyn Error + Send + Sync>)
            }
        }
    }

    fn get_account_data_for_launch(&self) -> Option<ql_instances::auth::AccountData> {
        if self.accounts.is_empty() || self.selected_account >= self.accounts.len() {
            return None;
        }
        let account = &self.accounts[self.selected_account];
        if account.account_type == "Offline" {
            return None;
        }
        if let Ok(config) = crate::config::LauncherConfig::load_s() {
            if let Some(config_accounts) = config.accounts {
                if let Some(config_account) = config_accounts.get(&account.username) {
                    let account_type = match account.account_type.as_str() {
                        "ElyBy" => ql_instances::auth::AccountType::ElyBy,
                        "LittleSkin" => ql_instances::auth::AccountType::LittleSkin,
                        _ => ql_instances::auth::AccountType::Microsoft,
                    };
                    let keyring_username = if let Some(keyring_id) =
                        &config_account.keyring_identifier
                    {
                        keyring_id.clone()
                    } else {
                        match account_type {
                            ql_instances::auth::AccountType::ElyBy => account
                                .username
                                .strip_suffix(" (elyby)")
                                .unwrap_or(&account.username)
                                .to_string(),
                            ql_instances::auth::AccountType::LittleSkin => account
                                .username
                                .strip_suffix(" (littleskin)")
                                .unwrap_or(&account.username)
                                .to_string(),
                            ql_instances::auth::AccountType::Microsoft => account.username.clone(),
                        }
                    };
                    match ql_instances::auth::read_refresh_token(&keyring_username, account_type) {
                        Ok(refresh_token) => {
                            return Some(ql_instances::auth::AccountData {
                                access_token: Some("0".to_string()),
                                uuid: config_account.uuid.clone(),
                                refresh_token,
                                needs_refresh: true,
                                account_type,
                                username: keyring_username.clone(),
                                nice_username: config_account
                                    .username_nice
                                    .clone()
                                    .unwrap_or(keyring_username),
                            });
                        }
                        Err(_) => {
                            if matches!(
                                account_type,
                                ql_instances::auth::AccountType::ElyBy
                                    | ql_instances::auth::AccountType::LittleSkin
                            ) {
                                let nice_username = config_account
                                    .username_nice
                                    .clone()
                                    .unwrap_or_else(|| account.username.clone());
                                return Some(ql_instances::auth::AccountData {
                                    access_token: Some("0".to_string()),
                                    uuid: config_account.uuid.clone(),
                                    refresh_token: String::new(),
                                    needs_refresh: true,
                                    account_type,
                                    username: account.username.clone(),
                                    nice_username,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn get_selected_account_data_fallback(&self) -> Option<ql_instances::auth::AccountData> {
        self.get_selected_account_data()
    }

    fn get_selected_account_data(&self) -> Option<ql_instances::auth::AccountData> {
        if self.accounts.is_empty() || self.selected_account >= self.accounts.len() {
            return None;
        }
        let account = &self.accounts[self.selected_account];
        if account.account_type == "Offline" {
            return Some(ql_instances::auth::AccountData {
                access_token: Some("0".to_string()),
                uuid: account.uuid.clone(),
                refresh_token: String::new(),
                needs_refresh: false,
                account_type: ql_instances::auth::AccountType::Microsoft,
                username: account.username.clone(),
                nice_username: account.username.clone(),
            });
        }
        if let Ok(config) = crate::config::LauncherConfig::load_s() {
            if let Some(config_accounts) = config.accounts {
                if let Some(config_account) = config_accounts.get(&account.username) {
                    let account_type = match account.account_type.as_str() {
                        "ElyBy" => ql_instances::auth::AccountType::ElyBy,
                        "LittleSkin" => ql_instances::auth::AccountType::LittleSkin,
                        _ => ql_instances::auth::AccountType::Microsoft,
                    };
                    let keyring_username = if let Some(keyring_id) =
                        &config_account.keyring_identifier
                    {
                        keyring_id.clone()
                    } else {
                        match account_type {
                            ql_instances::auth::AccountType::ElyBy => account
                                .username
                                .strip_suffix(" (elyby)")
                                .unwrap_or(&account.username)
                                .to_string(),
                            ql_instances::auth::AccountType::LittleSkin => account
                                .username
                                .strip_suffix(" (littleskin)")
                                .unwrap_or(&account.username)
                                .to_string(),
                            ql_instances::auth::AccountType::Microsoft => account.username.clone(),
                        }
                    };
                    match ql_instances::auth::read_refresh_token(&keyring_username, account_type) {
                        Ok(refresh_token) => {
                            return Some(ql_instances::auth::AccountData {
                                access_token: Some("0".to_string()),
                                uuid: config_account.uuid.clone(),
                                refresh_token,
                                needs_refresh: true,
                                account_type,
                                username: keyring_username.clone(),
                                nice_username: config_account
                                    .username_nice
                                    .clone()
                                    .unwrap_or(keyring_username),
                            });
                        }
                        Err(_) => {
                            if matches!(
                                account_type,
                                ql_instances::auth::AccountType::ElyBy
                                    | ql_instances::auth::AccountType::LittleSkin
                            ) {
                                let nice_username = config_account
                                    .username_nice
                                    .clone()
                                    .unwrap_or_else(|| account.username.clone());
                                return Some(ql_instances::auth::AccountData {
                                    access_token: Some("0".to_string()),
                                    uuid: config_account.uuid.clone(),
                                    refresh_token: String::new(),
                                    needs_refresh: true,
                                    account_type,
                                    username: account.username.clone(),
                                    nice_username,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn create_minimal_account_data_for_authlib(
        &self,
        account: &super::state::Account,
    ) -> Option<ql_instances::auth::AccountData> {
        let account_type = match account.account_type.as_str() {
            "ElyBy" => ql_instances::auth::AccountType::ElyBy,
            "LittleSkin" => ql_instances::auth::AccountType::LittleSkin,
            _ => return None,
        };
        let nice_username = if let Ok(config) = crate::config::LauncherConfig::load_s() {
            if let Some(config_accounts) = config.accounts {
                if let Some(config_account) = config_accounts.get(&account.username) {
                    config_account
                        .username_nice
                        .clone()
                        .unwrap_or_else(|| account.username.clone())
                } else {
                    account.username.clone()
                }
            } else {
                account.username.clone()
            }
        } else {
            account.username.clone()
        };
        Some(ql_instances::auth::AccountData {
            access_token: Some("0".to_string()),
            uuid: account.uuid.clone(),
            refresh_token: String::new(),
            needs_refresh: true,
            username: account.username.clone(),
            nice_username,
            account_type,
        })
    }

    fn get_clean_username_for_launch(
        &self,
        username: &str,
        nice_username: &str,
        account_type: &ql_instances::auth::AccountType,
    ) -> String {
        match account_type {
            ql_instances::auth::AccountType::ElyBy
            | ql_instances::auth::AccountType::LittleSkin => {
                if nice_username
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                    && nice_username.len() <= 16
                {
                    nice_username.to_string()
                } else if let Some(at_pos) = username.find('@') {
                    let local_part = &username[..at_pos];
                    let clean = local_part
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .take(16)
                        .collect::<String>();
                    if clean.is_empty() {
                        format!(
                            "User_{}",
                            username
                                .chars()
                                .filter(|c| c.is_alphanumeric())
                                .take(8)
                                .collect::<String>()
                        )
                    } else {
                        clean
                    }
                } else {
                    let clean = username
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .take(16)
                        .collect::<String>();
                    if clean.is_empty() {
                        "Player".to_string()
                    } else {
                        clean
                    }
                }
            }
            ql_instances::auth::AccountType::Microsoft => {
                if nice_username
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                    && nice_username.len() <= 16
                {
                    nice_username.to_string()
                } else {
                    let clean = username
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .take(16)
                        .collect::<String>();
                    if clean.is_empty() {
                        "Player".to_string()
                    } else {
                        clean
                    }
                }
            }
        }
    }

    fn get_clean_username_for_selected_account(&self, account: &super::state::Account) -> String {
        if account.account_type == "Offline" {
            let clean = account
                .username
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .take(16)
                .collect::<String>();
            return if clean.is_empty() {
                "Player".to_string()
            } else {
                clean
            };
        }
        if let Ok(config) = crate::config::LauncherConfig::load_s() {
            if let Some(config_accounts) = config.accounts {
                if let Some(config_account) = config_accounts.get(&account.username) {
                    if let Some(nice_username) = &config_account.username_nice {
                        if nice_username
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_')
                            && nice_username.len() <= 16
                        {
                            return nice_username.clone();
                        }
                    }
                }
            }
        }
        match account.account_type.as_str() {
            "ElyBy" | "LittleSkin" => {
                if let Some(at_pos) = account.username.find('@') {
                    let local_part = &account.username[..at_pos];
                    let clean = local_part
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .take(16)
                        .collect::<String>();
                    if clean.is_empty() {
                        format!(
                            "User_{}",
                            &account
                                .username
                                .chars()
                                .filter(|c| c.is_alphanumeric())
                                .take(8)
                                .collect::<String>()
                        )
                    } else {
                        clean
                    }
                } else {
                    let clean = account
                        .username
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .take(16)
                        .collect::<String>();
                    if clean.is_empty() {
                        "Player".to_string()
                    } else {
                        clean
                    }
                }
            }
            _ => {
                let clean = account
                    .username
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .take(16)
                    .collect::<String>();
                if clean.is_empty() {
                    "Player".to_string()
                } else {
                    clean
                }
            }
        }
    }
}
