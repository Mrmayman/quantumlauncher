//! Encrypted file-based token storage.
//!
//! Provides an alternative to system keyring storage
//! by encrypting tokens using AES-256-GCM with a password-derived key.
//!
//! # Security
//! - Uses Argon2id for key derivation (resistant to GPU/ASIC attacks)
//! - AES-256-GCM for authenticated encryption
//! - Unique salt per encryption file
//! - Unique nonce per token entry
//!
//! # Portability
//! The encrypted file (`encrypted_tokens.json`) lives in the launcher
//! directory and can be copied to other machines along with `config.json`.
//! Unlocking with the same password on the other machine restores all accounts.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ql_core::{info, IntoIoError, IntoJsonError, IoError, JsonError, LAUNCHER_DIR};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::PathBuf,
    sync::{Arc, RwLock},
};

/// The encrypted tokens file stored in the launcher directory.
const TOKENS_FILE: &str = "encrypted_tokens.json";

// Argon2 parameters for key derivation.
// Tuned for security while remaining reasonable on most hardware.
const ARGON2_M_COST: u32 = 65536; // 64 MB memory
const ARGON2_T_COST: u32 = 3; // 3 iterations
const ARGON2_P_COST: u32 = 4; // 4 parallel threads

/// In-memory cache for decrypted tokens.
/// Populated after the user enters their password on startup.
static TOKEN_CACHE: std::sync::LazyLock<Arc<RwLock<Option<TokenCache>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Cached decrypted tokens and derived key for the session.
struct TokenCache {
    /// The derived encryption key (kept for storing new tokens).
    key: [u8; 32],
    /// Decrypted tokens: storage_key -> refresh_token
    tokens: HashMap<String, String>,
    /// The salt used for key derivation (needed for saving).
    salt: String,
}

/// A known string we encrypt to verify password correctness.
const VERIFICATION_PLAINTEXT: &str = "QuantumLauncher_PasswordVerification_v1";

/// The on-disk format for encrypted tokens.
#[derive(Serialize, Deserialize)]
struct EncryptedTokensFile {
    /// Version of the file format (for future migrations).
    version: u32,
    /// Base64-encoded salt used for Argon2 key derivation.
    salt: String,
    /// Encrypted verification token to check password correctness.
    verification: EncryptedToken,
    /// Map of storage_key to encrypted token data.
    tokens: HashMap<String, EncryptedToken>,
}

/// A single encrypted token entry.
#[derive(Serialize, Deserialize, Clone)]
struct EncryptedToken {
    /// Base64-encoded 12-byte nonce for AES-GCM.
    nonce: String,
    /// Base64-encoded ciphertext (token + auth tag).
    ciphertext: String,
}

const ERR_PREFIX: &str = "while reading account data (from encrypted store):\n";

/// Errors that can occur during encrypted storage operations.
#[derive(Debug, thiserror::Error)]
pub enum EncryptedStoreError {
    #[error("{ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{ERR_PREFIX}{0}")]
    Json(#[from] JsonError),

    #[error("Encrypted token store (containing account data) is locked.\nPlease enter your password to unlock.")]
    NotUnlocked,
    #[error("{ERR_PREFIX}Encrypted tokens file not found. Please set up encrypted storage first.")]
    FileNotFound,
    #[error("{ERR_PREFIX}Invalid password. Please try again.")]
    InvalidPassword,
    #[error("{ERR_PREFIX}No token found for user: {0}")]
    TokenNotFound(String),

    #[error("{ERR_PREFIX}Encryption/decryption failed: {0}")]
    Encryption(String),

    #[error("{ERR_PREFIX}Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Get the path to the encrypted tokens file.
fn get_tokens_path() -> PathBuf {
    LAUNCHER_DIR.join(TOKENS_FILE)
}

/// Check if the encrypted tokens file exists.
#[must_use]
pub fn file_exists() -> bool {
    get_tokens_path().exists()
}

/// Check if the encrypted store is currently unlocked (password has been entered).
#[must_use]
pub fn is_unlocked() -> bool {
    TOKEN_CACHE
        .read()
        .map(|cache| cache.is_some())
        .unwrap_or(false)
}

/// Derive an encryption key from a password using Argon2id.
fn derive_key(password: &str, salt: &SaltString) -> Result<[u8; 32], EncryptedStoreError> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
            .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?,
    );

