// Account management and authentication helpers

use crate::config::{ConfigAccount, LauncherConfig};
use crate::tui::app::App;

impl App {
    // Load accounts from config on startup
    pub fn load_accounts(&mut self) {
        match LauncherConfig::load_s() {
            Ok(config) => {
                if let Some(config_accounts) = config.accounts {
                    self.accounts = config_accounts
                        .iter()
                        .map(|(username_key, config_account)| {
                            let account_type = config_account
                                .account_type
                                .clone()
                                .unwrap_or_else(|| "Microsoft".to_string());

                            let is_logged_in = if account_type == "Offline" {
                                true
                            } else {
                                let keyring_username =
                                    if let Some(keyring_id) = &config_account.keyring_identifier {
                                        keyring_id.clone()
                                    } else {
                                        match account_type.as_str() {
                                            "ElyBy" => username_key
                                                .strip_suffix(" (elyby)")
                                                .unwrap_or(username_key)
                                                .to_string(),
                                            "LittleSkin" => username_key
                                                .strip_suffix(" (littleskin)")
                                                .unwrap_or(username_key)
                                                .to_string(),
                                            _ => username_key.clone(),
                                        }
                                    };

                                let auth_account_type = match account_type.as_str() {
                                    "ElyBy" => ql_instances::auth::AccountType::ElyBy,
                                    "LittleSkin" => ql_instances::auth::AccountType::LittleSkin,
                                    _ => ql_instances::auth::AccountType::Microsoft,
                                };
                                ql_instances::auth::read_refresh_token(
                                    &keyring_username,
                                    auth_account_type,
                                )
                                .is_ok()
                            };

                            super::state::Account {
                                username: username_key.clone(),
                                account_type,
                                uuid: config_account.uuid.clone(),
                                is_logged_in,
                            }
                        })
                        .collect();
                    if let Some(selected_account) = config.account_selected {
                        self.current_account = Some(selected_account);
                    }
                    if self.accounts.is_empty() {
                        self.status_message += " (No accounts configured)";
                    } else {
                        self.status_message = format!(
                            "Loaded {} instances and {} accounts",
                            self.instances.len(),
                            self.accounts.len()
                        );
                    }
                } else {
                    self.status_message += " (No accounts configured)";
                }
            }
            Err(err) => {
                self.status_message = format!("Error loading config: {}", err);
            }
        }
    }

    pub fn get_selected_account(&self) -> Option<&super::state::Account> {
        self.accounts.get(self.selected_account)
    }

    pub fn toggle_add_account_mode(&mut self) {
        self.is_add_account_mode = !self.is_add_account_mode;
        if !self.is_add_account_mode {
            self.new_account_username.clear();
            self.new_account_password.clear();
            self.selected_account_type = 0;
            self.new_account_type = super::state::AccountType::ElyBy;
            self.new_account_otp = None;
            self.needs_otp = false;
            self.login_error = None;
            self.show_password = false;
            self.add_account_field_focus = super::state::AddAccountFieldFocus::Username;
        }
    }

    pub fn next_account_type(&mut self) {
        let account_types = super::state::AccountType::all();
        if !account_types.is_empty() {
            self.selected_account_type = (self.selected_account_type + 1) % account_types.len();
            if self.selected_account_type < account_types.len() {
                self.new_account_type = account_types[self.selected_account_type].clone();
            }
        }
    }

    pub fn prev_account_type(&mut self) {
        let account_types = super::state::AccountType::all();
        if !account_types.is_empty() && self.selected_account_type > 0 {
            self.selected_account_type -= 1;
        } else if !account_types.is_empty() {
            self.selected_account_type = account_types.len() - 1;
        }
        if self.selected_account_type < account_types.len() {
            self.new_account_type = account_types[self.selected_account_type].clone();
        }
    }

    pub fn next_add_account_field(&mut self) {
        use super::state::AccountType as T;
        use super::state::AddAccountFieldFocus as F;
        if self.new_account_type != T::ElyBy && self.new_account_type != T::LittleSkin {
            return;
        }
        self.add_account_field_focus = match self.add_account_field_focus {
            F::Username => F::Password,
            F::Password => {
                if self.needs_otp {
                    F::Otp
                } else {
                    F::Username
                }
            }
            F::Otp => F::Username,
        };
        let field_name = match self.add_account_field_focus {
            F::Username => "username/email",
            F::Password => "password",
            F::Otp => "OTP code",
        };
        self.status_message = format!("Now editing: {}", field_name);
    }

