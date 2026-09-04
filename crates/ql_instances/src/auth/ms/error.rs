use ql_core::{JsonError, RequestError};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::auth::KeyringError;

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct MsaResponseError {
    pub path: String,
    pub error: String,
    pub errorMessage: String,
}

impl std::fmt::Display for MsaResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error: {}\nat: {}\n({})",
            self.errorMessage, self.path, self.error
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseError {
    pub error: String,
    pub error_description: String,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.error_description)
    }
}

const AUTH_ERR_PREFIX: &str = "while managing Microsoft account:\n";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{AUTH_ERR_PREFIX}Invalid account access token!")]
    InvalidAccessToken,
    #[error(
        "{AUTH_ERR_PREFIX}An unknown error has occurred (code: {0})\n\nThis is a major bug! Please report in discord."
    )]
    UnknownError(StatusCode),
    #[error("{AUTH_ERR_PREFIX}missing JSON field: {0}")]
    MissingField(String),
    #[error("{AUTH_ERR_PREFIX}no uuid found for account")]
    NoUuid,
    #[error("{AUTH_ERR_PREFIX}{0}")]
    KeyringError(#[from] KeyringError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Response1(MsaResponseError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Response2(ResponseError),

    #[error(
        "Your Microsoft account doesn't own Minecraft!\nJust enter the username in the text box instead of logging in."
    )]
    DoesntOwnGame,
    #[error("Your account login has expired. Please log out and log in again.")]
    LoginExpired,
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

impl From<MsaResponseError> for Error {
    fn from(err: MsaResponseError) -> Self {
        if err.error == "NOT_FOUND" {
            Error::DoesntOwnGame
        } else {
            Error::Response1(err)
        }
    }
}

impl From<ResponseError> for Error {
    fn from(err: ResponseError) -> Self {
        if err.error.contains("invalid_grant") && err.error_description.contains("grant is expired")
        {
            Error::LoginExpired
        } else {
            Error::Response2(err)
        }
    }
}
