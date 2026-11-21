use std::collections::HashMap;

use ql_core::{IntoStringError, err};
use ql_instances::auth::{self, AccountData, AccountType};

use crate::config::{ConfigAccount, LauncherConfig};

pub struct AccountState {
    accounts: HashMap<String, AccountData>,
    dropdown: Vec<String>,
    pub selected: String,
}

impl AccountState {
    pub const OFFLINE_ACCOUNT_NAME: &str = "(Offline)";
    pub const NEW_ACCOUNT_NAME: &str = "+ Add Account";

    pub fn load_from_config(config: &mut LauncherConfig) -> AccountState {
        let mut accounts = HashMap::new();

        let mut dropdown = vec![
            Self::OFFLINE_ACCOUNT_NAME.to_owned(),
            Self::NEW_ACCOUNT_NAME.to_owned(),
        ];

        let mut accounts_to_remove = Vec::new();

        for (username, account) in config.accounts.iter_mut().flatten() {
            load_account(
                &mut accounts,
                &mut dropdown,
                &mut accounts_to_remove,
                username,
                account,
            );
        }

        if let Some(config_accounts) = &mut config.accounts {
            for rem in accounts_to_remove {
                config_accounts.remove(&rem);
            }
        }

        let selected = config.account_selected.clone().unwrap_or(
            dropdown
                .first()
                .cloned()
                .unwrap_or_else(|| Self::OFFLINE_ACCOUNT_NAME.to_owned()),
        );
        AccountState {
            accounts,
            dropdown,
            selected,
        }
    }

    pub fn empty() -> Self {
        Self {
            accounts: HashMap::new(),
            dropdown: vec![
                Self::OFFLINE_ACCOUNT_NAME.to_owned(),
                Self::NEW_ACCOUNT_NAME.to_owned(),
            ],
            selected: Self::OFFLINE_ACCOUNT_NAME.to_owned(),
        }
    }

    pub fn ui_dropdown(&self) -> &[String] {
        &self.dropdown
    }

    pub fn censors(&self) -> Vec<String> {
        let mut censors = Vec::new();
        for account in self.accounts.values() {
            if let Some(token) = &account.access_token {
                censors.push(token.clone());
            }
        }
        censors
    }

    pub fn has_selected_offline(&self) -> bool {
        self.selected == Self::OFFLINE_ACCOUNT_NAME
    }

    pub fn logged_in(&mut self, config: &mut LauncherConfig, data: AccountData) {
        if data.username == Self::OFFLINE_ACCOUNT_NAME || data.username == Self::NEW_ACCOUNT_NAME {
            return;
        }
        let username = data.get_username_modified();

        if self.dropdown.contains(&username) {
            // Account already logged in
            return;
        }
        self.dropdown.insert(0, username.clone());

        let config_accounts = config.accounts.get_or_insert_default();
        config_accounts.insert(username.clone(), ConfigAccount::from_account(&data));

        self.selected.clone_from(&username);
        self.accounts.insert(username.clone(), data);
    }

    pub fn logout_confirm(&mut self, config: &mut LauncherConfig) -> Result<(), String> {
        let username = self.selected.clone();
        let account_type = self
            .accounts
            .get(&username)
            .map_or(AccountType::Microsoft, |n| n.account_type);

        auth::logout(account_type.strip_name(&username), account_type)?;
        if let Some(accounts) = &mut config.accounts {
            accounts.remove(&username);
        }
        self.accounts.remove(&username);
        if let Some(idx) = self
            .dropdown
            .iter()
            .enumerate()
            .find_map(|(i, n)| (*n == username).then_some(i))
        {
            self.dropdown.remove(idx);
        }

        let selected = self
            .dropdown
            .first()
            .cloned()
            .unwrap_or_else(|| Self::OFFLINE_ACCOUNT_NAME.to_owned());
        self.selected = selected;

        Ok(())
    }

    pub fn get_selected_data(&self) -> Option<AccountData> {
        let account = &self.selected;
        if account == Self::NEW_ACCOUNT_NAME || account == Self::OFFLINE_ACCOUNT_NAME {
            None
        } else {
            self.accounts.get(account).cloned()
        }
    }

    pub fn insert(&mut self, data: AccountData) {
        self.accounts.insert(data.get_username_modified(), data);
    }

    pub fn get(&self, account: &str) -> Option<&AccountData> {
        self.accounts.get(account)
    }
}

fn load_account(
    accounts: &mut HashMap<String, AccountData>,
    accounts_dropdown: &mut Vec<String>,
    accounts_to_remove: &mut Vec<String>,
    username: &str,
    account: &mut crate::config::ConfigAccount,
) {
    let account_type = if username.ends_with(" (elyby)") {
        AccountType::ElyBy
    } else if username.ends_with(" (littleskin)") {
        AccountType::LittleSkin
    } else {
        account.account_type.unwrap_or_default()
    };

    let keyring_username = account.get_keyring_identifier(username);
    let refresh_token =
        ql_instances::auth::read_refresh_token(keyring_username, account_type).strerr();

    let keyring_username = account.get_keyring_identifier(username);

    match refresh_token {
        Ok(refresh_token) => {
            accounts_dropdown.insert(0, username.to_owned());
            accounts.insert(
                username.to_owned(),
                AccountData {
                    access_token: None,
                    uuid: account.uuid.clone(),
                    refresh_token,
                    needs_refresh: true,
                    account_type,

                    username: keyring_username.to_owned(),
                    nice_username: account
                        .username_nice
                        .clone()
                        .unwrap_or_else(|| username.to_owned()),
                },
            );
        }
        Err(err) => {
            err!(
                "Could not load account: {err}\nUsername: {keyring_username}, Account Type: {}",
                account_type.to_string()
            );
            accounts_to_remove.push(username.to_owned());
        }
    }
}
