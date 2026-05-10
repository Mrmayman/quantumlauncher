use std::collections::HashMap;

use ql_instances::auth::{AccountData, AccountType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigAccount {
    /// UUID of the Minecraft account. Stored as a string without dashes.
    ///
    /// Example: `2553495fc9094d40a82646cfc92cd7a5`
    ///
    /// A UUID is like an alternate username that can be used to identify
    /// an account. Unlike a username it can't be changed, so it's useful for
    /// dealing with account data in a stable manner.
    ///
    /// You can find someone's UUID through many online services where you
    /// input their username.
    pub uuid: String,

    /// Currently unimplemented, does nothing.
    pub skin: Option<String>, // TODO: Add skin visualization?

    /// Type of account (default: `Microsoft`)
    pub account_type: Option<AccountType>,

    /// The original login identifier used for keyring operations.
    /// This is the email address or username that was used during login.
    /// For email/password logins, this will be the email.
    /// For username/password logins, this will be the username.
    pub keyring_identifier: Option<String>,

    /// A game-readable "nice" username.
    ///
    /// This will be identical to the regular
    /// username of the account in most cases
    /// except for the case where the user
    /// has an `ely.by` account with an email.
    /// In that case, this will be the actual
    /// username while the regular "username"
    /// would be an email.
    pub username_nice: Option<String>,

    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl ConfigAccount {
    pub fn get_account_type(&self, display_username: &str) -> AccountType {
        match self.account_type {
            Some(AccountType::Microsoft) | None => {
                if display_username.ends_with(" (elyby)") {
                    AccountType::ElyBy
                } else if display_username.ends_with(" (littleskin)") {
                    AccountType::LittleSkin
                } else {
                    AccountType::Microsoft
                }
            }
            Some(a @ (AccountType::LittleSkin | AccountType::ElyBy)) => a,
        }
    }

    pub fn from_account(data: &AccountData) -> Self {
        Self {
            uuid: data.uuid.clone(),
            skin: None,
            account_type: Some(data.account_type),
            keyring_identifier: Some(data.username.clone()),
            username_nice: Some(data.nice_username.clone()),
            _extra: HashMap::new(),
        }
    }

    pub fn get_keyring_identifier<'a>(&'a self, display_username: &'a str) -> &'a str {
        if let Some(keyring_id) = self.keyring_identifier.as_deref() {
            keyring_id
        } else {
            // Fallback to old behavior for backwards compatibility
            match self.get_account_type(display_username) {
                AccountType::ElyBy => display_username
                    .strip_suffix(" (elyby)")
                    .unwrap_or(display_username),
                AccountType::LittleSkin => display_username
                    .strip_suffix(" (littleskin)")
                    .unwrap_or(display_username),
                AccountType::Microsoft => display_username,
            }
        }
    }
}