    let hash = argon2
        .hash_password(password.as_bytes(), salt)
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    let hash_bytes = hash
        .hash
        .ok_or_else(|| EncryptedStoreError::Encryption("Failed to get hash output".to_string()))?;

    let mut key = [0u8; 32];
    key.copy_from_slice(hash_bytes.as_bytes());
    Ok(key)
}

/// Initialize a new encrypted tokens file with the given password.
///
/// Creates a new file with a fresh salt. Call when the user first sets up
/// encrypted storage or wants to change their password.
pub fn initialize_new(password: &str) -> Result<(), EncryptedStoreError> {
    info!("Initializing new encrypted token store...");

    let salt = SaltString::generate(&mut OsRng);
    let key = derive_key(password, &salt)?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, VERIFICATION_PLAINTEXT.as_bytes())
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    let verification = EncryptedToken {
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
    };

    let file = EncryptedTokensFile {
        version: 1,
        salt: salt.to_string(),
        verification,
        tokens: HashMap::new(),
    };

    let json = serde_json::to_string_pretty(&file).json_to()?;
    let path = get_tokens_path();
    std::fs::write(&path, json).path(&path)?;

    let mut cache = TOKEN_CACHE.write().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;
    *cache = Some(TokenCache {
        key,
        tokens: HashMap::new(),
        salt: salt.to_string(),
    });

    info!("Encrypted token store initialized successfully");
    Ok(())
}

/// Unlock the encrypted store with the given password.
///
/// Decrypts all tokens and caches them in memory for the session.
/// Must be called before any token operations when using encrypted storage.
pub fn unlock(password: &str) -> Result<(), EncryptedStoreError> {
    info!(no_log, "Unlocking encrypted token store...");

    let file = read_json_file()?;

    let salt = SaltString::from_b64(&file.salt)
        .map_err(|e| EncryptedStoreError::Encryption(format!("Invalid salt: {e}")))?;
    let key = derive_key(password, &salt)?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    // Verify password by decrypting the verification token
    let verify_nonce_bytes = BASE64.decode(&file.verification.nonce)?;
    let verify_ciphertext = BASE64.decode(&file.verification.ciphertext)?;
    let verify_nonce = Nonce::from_slice(&verify_nonce_bytes);

    let verify_plaintext = cipher
        .decrypt(verify_nonce, verify_ciphertext.as_ref())
        .map_err(|_| {
            ql_core::err!(no_log, "Wrong passphrase! Please try again.");
            EncryptedStoreError::InvalidPassword
        })?;

    if verify_plaintext != VERIFICATION_PLAINTEXT.as_bytes() {
        ql_core::err!(no_log, "Wrong passphrase! Please try again.");
        return Err(EncryptedStoreError::InvalidPassword);
    }

    // Decrypt all tokens
    let mut decrypted_tokens = HashMap::new();
    for (storage_key, encrypted) in &file.tokens {
        let nonce_bytes = BASE64.decode(&encrypted.nonce)?;
        let ciphertext = BASE64.decode(&encrypted.ciphertext)?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| EncryptedStoreError::InvalidPassword)?;

        let token = String::from_utf8(plaintext)
            .map_err(|e| EncryptedStoreError::Encryption(format!("Invalid UTF-8: {e}")))?;
        decrypted_tokens.insert(storage_key.clone(), token);
    }

    let mut cache = TOKEN_CACHE.write().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;
    *cache = Some(TokenCache {
        key,
        tokens: decrypted_tokens,
        salt: file.salt,
    });

    info!(no_log, "Encrypted token store unlocked successfully");
    Ok(())
}

fn read_json_file() -> Result<EncryptedTokensFile, EncryptedStoreError> {
    let path = get_tokens_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(n) => n,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(EncryptedStoreError::FileNotFound);
        }
        Err(err) => {
            return Err(IoError::Io {
                error: err.to_string(),
                path,
            }
            .into())
        }
    };
    let file: EncryptedTokensFile = serde_json::from_str(&content).json(content)?;
    Ok(file)
}

/// Lock the encrypted store (clear the in-memory cache).
pub fn lock() {
    if let Ok(mut cache) = TOKEN_CACHE.write() {
        *cache = None;
    }
}