    pub fn add_char_to_add_account_field(&mut self, c: char) {
        use super::state::AccountType as T;
        use super::state::AddAccountFieldFocus as F;
        if self.new_account_type != T::ElyBy && self.new_account_type != T::LittleSkin {
            self.new_account_username.push(c);
            return;
        }
        match self.add_account_field_focus {
            F::Username => self.new_account_username.push(c),
            F::Password => self.new_account_password.push(c),
            F::Otp => {
                if let Some(ref mut otp) = self.new_account_otp {
                    otp.push(c);
                }
            }
        }
    }

    pub fn remove_char_from_add_account_field(&mut self) {
        use super::state::AccountType as T;
        use super::state::AddAccountFieldFocus as F;
        if self.new_account_type != T::ElyBy && self.new_account_type != T::LittleSkin {
            self.new_account_username.pop();
            return;
        }
        match self.add_account_field_focus {
            F::Username => {
                self.new_account_username.pop();
            }
            F::Password => {
                self.new_account_password.pop();
            }
            F::Otp => {
                if let Some(ref mut otp) = self.new_account_otp {
                    otp.pop();
                }
            }
        }
    }

    pub fn toggle_password_visibility(&mut self) {
        self.show_password = !self.show_password;
    }

    pub fn add_new_account(&mut self) {
        use super::state::AccountType as T;
        if self.new_account_username.is_empty() {
            self.status_message = "Please enter a username".to_string();
            return;
        }
        match self.new_account_type {
            T::Microsoft => {
                self.status_message =
                    "Microsoft accounts are not implemented yet - coming soon!".to_string();
                self.login_error = Some("Microsoft authentication not yet available".to_string());
            }
            T::LittleSkin => {
                self.status_message =
                    "LittleSkin accounts are not implemented yet - coming soon!".to_string();
                self.login_error = Some("LittleSkin authentication not yet available".to_string());
            }
            T::Offline => {
                let new_account = super::state::Account {
                    username: self.new_account_username.clone(),
                    account_type: self.new_account_type.to_string(),
                    uuid: "00000000-0000-0000-0000-000000000000".to_string(),
                    is_logged_in: true,
                };
                if let Err(err) = self.save_offline_account(&new_account) {
                    self.status_message =
                        format!("Warning: Failed to save offline account to config: {}", err);
                } else {
                    self.status_message = format!(
                        "Added offline account: {} (ready for launching)",
                        self.new_account_username
                    );
                }
                self.accounts.push(new_account);
                self.toggle_add_account_mode();
            }
            T::ElyBy => {
                if self.new_account_password.is_empty() {
                    self.login_error = Some("Password is required for ElyBy accounts".to_string());
                    return;
                }
                self.start_elyby_authentication();
            }
        }
    }

    /// Start ElyBy authentication process
    pub fn start_elyby_authentication(&mut self) {
        if let Some(sender) = &self.auth_sender {
            // Clear any previous errors
            self.login_error = None;

            // Prepare password with OTP if needed
            let mut password = self.new_account_password.clone();
            if let Some(ref otp) = self.new_account_otp {
                if !otp.is_empty() {
                    password.push(':');
                    password.push_str(otp);
                }
            }

            let username = self.new_account_username.clone();
            let sender_clone = sender.clone();

            // Send login started event
            let _ = sender.send(crate::tui::AuthEvent::LoginStarted);

            // Spawn authentication task
            tokio::spawn(async move {
                match ql_instances::auth::yggdrasil::login_new(
                    username,
                    password,
                    ql_instances::auth::AccountType::ElyBy,
                )
                .await
                {
                    Ok(ql_instances::auth::yggdrasil::Account::Account(account_data)) => {
                        let _ =
                            sender_clone.send(crate::tui::AuthEvent::LoginSuccess { account_data });
                    }
                    Ok(ql_instances::auth::yggdrasil::Account::NeedsOTP) => {
                        let _ = sender_clone.send(crate::tui::AuthEvent::LoginNeedsOtp);
                    }
                    Err(err) => {
                        let _ = sender_clone.send(crate::tui::AuthEvent::LoginError {
                            error: err.to_string(),
                        });
                    }
                }
            });
        }
    }

