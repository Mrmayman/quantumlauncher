use owo_colors::OwoColorize;
use std::{
    io::{self, IsTerminal},
    process::exit,
};

use ql_auth::{AccountType, TokenStorageMethod};
use ql_core::err;

use crate::{
    cli::show_notification,
    config::{ConfigAccount, LauncherConfig},
};

pub async fn refresh_account(
    username: &String,
    use_account: bool,
    show_progress: bool,
    override_account_type: Option<&str>,
) -> Result<Option<ql_auth::AccountData>, Box<dyn std::error::Error>> {
    if !use_account {
        if show_progress {
            tokio::task::spawn_blocking(|| {
                show_notification("Launching game", "Enjoy!");
            });
        }
        return Ok(None);
    }

    let config = LauncherConfig::load_s()?;
    let Some(account) = get_account(&config, username, override_account_type) else {
        err!("No logged-in account called {username:?} was found!");
        exit(1);
    };
    let refresh_name = account.get_keyring_identifier(username);
    let method = account.c_token_storage();

    ql_auth::token_store::set_storage_method(method);
    unlock_encrypted_store_if_needed(method)?;

    if show_progress {
        tokio::task::spawn_blocking(|| {
            show_notification("Launching game", "Refreshing account...");
        });
    }

    let refresh_token = ql_auth::token_store::read_refresh_token_from(
        refresh_name,
        account.account_type.unwrap_or_default(),
        method,
    )?;

    // Hook: Account types
    let account = if let Some(account_type @ (AccountType::ElyBy | AccountType::LittleSkin)) =
        account.account_type
    {
        ql_auth::yggdrasil::login_refresh(refresh_name.to_owned(), refresh_token, account_type)
            .await?
    } else {
        let refresh_token = ql_auth::token_store::read_refresh_token_from(
            username,
            AccountType::Microsoft,
            method,
        )?;
        ql_auth::ms::login_refresh(username.clone(), refresh_token, None).await?
    };

    Ok(Some(account))
}

fn unlock_encrypted_store_if_needed(
    method: TokenStorageMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    if method != TokenStorageMethod::EncryptedFile {
        return Ok(());
    }
    if ql_auth::encrypted_store::is_unlocked() {
        return Ok(());
    }
    if !ql_auth::encrypted_store::file_exists() {
        return Err(ql_auth::encrypted_store::EncryptedStoreError::FileNotFound.into());
    }

    if let Ok(secret) = std::env::var("QL_FILE_SECRET") {
        let secret = secret.trim();
        if !secret.is_empty() {
            ql_auth::encrypted_store::unlock(secret)?;
            return Ok(());
        }
    }

    if !io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Encrypted token store is locked. Set QL_FILE_SECRET or run in an interactive terminal to enter the password.",
        )
        .into());
    }

    let password = rpassword::prompt_password("Encrypted token store password: ")?;
    ql_auth::encrypted_store::unlock(&password)?;
    Ok(())
}

fn get_account<'a>(
    config: &'a LauncherConfig,
    username: &str,
    override_account_type: Option<&str>,
) -> Option<&'a ConfigAccount> {
    let Some(accounts) = &config.accounts else {
        return None;
    };

    if let Some(acc_type) = override_account_type {
        let acc_type = acc_type.to_lowercase();
        let acc_type = match acc_type.as_str() {
            "elyby" | "ely.by" => AccountType::ElyBy,
            "littleskin" | "littleskin.cn" => AccountType::LittleSkin,
            "microsoft" | "ms" => AccountType::Microsoft,
            _ => {
                err!(
                    "Unknown account type override: {}\nSupported types are: elyby, littleskin, microsoft",
                    acc_type.underline().bold()
                );
                exit(1);
            }
        };

        let key_username = acc_type.add_suffix_to_name(username);
        return accounts.get(&key_username);
    }

    accounts.get(username).or_else(|| {
        accounts
            .iter()
            .find(|a| {
                a.1.keyring_identifier
                    .as_ref()
                    .is_some_and(|i| i == username)
                    || a.1.username_nice.as_ref().is_some_and(|u| u == username)
            })
            .map(|n| n.1)
    })
}
