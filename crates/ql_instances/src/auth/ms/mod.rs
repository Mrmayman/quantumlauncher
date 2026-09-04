//! # Minecraft Authentication for Microsoft Accounts
//!
//! This module allows you to log into Minecraft with
//! your paid Microsoft account.
//!
//! Taken from <https://github.com/minecraft-rs/auth>
//!
//! ## Modifications:
//! - Changed to `reqwest::Client` and `async`
//!   from `reqwest::blocking::Client`
//! - Changed error handling code
//! - Split it up into clean, independent functions
//!
//! # Login Process
//! ## 1) Adding a new account
//! If you are logging in and adding a new account, then:
//!
//! ```no_run
//! # async fn do1() -> Result<(), Box<dyn std::error::Error>> {
//! use ql_instances::auth::ms::login_1_link;
//! let auth_code_response = login_1_link().await?;
//! // AuthCodeResponse { verification_uri, user_code, .. }
//! # Ok(()) }
//! ```
//!
//! Now we wait for user to open the `verification_uri` link in browser,
//! login with their account,
//! then enter `user_code`.
//!
//! ```no_run
//! # async fn do2() -> Result<(), Box<dyn std::error::Error>> {
//! # // Default construction
//! # let auth_code_response = ql_instances::auth::ms::AuthCodeResponse {
//! #     user_code: String::new(),
//! #     device_code: String::new(),
//! #     verification_uri: String::new(),
//! #     expires_in: 0,
//! #     interval: 0,
//! #     message: String::new(),
//! # };
//! use ql_instances::auth::ms::login_3_xbox;
//! use ql_instances::auth::ms::login_2_wait;
//!
//! let auth_token_response = login_2_wait(auth_code_response).await?;
//! // AuthTokenResponse { access_token, refresh_token }
//!
//! let account_data = login_3_xbox(auth_token_response, None, true).await?;
//! // AccountData { access_token, uuid, username, refresh_token, needs_refresh }
//! # Ok(()) }
//! ```
//!
//! Now save the `username` and corresponding `refresh_token` to disk
//! and play the game with `access_token`.
//!
//! ## 2) Refreshing the account on every play session
//! After starting the launcher later, to refresh
//! the token, we do
//!
//! ```no_run
//! # async fn do3() -> Result<(), Box<dyn std::error::Error>> {
//! # let username = String::new();
//! # let refresh_token = String::new();
//! use ql_instances::auth::ms::login_refresh;
//! let account_data = login_refresh(username, refresh_token, None).await?;
//! # Ok(()) }
//! ```

use ql_core::{CLIENT, GenericProgress, IntoJsonError, info, pt, retry};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::auth::AccountType;

use super::AccountData;

mod error;
pub use error::{Error, MsaResponseError, ResponseError};

/// The API key for logging into Minecraft.
///
/// It's (kinda) safe to leave this public,
/// as the worst that can happen is someone
/// uses this for auth in their own launcher.
/// If you're working on Quantum Launcher or
/// just playing around with your own code
/// **for testing purposes** feel free to use this.
///
/// **Do not use this for any real projects or production code,
/// outside of this launcher**.
pub const CLIENT_ID: &str = "43431a16-38f5-4b42-91f9-4bf70c3bee1e";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthCodeResponse {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    expires_in: isize,
    interval: u64,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthTokenResponse {
    // pub token_type: String,
    // pub scope: String,
    // pub expires_in: i64,
    // pub ext_expires_in: i64,
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLiveAuthResponse {
    issue_instant: String,
    not_after: String,
    token: String,
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Deserialize, Debug, Clone)]
struct MinecraftAuthResponse {
    access_token: String,
    // username: String,
    // roles: Vec<String>,
    // expires_in: u32,
    // token_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RefreshResponse {
    // pub expires_in: u64,
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct MinecraftFinalDetails {
    id: Option<String>,
    name: String,
}

/// Gets the account info from the
/// refresh token.
///
/// You can read an existing refresh token
/// from disk using [`super::read_refresh_token`].
///
/// This is for reusing an existing logged-in
/// account. If you want to freshly log in, use
/// [`login_1_link`], [`login_2_wait`], [`login_3_xbox`]
/// respectively in that order.
pub async fn login_refresh(
    username: String,
    refresh_token: String,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, Error> {
    send_progress(sender.as_ref(), 0, 4, "Refreshing account token...");

    let response: String = retry(|| async {
        CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .form(&[
                ("client_id", CLIENT_ID),
                ("refresh_token", &refresh_token),
                ("grant_type", "refresh_token"),
                ("redirect_uri", "https://login.live.com/oauth20_desktop.srf"),
                ("scope", "XboxLive.signin offline_access"),
            ])
            .send()
            .await?
            .text()
            .await
    })
    .await?;

    let data: RefreshResponse = parse_json(&response)?;

    let entry = keyring::Entry::new("QuantumLauncher", &username)?;
    entry.set_password(&data.refresh_token)?;

    let data = login_3_xbox(
        AuthTokenResponse {
            access_token: data.access_token,
            refresh_token: data.refresh_token,
        },
        sender,
        false,
    )
    .await?;

    Ok(data)
}

pub async fn login_1_link() -> Result<AuthCodeResponse, Error> {
    info!("Logging into Microsoft Account...");

    pt!("Sending device code request");
    let response = CLIENT
        .get("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .query(&[
            ("client_id", CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .text()
        .await?;

    let data: AuthCodeResponse = parse_json(&response)?;

    pt!(
        "Go to {} and enter code {} (shown in menu)",
        data.verification_uri,
        data.user_code
    );

    Ok(data)
}

pub async fn login_3_xbox(
    data: AuthTokenResponse,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
    check_ownership: bool,
) -> Result<AccountData, Error> {
    let steps = if check_ownership { 5 } else { 4 };

    send_progress(sender.as_ref(), 1, steps, "Logging into Xbox live...");
    let xbox = login_in_xbox_live(&CLIENT, &data).await?;
    send_progress(sender.as_ref(), 2, steps, "Logging into Minecraft...");
    let minecraft = login_in_minecraft(&CLIENT, &xbox).await?;
    send_progress(sender.as_ref(), 3, steps, "Getting account details...");
    let final_details = get_final_details(&CLIENT, &minecraft).await?;

    if check_ownership {
        send_progress(sender.as_ref(), 4, steps, "Checking game ownership...");
        let owns_game = check_minecraft_ownership(&minecraft.access_token).await?;

        if !owns_game {
            return Err(Error::DoesntOwnGame);
        }
    }

    let entry = keyring::Entry::new("QuantumLauncher", &final_details.name)?;
    entry.set_password(&data.refresh_token)?;

    let data = AccountData {
        access_token: Some(minecraft.access_token),
        uuid: final_details.id.ok_or(Error::NoUuid)?,
        refresh_token: data.refresh_token,
        needs_refresh: false,
        account_type: AccountType::Microsoft,

        username: final_details.name.clone(),
        nice_username: final_details.name,
    };

    info!("Finished Microsoft Account login!");

    Ok(data)
}

fn send_progress(
    sender: Option<&std::sync::mpsc::Sender<GenericProgress>>,
    done: usize,
    total: usize,
    message: &str,
) {
    pt!("{message}");
    if let Some(sender) = sender {
        _ = sender.send(GenericProgress {
            done,
            total,
            message: Some(message.to_owned()),
            has_finished: false,
        });
    }
}

pub async fn login_2_wait(response: AuthCodeResponse) -> Result<AuthTokenResponse, Error> {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(response.interval + 1)).await;

        let code_resp = CLIENT
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("client_id", CLIENT_ID),
                ("scope", "XboxLive.signin offline_access"),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &response.device_code),
            ])
            .send()
            .await?;

        match code_resp.status() {
            StatusCode::BAD_REQUEST => {
                #[derive(Deserialize)]
                struct AuthServiceErrorMessage {
                    error: String,
                }

                let txt = code_resp.text().await?;
                let error: AuthServiceErrorMessage = parse_json(&txt)?;
                match error.error.as_str() {
                    "authorization_declined" | "expired_token" | "invalid_grant" => {
                        return Err(Error::InvalidAccessToken);
                    }
                    _ => {}
                }
            }

            StatusCode::OK => {
                let text = code_resp.text().await?;
                let response: AuthTokenResponse = parse_json(&text)?;
                return Ok(response);
            }
            code => {
                return Err(Error::UnknownError(code));
            }
        }
    }
}

