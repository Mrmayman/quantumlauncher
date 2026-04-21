use owo_colors::OwoColorize;
use std::process::exit;

use ql_core::err;
use ql_instances::auth::{self, AccountType};

use crate::{
    cli::show_notification,
    config::{ConfigAccount, LauncherConfig},
};

pub async fn refresh_account(
    username: &String,
    use_account: bool,
    show_progress: bool,
    override_account_type: Option<&str>,
) -> Result<Option<auth::AccountData>, Box<dyn std::error::Error>> {
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

    if show_progress {
        tokio::task::spawn_blocking(|| {
            show_notification("Launching game", "Refreshing account...");
        });
    }

    let refresh_token =
        auth::read_refresh_token(refresh_name, account.account_type.unwrap_or_default())?;

    // Hook: Account types
    let account = if let Some(account_type @ (AccountType::ElyBy | AccountType::LittleSkin)) =
        account.account_type
    {
        auth::yggdrasil::login_refresh(refresh_name.to_owned(), refresh_token, account_type).await?
    } else {
        let refresh_token = auth::read_refresh_token(username, AccountType::Microsoft)?;
        auth::ms::login_refresh(username.clone(), refresh_token, None).await?
    };

    Ok(Some(account))
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