    pub fn logout_account(&mut self) {
        if self.selected_account >= self.accounts.len() {
            self.status_message = "No account selected to logout".to_string();
            return;
        }
        let account = &self.accounts[self.selected_account];
        if account.account_type == "Offline" {
            self.status_message =
                "Cannot logout offline accounts - they are always ready to use".to_string();
            return;
        }
        let username = account.username.clone();
        let account_type_str = account.account_type.clone();
        let account_type = match account_type_str.as_str() {
            "ElyBy" => ql_instances::auth::AccountType::ElyBy,
            "LittleSkin" => ql_instances::auth::AccountType::LittleSkin,
            _ => ql_instances::auth::AccountType::Microsoft,
        };
        let keyring_username = match account_type {
            ql_instances::auth::AccountType::ElyBy => username
                .strip_suffix(" (elyby)")
                .unwrap_or(&username)
                .to_string(),
            ql_instances::auth::AccountType::LittleSkin => username
                .strip_suffix(" (littleskin)")
                .unwrap_or(&username)
                .to_string(),
            ql_instances::auth::AccountType::Microsoft => username.clone(),
        };
        if let Err(err) = ql_instances::auth::logout(&keyring_username, account_type) {
            self.status_message =
                format!("Failed to logout keyring for {}: {}", keyring_username, err);
        } else {
            self.status_message = format!("Successfully logged out: {}", account.username);
        }
        if let Err(err) = self.remove_account_from_config(&username) {
            self.status_message = format!("Warning: Failed to remove account from config: {}", err);
        }
        self.accounts.remove(self.selected_account);
        if let Some(ref current) = self.current_account {
            if current == &username {
                self.current_account = None;
            }
        }
        if self.selected_account >= self.accounts.len() && !self.accounts.is_empty() {
            self.selected_account = self.accounts.len() - 1;
        }
        if self.accounts.is_empty() {
            self.selected_account = 0;
        }
    }

    pub fn set_default_account(&mut self) {
        if self.accounts.is_empty() {
            self.status_message = "No accounts available to set as default.".to_string();
            return;
        }
        let selected_account = &self.accounts[self.selected_account];
        let default_key = selected_account.username.clone();
        self.current_account = Some(default_key.clone());
        if let Err(err) = self.save_default_account_to_config(&default_key) {
            self.status_message = format!("❌ Failed to save default account: {}", err);
        } else {
            self.status_message =
                format!("✅ Set {} as default account", selected_account.username);
        }
    }

    fn save_default_account_to_config(&self, default_key: &str) -> Result<(), String> {
        let mut config = LauncherConfig::load_s().unwrap_or_default();
        config.account_selected = Some(default_key.to_string());
        self.save_config_sync(&config)
    }

    pub(crate) fn save_authenticated_account(
        &mut self,
        account_data: &ql_instances::auth::AccountData,
    ) -> Result<(), String> {
        let mut config = LauncherConfig::load_s().unwrap_or_default();
        let accounts = config.accounts.get_or_insert_with(Default::default);
        let username_modified = account_data.get_username_modified();
        accounts.insert(
            username_modified.clone(),
            ConfigAccount {
                uuid: account_data.uuid.clone(),
                skin: None,
                account_type: Some(account_data.account_type.to_string()),
                keyring_identifier: Some(account_data.username.clone()),
                username_nice: Some(account_data.nice_username.clone()),
            },
        );
        config.account_selected = Some(username_modified);
        self.save_config_sync(&config)?;
        Ok(())
    }

    fn remove_account_from_config(&self, username: &str) -> Result<(), String> {
        let mut config = LauncherConfig::load_s().unwrap_or_default();
        if let Some(accounts) = &mut config.accounts {
            accounts.remove(username);
        }
        if config.account_selected.as_deref() == Some(username) {
            config.account_selected = None;
        }
        self.save_config_sync(&config)
    }

    fn save_offline_account(&self, account: &super::state::Account) -> Result<(), String> {
        let mut config = LauncherConfig::load_s().unwrap_or_default();
        let accounts = config.accounts.get_or_insert_with(Default::default);
        let username = account.username.clone();
        accounts.insert(
            username.clone(),
            ConfigAccount {
                uuid: account.uuid.clone(),
                skin: None,
                account_type: Some("Offline".to_string()),
                keyring_identifier: None,
                username_nice: Some(username.clone()),
            },
        );
        if self.accounts.is_empty() {
            config.account_selected = Some(username);
        }
        self.save_config_sync(&config)
            .map_err(|e| format!("Failed to save config: {}", e))
    }
}