async fn login_in_xbox_live(
    client: &Client,
    auth_token: &AuthTokenResponse,
) -> Result<XboxLiveAuthResponse, Error> {
    let xbox_authenticate_json = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": &format!("d={}", auth_token.access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbox_res = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbox_authenticate_json)
        .send()
        .await?
        .text()
        .await?;

    let xbox_res: XboxLiveAuthResponse = parse_json(&xbox_res)?;
    Ok(xbox_res)
}

async fn login_in_minecraft(
    client: &Client,
    xbox_res: &XboxLiveAuthResponse,
) -> Result<MinecraftAuthResponse, Error> {
    let xbox_token = &xbox_res.token;
    let user_hash = &xbox_res
        .display_claims
        .get("xui")
        .ok_or(Error::MissingField(
            "xbox_res.display_claims.xui".to_owned(),
        ))?
        .first()
        .ok_or(Error::MissingField(
            "xbox_res.display_claims.xui[0]".to_owned(),
        ))?
        .get("uhs")
        .ok_or(Error::MissingField(
            "xbox_res.display_claims.xui[0].uhs".to_owned(),
        ))?;

    let xbox_security_token_res = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .text()
        .await?;

    let xbox_security_token_res: XboxLiveAuthResponse = parse_json(&xbox_security_token_res)?;

    let xbox_security_token = &xbox_security_token_res.token;

    let minecraft_resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({
            "identityToken":
                format!(
                    "XBL3.0 x={user_hash};{xbox_security_token}"
                )
        }))
        .send()
        .await?
        .text()
        .await?;

    let minecraft_resp: MinecraftAuthResponse = parse_json(&minecraft_resp)?;
    Ok(minecraft_resp)
}

async fn get_final_details(
    client: &Client,
    minecraft_res: &MinecraftAuthResponse,
) -> Result<MinecraftFinalDetails, Error> {
    let text = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Accept", "application/json")
        .bearer_auth(&minecraft_res.access_token)
        .send()
        .await?
        .text()
        .await?;

    let info: MinecraftFinalDetails = parse_json(&text)?;
    Ok(info)
}

async fn check_minecraft_ownership(access_token: &str) -> Result<bool, Error> {
    #[derive(Deserialize)]
    struct Ownership {
        items: Vec<serde_json::Value>,
    }

    let client = Client::new();

    let response = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(access_token)
        .send()
        .await?
        .text()
        .await?;
    let response: Ownership = parse_json(&response)?;

    Ok(!response.items.is_empty())
}

fn parse_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Error> {
    match serde_json::from_str(text) {
        Ok(n) => Ok(n),
        Err(err) => {
            if let Ok(response_err) = serde_json::from_str::<MsaResponseError>(text) {
                Err(response_err.into())
            } else if let Ok(response_err) = serde_json::from_str::<ResponseError>(text) {
                Err(response_err.into())
            } else {
                Err(err.json(text.to_owned()).into())
            }
        }
    }
}
