use ql_core::{JsonError, RequestError};
use serde::Deserialize;

use crate::auth::KeyringError;

use super::AccountData;

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct AccountResponseError {
    pub error: String,
    pub errorMessage: String,
}

impl std::error::Error for AccountResponseError {}
impl std::fmt::Display for AccountResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.errorMessage)
    }
}

const AUTH_ERR_PREFIX: &str = "while logging into ely.by/littleskin account:\n";
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{AUTH_ERR_PREFIX}tokio::task join:\n{0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("{AUTH_ERR_PREFIX}\n{0}")]
    Response(#[from] AccountResponseError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    KeyringError(#[from] KeyringError),
    #[error("{AUTH_ERR_PREFIX}Littleskin response:\n{0}")]
    LittleSkin(String),
    #[error("incorrect password entered (ely.by/littleskin account)")]
    IncorrectPassword,

    #[error("{AUTH_ERR_PREFIX}while logging in through oauth:\n{0}")]
    Oauth(#[from] OauthError),
}

#[derive(Debug, thiserror::Error)]
pub enum OauthError {
    #[error("device code expired")]
    DeviceCodeExpired,
    #[error("unexpected response from littleskin:\n\n{0}")]
    UnexpectedResponse(String),
    #[error("no access token in response")]
    NoAccessToken,
    #[error("no minecraft profile found for account")]
    NoMinecraftProfile,
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(RequestError::ReqwestError(value))
    }
}

impl From<keyring::Error> for Error {
    fn from(err: keyring::Error) -> Self {
        Self::KeyringError(KeyringError(err))
    }
}

#[derive(Debug, Clone)]
pub enum Account {
    Account(AccountData),
    NeedsOTP,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct AccountResponse {
    pub accessToken: String,
    pub selectedProfile: AccountResponseProfile,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AccountResponseProfile {
    pub id: String,
    pub name: String,
}
