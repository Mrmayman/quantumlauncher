use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use auth::AccountData;
use iced::Task;
use ql_core::IntoStringError;
use ql_instances::auth::{self, AccountType};

use crate::{
    config::ConfigAccount,
    sip,
    state::{
        AccountMessage, AutoSaveKind, Launcher, LittleSkinOauth, MenuLoginAlternate, MenuLoginMS,
        Message, ProgressBar, State, NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
    },
};

impl Launcher {
    pub fn update_account(&mut self, msg: AccountMessage) -> Task<Message> {
        match msg {
            AccountMessage::AltLoginResponse(Err(err)) if err.contains("incorrect password") => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.is_incorrect_password = true;
                    menu.is_loading = false;
                }
            }

            AccountMessage::Response1 { r: Err(err), .. }
            | AccountMessage::Response2(Err(err))
            | AccountMessage::Response3(Err(err))
            | AccountMessage::AltLoginResponse(Err(err))
            | AccountMessage::RefreshComplete(Err(err)) => {
                self.set_error(err);
            }
            AccountMessage::Selected(account) => {
                return self.account_selected(account);
            }
            AccountMessage::Response1 {
                r: Ok(code),
                is_from_welcome_screen,
            } => {
                return self.account_response_1(code, is_from_welcome_screen);
            }
            AccountMessage::Response2(Ok(token)) => {
                return self.account_response_2(token);
            }
            AccountMessage::Response3(Ok(data)) => {
                return self.account_response_3(data);
            }
            AccountMessage::LogoutCheck => {
                let username = self.accounts_selected.as_ref().unwrap();
                self.state = State::ConfirmAction {
                    msg1: format!("log out of your account: {username}"),
                    msg2: "You can always log in later".to_owned(),
                    yes: Message::Account(AccountMessage::LogoutConfirm),
                    no: Message::MScreenOpen {
                        message: None,
                        clear_selection: false,
                        is_server: None,
                    },
                }
            }
            AccountMessage::LittleSkinDeviceCodeReady {
                user_code,
                verification_uri,
                expires_in,
                interval,
                device_code,
            } => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.oauth = Some(LittleSkinOauth {
                        // device_code: device_code.clone(),
                        user_code: user_code.clone(),
                        verification_uri: verification_uri.clone(),
                        device_code_expires_at: Instant::now() + Duration::from_secs(expires_in),
                    });
                    menu.is_loading = false;
                }

                // Start polling for token
                let device_code_clone = device_code.clone();
                return Task::perform(
                    auth::yggdrasil::oauth::poll_device_token(
                        device_code_clone,
                        interval,
                        expires_in,
                    ),
                    |resp| match resp {
                        Ok(account) => {
                            Message::Account(AccountMessage::AltLoginResponse(Ok(account)))
                        }
                        Err(e) => Message::Account(AccountMessage::LittleSkinDeviceCodeError(
                            e.to_string(),
                        )),
                    },
                );
            }
            AccountMessage::LittleSkinDeviceCodeError(err_msg) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.is_loading = false;
                    menu.device_code_error = Some(err_msg);
                }
            }
            AccountMessage::LogoutConfirm => {
                self.autosave.remove(&AutoSaveKind::LauncherConfig);
                let username = self.accounts_selected.clone().unwrap();
                let account_type = self
                    .accounts
                    .get(&username)
                    .map_or(AccountType::Microsoft, |n| n.account_type);

                if let Err(err) = auth::logout(account_type.strip_name(&username), account_type) {
                    self.set_error(err);
                }
                if let Some(accounts) = &mut self.config.accounts {
                    accounts.remove(&username);
                }
                self.accounts.remove(&username);
                if let Some(idx) = self
                    .accounts_dropdown
                    .iter()
                    .enumerate()
                    .find_map(|(i, n)| (*n == username).then_some(i))
                {
                    self.accounts_dropdown.remove(idx);
                }
                let selected_account = self
                    .accounts_dropdown
                    .first()
                    .cloned()
                    .unwrap_or_else(|| OFFLINE_ACCOUNT_NAME.to_owned());
                self.accounts_selected = Some(selected_account);

                return self.go_to_launch_screen(Option::<String>::None);
            }
            AccountMessage::RefreshComplete(Ok(data)) => {
                self.accounts.insert(data.get_username_modified(), data);

                let account_data = self.get_selected_account_data();

                return Task::batch([
                    self.go_to_launch_screen::<String>(None),
                    self.launch_game(account_data),
                ]);
            }

            AccountMessage::OpenMenu {
                is_from_welcome_screen,
                kind,
            } => match kind {
                AccountType::Microsoft => {
                    self.state = State::GenericMessage("Loading Login...".to_owned());
                    return Task::perform(auth::ms::login_1_link(), move |n| {
                        Message::Account(AccountMessage::Response1 {
                            r: n.strerr(),
                            is_from_welcome_screen,
                        })
                    });
                }
                AccountType::ElyBy | AccountType::LittleSkin => {
                    self.state = State::LoginAlternate(MenuLoginAlternate {
                        username: String::new(),
                        password: String::new(),
                        is_loading: false,
                        otp: None,
                        show_password: false,
                        is_from_welcome_screen,
                        is_incorrect_password: false,

                        is_littleskin: matches!(kind, AccountType::LittleSkin),
                        device_code_error: None,
                        oauth: None,
                    });
                }
            },

            AccountMessage::AltUsernameInput(username) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.username = username;
                }
            }
            AccountMessage::AltPasswordInput(password) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.password = password;
                }
            }
            AccountMessage::AltOtpInput(otp) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.otp = Some(otp);
                }
            }
            AccountMessage::AltShowPassword(t) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.show_password = t;
                }
            }

            AccountMessage::AltLogin => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.is_incorrect_password = false;
                    let mut password = menu.password.clone();
                    if let Some(otp) = &menu.otp {
                        password.push(':');
                        password.push_str(otp);
                    }
                    menu.is_loading = true;

                    return Task::perform(
                        auth::yggdrasil::login_new(
                            menu.username.clone(),
                            password,
                            if menu.is_littleskin {
                                AccountType::LittleSkin
                            } else {
                                AccountType::ElyBy
                            },
                        ),
                        |n| Message::Account(AccountMessage::AltLoginResponse(n.strerr())),
                    );
                }
            }
            AccountMessage::AltLoginResponse(Ok(acc)) => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.is_loading = false;
                    match acc {
                        auth::yggdrasil::Account::Account(data) => {
                            return self.account_response_3(data);
                        }
                        auth::yggdrasil::Account::NeedsOTP => {
                            menu.otp = Some(String::new());
                        }
                    }
                }
            }

            AccountMessage::LittleSkinOauthButtonClicked => {
                if let State::LoginAlternate(menu) = &mut self.state {
                    menu.is_loading = true;
                    menu.is_incorrect_password = false;
                }

                return Task::perform(auth::yggdrasil::oauth::request_device_code(), |resp| {
                    Message::Account(match resp {
                        Ok(code) => AccountMessage::LittleSkinDeviceCodeReady {
                            user_code: code.user_code,
                            verification_uri: code.verification_uri,
                            expires_in: code.expires_in,
                            interval: code.interval,
                            device_code: code.device_code,
                        },
                        Err(e) => AccountMessage::LittleSkinDeviceCodeError(e.to_string()),
                    })
                });
            }
        }
        Task::none()
    }

    fn account_selected(&mut self, account: String) -> Task<Message> {
        if account == NEW_ACCOUNT_NAME {
            self.state = State::AccountLogin;
        } else {
            if account != OFFLINE_ACCOUNT_NAME {
                self.config.account_selected = Some(account.clone());
            }
            self.accounts_selected = Some(account);
        }
        Task::none()
    }

    pub fn account_refresh(&mut self, account: &AccountData) -> Task<Message> {
        match account.account_type {
            AccountType::Microsoft => {
                self.state = State::AccountLoginProgress(ProgressBar::new());
                let username = account.username.clone();
                let refresh_token = account.refresh_token.clone();
                sip(
                    |sender| auth::ms::login_refresh(username, refresh_token, Some(sender)),
                    |n| Message::Account(AccountMessage::RefreshComplete(n.strerr())),
                )
            }
            AccountType::ElyBy | AccountType::LittleSkin => Task::perform(
                auth::yggdrasil::login_refresh(
                    account.username.clone(),
                    account.refresh_token.clone(),
                    account.account_type,
                ),
                |n| Message::Account(AccountMessage::RefreshComplete(n.strerr())),
            ),
        }
    }

    fn account_response_3(&mut self, data: AccountData) -> Task<Message> {
        self.autosave.remove(&AutoSaveKind::LauncherConfig);
        if data.username == OFFLINE_ACCOUNT_NAME || data.username == NEW_ACCOUNT_NAME {
            return self.go_to_launch_screen::<String>(None);
        }
        let username = data.get_username_modified();

        if self.accounts_dropdown.contains(&username) {
            // Account already logged in
            return self.go_to_launch_screen::<String>(None);
        }
        self.accounts_dropdown.insert(0, username.clone());

        let config_accounts = self.config.accounts.get_or_insert_with(HashMap::new);
        config_accounts.insert(
            username.clone(),
            ConfigAccount {
                uuid: data.uuid.clone(),
                skin: None,
                account_type: Some(data.account_type.to_string()),
                keyring_identifier: Some(data.username.clone()),
                username_nice: Some(data.nice_username.clone()),
            },
        );

        self.accounts_selected = Some(username.clone());
        self.accounts.insert(username.clone(), data);

        self.go_to_launch_screen::<String>(None)
    }

    fn account_response_2(&mut self, token: auth::ms::AuthTokenResponse) -> Task<Message> {
        self.state = State::AccountLoginProgress(ProgressBar::new());
        sip(
            |sender| auth::ms::login_3_xbox(token, Some(sender), true),
            |n| Message::Account(AccountMessage::Response3(n.strerr())),
        )
    }

    fn account_response_1(
        &mut self,
        code: auth::ms::AuthCodeResponse,
        is_from_welcome_screen: bool,
    ) -> Task<Message> {
        let (task, handle) = Task::perform(auth::ms::login_2_wait(code.clone()), |n| {
            Message::Account(AccountMessage::Response2(n.strerr()))
        })
        .abortable();

        self.state = State::LoginMS(MenuLoginMS {
            url: code.verification_uri,
            code: code.user_code,
            is_from_welcome_screen,
            _cancel_handle: handle.abort_on_drop(),
        });

        task
    }

    pub fn get_selected_account_data(&self) -> Option<AccountData> {
        if let Some(account) = &self.accounts_selected {
            if account == NEW_ACCOUNT_NAME || account == OFFLINE_ACCOUNT_NAME {
                None
            } else {
                self.accounts.get(account).cloned()
            }
        } else {
            None
        }
    }
}
