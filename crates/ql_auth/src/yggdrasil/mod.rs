use crate::alt::AccountResponse;

use super::{token_store, AccountData, AccountType};
use ql_core::{info, pt, IntoJsonError, CLIENT};

pub use super::alt::{Account, AccountResponseError, Error};
use ql_core::request::check_for_success;
use serde::Serialize;

pub mod oauth;

const CLIENT_ID: &str = "1160";

#[derive(Serialize)]
struct Agent {
    name: &'static str,
    version: u8,
}
const AGENT: Agent = Agent {
    name: "Minecraft",
    version: 1,
};

pub async fn login_new(
    email_or_username: String,
    password: String,
    account_type: AccountType,
) -> Result<Account, Error> {
    info!("Logging into {account_type}... ({email_or_username})");
    let mut value = serde_json::json!({
        "username": &email_or_username,
        "password": &password,
        "clientToken": account_type.get_client_id()
    });
    insert_agent_field(account_type, &mut value);

    let response = CLIENT
        .post(account_type.yggdrasil_authenticate())
        .json(&value)
        .send()
        .await?;
    if response.status().as_u16() == 401 {
        return Err(Error::IncorrectPassword);
    }
    check_for_success(&response)?;
    let text = response.text().await?;

    let account_response = match serde_json::from_str::<AccountResponse>(&text).json(text.clone()) {
        Ok(n) => n,
        Err(err) => {
            if let Ok(res_err) = serde_json::from_str::<AccountResponseError>(&text).json(text) {
                if res_err.error == "ForbiddenOperationException"
                    && res_err.errorMessage == "Account protected with two factor auth."
                {
                    return Ok(Account::NeedsOTP);
                }
            }
            return Err(err.into());
        }
    };

    token_store::store_token_with(
        &email_or_username,
        account_type,
        &account_response.accessToken,
        token_store::get_storage_method(),
    )?;

    Ok(Account::Account(AccountData {
        access_token: Some(account_response.accessToken.clone()),
        uuid: account_response.selectedProfile.id,

        username: email_or_username,
        nice_username: account_response.selectedProfile.name,

        refresh_token: account_response.accessToken,
        needs_refresh: false,
        account_type,
    }))
}

fn insert_agent_field(account_type: AccountType, value: &mut serde_json::Value) {
    if account_type.yggdrasil_needs_agent_field() {
        if let (Some(value), Ok(insert)) = (value.as_object_mut(), serde_json::to_value(AGENT)) {
            value.insert("agent".to_owned(), insert);
        }
    }
}

pub async fn login_refresh(
    email_or_username: String,
    refresh_token: String,
    account_type: AccountType,
) -> Result<AccountData, Error> {
    pt!("Refreshing {account_type} account...");

    let mut value = serde_json::json!({
        "accessToken": refresh_token,
        "clientToken": account_type.get_client_id()
    });
    insert_agent_field(account_type, &mut value);
    let response = CLIENT
        .post(account_type.yggdrasil_refresh())
        .json(&value)
        .send()
        .await?;
    check_for_success(&response)?;
    let text = response.text().await?;

    let account_response = serde_json::from_str::<AccountResponse>(&text).json(text.clone())?;
    token_store::store_token_with(
        &email_or_username,
        account_type,
        &account_response.accessToken,
        token_store::get_storage_method(),
    )?;

    Ok(AccountData {
        access_token: Some(account_response.accessToken.clone()),
        uuid: account_response.selectedProfile.id,

        username: email_or_username,
        nice_username: account_response.selectedProfile.name,

        refresh_token: account_response.accessToken,
        needs_refresh: false,
        account_type,
    })
}
