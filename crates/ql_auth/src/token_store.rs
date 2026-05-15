//! Abstraction layer for token storage backends.
//!
//! Routes between system keyring and the encrypted file backend.
//! The global `STORAGE_METHOD` tracks which backend is currently active.

use std::sync::atomic::{AtomicU8, Ordering};

use super::{
    encrypted_store::{self, EncryptedStoreError},
    AccountType, KeyringError,
};

use ql_core::{err, IntoStringError};

/// Global storage method selector.
/// 0 = Keyring, 1 = EncryptedFile
static STORAGE_METHOD: AtomicU8 = AtomicU8::new(0);

/// Which backend to use for token storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TokenStorageMethod {
    Keyring = 0,
    EncryptedFile = 1,
}

impl TokenStorageMethod {
    #[must_use]
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => TokenStorageMethod::EncryptedFile,
            _ => TokenStorageMethod::Keyring,
        }
    }
}

impl std::fmt::Display for TokenStorageMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenStorageMethod::Keyring => write!(f, "System Keyring"),
            TokenStorageMethod::EncryptedFile => write!(f, "Encrypted File"),
        }
    }
}

/// Set the global storage method.
pub fn set_storage_method(method: TokenStorageMethod) {
    STORAGE_METHOD.store(method as u8, Ordering::Relaxed);
}

/// Get the current global storage method.
#[must_use]
pub fn get_storage_method() -> TokenStorageMethod {
    TokenStorageMethod::from_u8(STORAGE_METHOD.load(Ordering::Relaxed))
}

/// Check if the system keyring is available and functional.
/// Especially useful on Linux to detect missing secret-service.
#[must_use]
pub fn is_keyring_available() -> bool {
    let entry = match keyring::Entry::new("QuantumLauncher", "availability_test") {
        Ok(e) => e,
        Err(_) => return false,
    };

    // To accurately check if the keyring is functional and unlocked (or unlockable),
    // we attempt to set a dummy password. This forces a check of the underlying 
    // storage subsystem and will trigger an unlock prompt if necessary.
    // Since this result is usually cached by the caller, it's safe to do once.
    if entry.set_password("availability_test").is_err() {
        return false;
    }

    // Clean up the test entry
    _ = entry.delete_credential();
    true
}

/// Errors from the token store abstraction layer.
#[derive(Debug, thiserror::Error)]
pub enum TokenStoreError {
    #[error("Keyring error: {0}")]
    Keyring(#[from] KeyringError),
    #[error("{0}")]
    EncryptedStore(#[from] EncryptedStoreError),
}

/// Store a token using the *currently active* backend.
pub fn store_token(
    username: &str,
    account_type: AccountType,
    token: &str,
) -> Result<(), TokenStoreError> {
    store_token_with(username, account_type, token, get_storage_method())
}

/// Store a token using a *specific* backend (ignores global method).
pub fn store_token_with(
    username: &str,
    account_type: AccountType,
    token: &str,
    method: TokenStorageMethod,
) -> Result<(), TokenStoreError> {
    match method {
        TokenStorageMethod::Keyring => {
            let entry = account_type.get_keyring_entry(username)?;
            entry.set_password(token).map_err(KeyringError)?;
            Ok(())
        }
        TokenStorageMethod::EncryptedFile => {
            let key = account_type.create_storage_key(username);
            encrypted_store::store_token(&key, token)?;
            Ok(())
        }
    }
}

fn read_refresh_token_keyring(
    username: &str,
    account_type: AccountType,
) -> Result<String, KeyringError> {
    let entry = account_type.get_keyring_entry(username)?;
    let refresh_token = entry.get_password()?;
    Ok(refresh_token)
}

/// Read a token using the *currently active* backend.
pub fn read_refresh_token(
    username: &str,
    account_type: AccountType,
) -> Result<String, TokenStoreError> {
    read_refresh_token_from(username, account_type, get_storage_method())
}

/// Read a token using a *specific* backend (ignores global method).
pub fn read_refresh_token_from(
    username: &str,
    account_type: AccountType,
    method: TokenStorageMethod,
) -> Result<String, TokenStoreError> {
    match method {
        TokenStorageMethod::Keyring => Ok(read_refresh_token_keyring(username, account_type)?),
        TokenStorageMethod::EncryptedFile => {
            let key = account_type.create_storage_key(username);
            let token = encrypted_store::read_token(&key)?;
            Ok(token)
        }
    }
}

fn logout_keyring(username: &str, account_type: AccountType) -> Result<(), String> {
    let entry = account_type.get_keyring_entry(username).strerr()?;
    if let Err(err) = entry.delete_credential() {
        err!("Couldn't remove {account_type} account credential (Username: {username}):\n{err}");
    }
    Ok(())
}

/// Delete a token using the *currently active* backend.
pub fn logout(username: &str, account_type: AccountType) -> Result<(), TokenStoreError> {
    logout_from(username, account_type, get_storage_method())
}

/// Delete a token using a *specific* backend.
pub fn logout_from(
    username: &str,
    account_type: AccountType,
    method: TokenStorageMethod,
) -> Result<(), TokenStoreError> {
    match method {
        TokenStorageMethod::Keyring => {
            logout_keyring(username, account_type)
                .map_err(|_| TokenStoreError::Keyring(KeyringError(keyring::Error::NoEntry)))?;
            Ok(())
        }
        TokenStorageMethod::EncryptedFile => {
            let key = account_type.create_storage_key(username);
            encrypted_store::delete_token(&key)?;
            Ok(())
        }
    }
}
