//! Handler for `TokenPasswordMessage` — password prompt for the encrypted token store.

use iced::Task;
use ql_auth::{encrypted_store, TokenStorageMethod};

use crate::{
    state::{
        load_accounts, LauncherSettingsMessage, LauncherSettingsTab, MenuTokenPassword,
        TokenPasswordMessage, TokenStoreMessage, NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
    },
    Launcher, Message, State,
};

impl Launcher {
    pub fn update_token_password(&mut self, msg: TokenPasswordMessage) -> Task<Message> {
        match msg {
            TokenPasswordMessage::PasswordChanged(p) => {
                if let State::TokenPasswordPrompt(menu) = &mut self.state {
                    menu.password = p;
                    menu.error = None;
                }
            }
            TokenPasswordMessage::ConfirmPasswordChanged(p) => {
                if let State::TokenPasswordPrompt(menu) = &mut self.state {
                    if let Some(confirm) = &mut menu.confirm_password {
                        *confirm = p;
                    }
                    menu.error = None;
                }
            }
            TokenPasswordMessage::ToggleShowPassword(show) => {
                if let State::TokenPasswordPrompt(menu) = &mut self.state {
                    menu.show_password = show;
                }
            }
            TokenPasswordMessage::Submit => {
                if let State::TokenPasswordPrompt(menu) = &mut self.state {
                    menu.is_loading = true;
                    let password = menu.password.clone();
                    let confirm = menu.confirm_password.clone();

                    if let Some(confirm_pw) = confirm {
                        // "Create new store" flow
                        if password != confirm_pw {
                            menu.is_loading = false;
                            menu.error = Some("Passwords do not match.".to_owned());
                            return Task::none();
                        }
                        return Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    encrypted_store::initialize_new(&password)
                                        .map_err(|e| e.to_string())
                                })
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                            },
                            |res| Message::TokenPassword(TokenPasswordMessage::SubmitDone(res)),
                        );
                    } else {
                        // "Unlock existing store" flow
                        return Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    encrypted_store::unlock(&password).map_err(|e| e.to_string())
                                })
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                            },
                            |res| Message::TokenPassword(TokenPasswordMessage::SubmitDone(res)),
                        );
                    }
                }
            }
            TokenPasswordMessage::SubmitDone(result) => {
                if let State::TokenPasswordPrompt(menu) = &mut self.state {
                    menu.is_loading = false;
                    match result {
                        Ok(()) => {
                            let new_accounts =
                                crate::state::reload_encrypted_accounts(&mut self.config);
                            self.accounts.extend(new_accounts.0);
                            for entry in new_accounts.1 {
                                if !self.accounts_dropdown.contains(&entry) {
                                    self.accounts_dropdown.insert(0, entry);
                                }
                            }
                            if let Some(saved) = self.config.c_account_selected() {
                                if self.accounts.contains_key(saved) {
                                    self.account_selected = saved.to_owned();
                                }
                            }
                            return self.go_to_launch_screen::<&str>(None);
                        }
                        Err(err) => {
                            menu.error = Some(err);
                        }
                    }
                }
            }
            TokenPasswordMessage::Skip => {
                return self.go_to_launch_screen::<&str>(None);
            }
        }
        Task::none()
    }

    pub fn update_token_store(&mut self, msg: TokenStoreMessage) -> Task<Message> {
        match msg {
            TokenStoreMessage::TokenEnsureLoaded => {
                if self.config.c_token_storage() == TokenStorageMethod::EncryptedFile
                    && !encrypted_store::is_unlocked()
                    && encrypted_store::file_exists()
                {
                    self.state = State::TokenPasswordPrompt(MenuTokenPassword {
                        password: String::new(),
                        confirm_password: None,
                        show_password: false,
                        error: None,
                        is_loading: false,
                    });
                }
            }
            TokenStoreMessage::TokenStorageChanged(method) => {
                self.config.token_storage = Some(method);
                ql_auth::token_store::set_storage_method(method);
                // Reload the account list so the dropdown immediately reflects the new backend
                let (accounts, dropdown, selected) = load_accounts(&mut self.config);
                self.accounts = accounts;
                self.accounts_dropdown = dropdown;
                self.account_selected = selected;
            }
            TokenStoreMessage::SetupEncryptedStore => {
                self.state = State::TokenPasswordPrompt(MenuTokenPassword {
                    password: String::new(),
                    confirm_password: Some(String::new()),
                    show_password: false,
                    error: None,
                    is_loading: false,
                });
            }
            TokenStoreMessage::UnlockEncryptedStore => {
                self.state = State::TokenPasswordPrompt(MenuTokenPassword {
                    password: String::new(),
                    confirm_password: None,
                    show_password: false,
                    error: None,
                    is_loading: false,
                });
            }
            TokenStoreMessage::DeleteEncryptedStore => {
                self.state = State::ConfirmAction {
                    msg1: "delete the encrypted token store".to_owned(),
                    msg2: "All accounts using encrypted storage will be removed from the launcher."
                        .to_owned(),
                    yes: TokenStoreMessage::DeleteEncryptedStoreConfirm.into(),
                    no: LauncherSettingsMessage::ChangeTab(LauncherSettingsTab::Security).into(),
                };
            }
            TokenStoreMessage::DeleteEncryptedStoreConfirm => {
                if let Err(err) = encrypted_store::delete_store() {
                    self.set_error(format!("Could not delete encrypted store: {err}"));
                    return Task::none();
                }
                encrypted_store::lock();
                // Remove encrypted-file accounts from config
                if let Some(accounts) = &mut self.config.accounts {
                    accounts
                        .retain(|_, v| v.c_token_storage() != TokenStorageMethod::EncryptedFile);
                }
                // Remove them from the in-memory dropdown/map
                let config_accounts = self.config.accounts.clone();
                self.accounts
                    .retain(|k, _| config_accounts.as_ref().is_some_and(|a| a.contains_key(k)));
                self.accounts_dropdown.retain(|entry| {
                    self.accounts.contains_key(entry)
                        || entry == OFFLINE_ACCOUNT_NAME
                        || entry == NEW_ACCOUNT_NAME
                });
                // Switch back to keyring backend
                self.config.token_storage = Some(TokenStorageMethod::Keyring);
                ql_auth::token_store::set_storage_method(TokenStorageMethod::Keyring);
                self.go_to_launcher_settings();
                return Task::none();
            }
        }
        Task::none()
    }
}
