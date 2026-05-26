use ql_core::{IntoStringError, err};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

mod alt;
pub mod authlib;
pub mod ms;
pub mod yggdrasil;
pub use authlib::get_authlib_injector;

#[derive(Debug, Clone)]
pub struct AccountData {
    pub access_token: Option<String>,
    pub uuid: String,
    pub refresh_token: String,
    pub needs_refresh: bool,

    pub username: String,
    pub nice_username: String,

    pub account_type: AccountType,
}

impl AccountData {
    #[must_use]
    pub fn get_username_modified(&self) -> String {
        self.account_type.add_suffix_to_name(&self.nice_username)
    }

    #[must_use]
    pub fn get_authlib_url(&self) -> Option<&'static str> {
        match self.account_type {
            AccountType::Microsoft => None,
            AccountType::ElyBy => Some("ely.by"),
            AccountType::LittleSkin => Some("https://littleskin.cn/api/yggdrasil"),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub enum AccountType {
    ElyBy,
    LittleSkin,
    #[serde(other)]
    #[default]
    Microsoft,
}

impl Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AccountType::Microsoft => "Microsoft",
            AccountType::ElyBy => "ElyBy",
            AccountType::LittleSkin => "LittleSkin",
        })
    }
}

impl AccountType {
    #[must_use]
    fn yggdrasil_authenticate(self) -> &'static str {
        match self {
            AccountType::Microsoft => unreachable!(),
            AccountType::ElyBy => "https://authserver.ely.by/auth/authenticate",
            AccountType::LittleSkin => {
                "https://littleskin.cn/api/yggdrasil/authserver/authenticate"
            }
        }
    }

    #[must_use]
    fn yggdrasil_refresh(self) -> &'static str {
        match self {
            AccountType::Microsoft => unreachable!(),
            AccountType::ElyBy => "https://authserver.ely.by/auth/refresh",
            AccountType::LittleSkin => "https://littleskin.cn/api/yggdrasil/authserver/refresh",
        }
    }

    #[must_use]
    fn yggdrasil_needs_agent_field(self) -> bool {
        match self {
            AccountType::Microsoft | AccountType::ElyBy => false,
            AccountType::LittleSkin => true,
        }
    }

    fn get_keyring_entry(self, username: &str) -> Result<keyring::Entry, KeyringError> {
        Ok(keyring::Entry::new(
            "QuantumLauncher",
            &format!(
                "{username}{}",
                match self {
                    AccountType::Microsoft => "",
                    AccountType::ElyBy => "#elyby",
                    AccountType::LittleSkin => "#littleskin",
                }
            ),
        )?)
    }

    #[must_use]
    pub(crate) fn get_client_id(self) -> &'static str {
        match self {
            AccountType::Microsoft => ms::CLIENT_ID,
            AccountType::ElyBy => "quantumlauncher1",
            AccountType::LittleSkin => "1160",
        }
    }

    #[must_use]
    pub fn strip_name(self, name: &str) -> &str {
        match self {
            AccountType::Microsoft => name,
            AccountType::ElyBy => name.strip_suffix(" (elyby)").unwrap_or(name),
            AccountType::LittleSkin => name.strip_suffix(" (littleskin)").unwrap_or(name),
        }
    }

    #[must_use]
    pub fn add_suffix_to_name(self, name: &str) -> String {
        let suffix = match self {
            AccountType::Microsoft => "",
            AccountType::ElyBy => " (elyby)",
            AccountType::LittleSkin => " (littleskin)",
        };
        format!("{name}{suffix}")
    }
}

impl AccountData {
    #[must_use]
    pub fn is_microsoft(&self) -> bool {
        matches!(self.account_type, AccountType::Microsoft)
    }
}

#[derive(Debug, thiserror::Error)]
pub struct KeyringError(#[from] pub keyring::Error);

impl Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Account keyring error:")?;
        match &self.0 {
            #[cfg(target_os = "linux")]
            keyring::Error::PlatformFailure(error)
                if error.to_string().contains("The name is not activatable") =>
            {
                write!(
                    f,
                    "{error}\n\nTry installing gnome-keyring and libsecret packages\n(may be called differently depending on your distro)"
                )
            }
            #[cfg(target_os = "linux")]
            keyring::Error::NoStorageAccess(error)
                if error.to_string().contains("no result found") =>
            {
                write!(
                    f,
                    r#"{error}

Install the "seahorse" app and open it,
Check for "Login" in the sidebar.
If it's there, make sure it's unlocked (right-click -> Unlock)

If it's not there, click on + then "Password Keyring",
and name it "Login" and put your preferred password

Now after this, in the sidebar, right click it and click "Set as Default""#
                )
            }

            _ => write!(f, "{}", self.0),
        }
    }
}

pub fn read_refresh_token(
    username: &str,
    account_type: AccountType,
) -> Result<String, KeyringError> {
    let entry = account_type.get_keyring_entry(username)?;
    let refresh_token = entry.get_password()?;
    Ok(refresh_token)
}

pub fn logout(username: &str, account_type: AccountType) -> Result<(), String> {
    let entry = account_type.get_keyring_entry(username).strerr()?;
    if let Err(err) = entry.delete_credential() {
        err!("Couldn't remove {account_type} account credential (Username: {username}):\n{err}");
    }
    Ok(())
}