/// Delete the encrypted store file from disk.
///
/// The in-memory cache should be cleared separately by calling [`lock`].
pub fn delete_store() -> Result<(), EncryptedStoreError> {
    let path = get_tokens_path();
    if path.exists() {
        std::fs::remove_file(&path).path(&path)?;
        info!("Encrypted token store deleted");
    }
    Ok(())
}

/// Store a token for the given storage key.
///
/// The store must be unlocked first.
pub fn store_token(storage_key: &str, token: &str) -> Result<(), EncryptedStoreError> {
    let mut cache_guard = TOKEN_CACHE.write().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;

    let cache = cache_guard
        .as_mut()
        .ok_or(EncryptedStoreError::NotUnlocked)?;

    cache
        .tokens
        .insert(storage_key.to_string(), token.to_string());
    save_to_disk(&cache.key, &cache.salt, &cache.tokens)?;
    Ok(())
}

/// Read a token for the given storage key.
///
/// The store must be unlocked first.
pub fn read_token(storage_key: &str) -> Result<String, EncryptedStoreError> {
    let cache_guard = TOKEN_CACHE.read().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;

    let cache = cache_guard
        .as_ref()
        .ok_or(EncryptedStoreError::NotUnlocked)?;

    cache
        .tokens
        .get(storage_key)
        .cloned()
        .ok_or_else(|| EncryptedStoreError::TokenNotFound(storage_key.to_string()))
}

/// Delete a token for the given storage key.
///
/// The store must be unlocked first.
pub fn delete_token(storage_key: &str) -> Result<(), EncryptedStoreError> {
    let mut cache_guard = TOKEN_CACHE.write().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;

    let cache = cache_guard
        .as_mut()
        .ok_or(EncryptedStoreError::NotUnlocked)?;

    cache.tokens.remove(storage_key);
    save_to_disk(&cache.key, &cache.salt, &cache.tokens)?;
    Ok(())
}

/// Change the password for the encrypted store.
///
/// Re-encrypts all tokens with a new key derived from the new password.
fn change_password(old_password: &str, new_password: &str) -> Result<(), EncryptedStoreError> {
    if !is_unlocked() {
        unlock(old_password)?;
    }

    let cache_guard = TOKEN_CACHE.read().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;

    let old_cache = cache_guard
        .as_ref()
        .ok_or(EncryptedStoreError::NotUnlocked)?;
    let tokens = old_cache.tokens.clone();
    drop(cache_guard);

    let new_salt = SaltString::generate(&mut OsRng);
    let new_key = derive_key(new_password, &new_salt)?;

    save_to_disk(&new_key, new_salt.as_str(), &tokens)?;

    let mut cache_guard = TOKEN_CACHE.write().map_err(|e| {
        EncryptedStoreError::Encryption(format!("Failed to acquire cache lock: {e}"))
    })?;
    *cache_guard = Some(TokenCache {
        key: new_key,
        tokens,
        salt: new_salt.to_string(),
    });

    info!("Password changed successfully");
    Ok(())
}

/// Save all tokens to disk (called after any modification).
fn save_to_disk(
    key: &[u8; 32],
    salt: &str,
    tokens: &HashMap<String, String>,
) -> Result<(), EncryptedStoreError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    // Create a fresh verification token (re-encrypted each save)
    let mut verify_nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut verify_nonce_bytes);
    let verify_nonce = Nonce::from_slice(&verify_nonce_bytes);

    let verify_ciphertext = cipher
        .encrypt(verify_nonce, VERIFICATION_PLAINTEXT.as_bytes())
        .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

    let verification = EncryptedToken {
        nonce: BASE64.encode(verify_nonce_bytes),
        ciphertext: BASE64.encode(verify_ciphertext),
    };

    let mut encrypted_tokens = HashMap::new();
    for (storage_key, token) in tokens {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, token.as_bytes())
            .map_err(|e| EncryptedStoreError::Encryption(e.to_string()))?;

        encrypted_tokens.insert(
            storage_key.clone(),
            EncryptedToken {
                nonce: BASE64.encode(nonce_bytes),
                ciphertext: BASE64.encode(ciphertext),
            },
        );
    }

    let file = EncryptedTokensFile {
        version: 1,
        salt: salt.to_string(),
        verification,
        tokens: encrypted_tokens,
    };

    let json = serde_json::to_string_pretty(&file).json_to()?;
    let path = get_tokens_path();
    std::fs::write(&path, json).path(&path)?;
    Ok(())
}
