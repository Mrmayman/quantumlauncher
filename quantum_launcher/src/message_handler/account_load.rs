use std::collections::HashMap;

use ql_core::IntoStringError;
use ql_instances::auth::AccountData;

use crate::{
    config::ConfigAccount,
    state::{NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME},
};

#[derive(Debug, Clone)]
pub struct AccountLoad {
    pub accounts: HashMap<String, AccountData>,
    pub accounts_dropdown: Vec<String>,
}

pub async fn load_all_accounts(config_accounts: HashMap<String, ConfigAccount>) -> AccountLoad {
    let mut accounts = HashMap::new();
    let mut accounts_dropdown = vec![OFFLINE_ACCOUNT_NAME.to_owned(), NEW_ACCOUNT_NAME.to_owned()];

    for (username, account) in &config_accounts {
        load_account(&mut accounts, &mut accounts_dropdown, username, account).await;
    }

    AccountLoad {
        accounts,
        accounts_dropdown,
    }
}

async fn load_account(
    accounts: &mut HashMap<String, AccountData>,
    accounts_dropdown: &mut Vec<String>,
    username: &str,
    account: &crate::config::ConfigAccount,
) {
    let account_type = account.get_account_type(username);
    let keyring_username = account.get_keyring_identifier(username).to_owned();
    let refresh_token =
        ql_instances::auth::read_refresh_token(keyring_username.clone(), account_type)
            .await
            .strerr();

    accounts_dropdown.insert(0, username.to_owned());
    accounts.insert(
        username.to_owned(),
        AccountData {
            access_token: None,
            uuid: account.uuid.clone(),
            refresh_token,
            needs_refresh: true,
            account_type,

            username: keyring_username.clone(),
            nice_username: account
                .username_nice
                .clone()
                .unwrap_or(keyring_username.clone()),
        },
    );
}
