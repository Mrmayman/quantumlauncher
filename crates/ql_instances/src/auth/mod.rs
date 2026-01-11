use ql_core::{err, IntoStringError};
use std::fmt::Display;
use tokio::task::spawn_blocking;

mod alt;
pub mod authlib;
pub mod ms;
pub mod yggdrasil;
pub use authlib::get_authlib_injector;

#[derive(Debug, Clone)]
pub struct AccountData {
    pub access_token: Option<String>,
    pub uuid: String,
    pub refresh_token: Result<String, String>,
    pub needs_refresh: bool,

    pub username: String,
    pub nice_username: String,

    pub account_type: AccountType,
}

impl AccountData {
    #[must_use]
    pub fn get_username_modified(&self) -> String {
        let suffix = match self.account_type {
            AccountType::Microsoft => "",
            AccountType::ElyBy => " (elyby)",
            AccountType::LittleSkin => " (littleskin)",
        };
        format!("{}{suffix}", self.nice_username)
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

#[derive(Debug, Clone, Copy)]
pub enum AccountType {
    Microsoft,
    ElyBy,
    LittleSkin,
}

impl Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AccountType::Microsoft => "Microsoft",
                AccountType::ElyBy => "ElyBy",
                AccountType::LittleSkin => "LittleSkin",
            }
        )
    }
}

impl AccountType {
    #[must_use]
    pub fn yggdrasil_authenticate(self) -> &'static str {
        match self {
            AccountType::Microsoft => unreachable!(),
            AccountType::ElyBy => "https://authserver.ely.by/auth/authenticate",
            AccountType::LittleSkin => {
                "https://littleskin.cn/api/yggdrasil/authserver/authenticate"
            }
        }
    }

    #[must_use]
    pub fn yggdrasil_refresh(self) -> &'static str {
        match self {
            AccountType::Microsoft => unreachable!(),
            AccountType::ElyBy => "https://authserver.ely.by/auth/refresh",
            AccountType::LittleSkin => "https://littleskin.cn/api/yggdrasil/authserver/refresh",
        }
    }

    #[must_use]
    pub fn yggdrasil_needs_agent_field(self) -> bool {
        match self {
            AccountType::Microsoft | AccountType::ElyBy => false,
            AccountType::LittleSkin => true,
        }
    }

    async fn get_keyring_entry(self, username: &str) -> Result<keyring::Entry, alt::Error> {
        let username = username.to_owned();
        Ok(tokio::task::spawn_blocking(move || {
            keyring::Entry::new(
                "QuantumLauncher",
                &format!(
                    "{username}{}",
                    match self {
                        AccountType::Microsoft => "",
                        AccountType::ElyBy => "#elyby",
                        AccountType::LittleSkin => "#littleskin",
                    }
                ),
            )
        })
        .await??)
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
}

impl AccountData {
    #[must_use]
    pub fn is_elyby(&self) -> bool {
        matches!(self.account_type, AccountType::ElyBy)
    }
    #[must_use]
    pub fn is_littleskin(&self) -> bool {
        matches!(self.account_type, AccountType::LittleSkin)
    }
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
                write!(f, "{error}\n\nTry installing gnome-keyring and libsecret packages\n(may be called differently depending on your distro)")
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

pub async fn read_refresh_token(
    username: String,
    account_type: AccountType,
) -> Result<String, String> {
    let entry = account_type.get_keyring_entry(&username).await.strerr()?;
    let refresh_token = spawn_blocking(move || entry.get_password())
        .await
        .strerr()?
        .strerr()?;
    Ok(refresh_token)
}

pub async fn logout(username: String, account_type: AccountType) -> Result<(), String> {
    let entry = account_type.get_keyring_entry(&username).await.strerr()?;
    if let Err(err) = spawn_blocking(move || entry.delete_credential())
        .await
        .strerr()?
    {
        err!("Couldn't remove {account_type} account credential (Username: {username}):\n{err}");
    }
    Ok(())
}
