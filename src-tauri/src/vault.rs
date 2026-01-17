//! Vault encryption and decryption module.
//!
//! This module handles all cryptographic operations for the password vault:
//! - Key derivation using Argon2id (memory-hard KDF)
//! - Encryption/decryption using XChaCha20-Poly1305 (AEAD cipher)
//! - Vault file format management with versioning support
//!
//! # Security
//!
//! - **KDF**: Argon2id with 64 MiB memory, 3 iterations, parallelism=1
//! - **Cipher**: XChaCha20-Poly1305 (authenticated encryption)
//! - **Nonce**: 24 bytes, randomly generated per save operation
//! - **Salt**: 32 bytes, randomly generated once per vault
//! - **Memory Safety**: Sensitive data (keys, plaintext) zeroized after use

use crate::models::{Entry, NONCE_LEN, SALT_LEN, VAULT_FORMAT_VERSION};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs;
use std::io;
use std::path::Path;
use zeroize::Zeroize;

/// Errors that can occur during vault operations.
#[derive(Debug)]
pub enum VaultError {
  /// File I/O error
  Io(String),
  /// Invalid vault file format
  Format(String),
  /// Encryption/decryption error (wrong password or corrupted data)
  Crypto(String),
  /// JSON serialization/deserialization error
  Json(String),
  /// Key derivation function error
  Kdf(String),
}

/// Result of loading a vault: entries, salt, and derived key.
pub type VaultLoadResult = (Vec<Entry>, [u8; SALT_LEN], [u8; 32]);

impl From<io::Error> for VaultError {
  fn from(e: io::Error) -> Self {
    VaultError::Io(e.to_string())
  }
}

/// Generates a cryptographically secure random salt for key derivation.
///
/// # Returns
///
/// A 32-byte array filled with random data from the OS's CSPRNG.
///
/// # Security
///
/// Uses `OsRng` which provides cryptographically secure randomness.
/// The salt should be unique per vault and stored alongside the ciphertext.
pub fn generate_salt() -> [u8; SALT_LEN] {
  let mut salt = [0u8; SALT_LEN];
  OsRng.fill_bytes(&mut salt);
  salt
}

/// Derives a 256-bit encryption key from the master password using Argon2id.
///
/// # Parameters
///
/// - `master_password`: User's master password (not stored, used only for derivation)
/// - `salt`: 32-byte random salt (unique per vault)
///
/// # Returns
///
/// A 32-byte (256-bit) key suitable for XChaCha20-Poly1305 encryption.
///
/// # Security
///
/// Uses Argon2id (RFC 9106) with:
/// - Memory cost: 64 MiB (65,536 KiB) - resists GPU/ASIC attacks
/// - Time cost: 3 iterations - balances security and performance
/// - Parallelism: 1 thread - suitable for interactive use
/// - Output length: 32 bytes (256 bits)
///
/// These parameters provide strong protection against brute force attacks
/// while remaining responsive on modern hardware (~200ms on typical machines).
pub fn derive_key(master_password: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; 32], VaultError> {
  // Interactive-optimized parameters: 64 MiB memory, 3 iterations, 1 thread, 32-byte output
  let params = Params::new(64 * 1024, 3, 1, Some(32))
    .map_err(|e| VaultError::Kdf(format!("argon2 params: {e}")))?;
  let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

  let mut key = [0u8; 32];
  argon2
    .hash_password_into(master_password.as_bytes(), salt, &mut key)
    .map_err(|e| VaultError::Kdf(format!("argon2: {e}")))?;

  Ok(key)
}

/// Saves the vault with the current format version.
/// File format: [1B version][32B salt][24B nonce][ciphertext+tag]
pub fn save_with_key(
  path: &Path,
  entries: &[Entry],
  salt: &[u8; SALT_LEN],
  key_bytes: &[u8; 32],
) -> Result<(), VaultError> {
  let cipher = XChaCha20Poly1305::new(Key::from_slice(key_bytes));

  let mut nonce = [0u8; NONCE_LEN];
  OsRng.fill_bytes(&mut nonce);

  let mut plaintext =
    serde_json::to_vec(entries).map_err(|e| VaultError::Json(e.to_string()))?;

  let ciphertext = cipher
    .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
    .map_err(|e| VaultError::Crypto(e.to_string()))?;

  plaintext.zeroize();

  // New format: [version][salt][nonce][ciphertext]
  let mut out = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
  out.push(VAULT_FORMAT_VERSION);
  out.extend_from_slice(salt);
  out.extend_from_slice(&nonce);
  out.extend_from_slice(&ciphertext);

  fs::write(path, out)?;
  Ok(())
}

/// Loads the vault, supporting both versioned (v1+) and legacy (v0) formats.
/// Versioned format: [1B version][32B salt][24B nonce][ciphertext+tag]
/// Legacy format:    [32B salt][24B nonce][ciphertext+tag]
pub fn load_with_password(
  path: &Path,
  master_password: &str,
) -> Result<VaultLoadResult, VaultError> {
  let bytes = fs::read(path)?;

  // Minimum size check
  let min_size = SALT_LEN + NONCE_LEN;
  if bytes.len() < min_size {
    return Err(VaultError::Format("vault file too small".to_string()));
  }

  // Try to detect version byte (v1+ starts with 0x01, legacy starts with random salt)
  let (_version, offset) = if bytes[0] == VAULT_FORMAT_VERSION {
    // Versioned format detected
    if bytes.len() < 1 + SALT_LEN + NONCE_LEN {
      return Err(VaultError::Format("versioned vault file too small".to_string()));
    }
    (bytes[0], 1)
  } else {
    // Legacy format (no version byte)
    (0u8, 0)
  };

  let mut salt = [0u8; SALT_LEN];
  salt.copy_from_slice(&bytes[offset..(offset + SALT_LEN)]);

  let mut nonce = [0u8; NONCE_LEN];
  nonce.copy_from_slice(&bytes[(offset + SALT_LEN)..(offset + SALT_LEN + NONCE_LEN)]);

  let ciphertext = &bytes[(offset + SALT_LEN + NONCE_LEN)..];

  let mut key = derive_key(master_password, &salt)?;
  let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));

  let mut plaintext = cipher
    .decrypt(XNonce::from_slice(&nonce), ciphertext)
    .map_err(|e| VaultError::Crypto(e.to_string()))?;

  let entries: Vec<Entry> =
    serde_json::from_slice(&plaintext).map_err(|e| VaultError::Json(e.to_string()))?;

  // Zeroize plaintext bytes after parsing.
  plaintext.zeroize();

  // Version is available for future format migrations if needed.
  // Currently only used for logging in debug builds.
  #[cfg(debug_assertions)]
  eprintln!("Loaded vault format version: {}", _version);

  // We return a copy so caller can keep it while unlocked.
  let key_out = key;
  key.zeroize();

  Ok((entries, salt, key_out))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::Entry;
  use chrono::Utc;

  fn temp_file_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("the-organizer-test-{}-{}.dat", name, std::process::id()));
    p
  }

  #[test]
  fn roundtrip_encrypt_decrypt() {
    let path = temp_file_path("roundtrip");
    let _ = std::fs::remove_file(&path);

    let salt = generate_salt();
    let password = "correct horse battery staple";
    let key = derive_key(password, &salt).expect("kdf");

    let now = Utc::now();
    let entries = vec![Entry {
      id: "id1".to_string(),
      title: "Example".to_string(),
      username: "alice".to_string(),
      password: "secret".to_string(),
      url: "https://example.com".to_string(),
      notes: "n".to_string(),
      created_at: now,
      updated_at: now,
    }];

    save_with_key(&path, &entries, &salt, &key).expect("save");

    let (loaded, salt2, key2) = load_with_password(&path, password).expect("load");
    assert_eq!(salt, salt2);
    assert_eq!(key, key2);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].title, "Example");
    assert_eq!(loaded[0].username, "alice");
    assert_eq!(loaded[0].password, "secret");

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn wrong_password_fails() {
    let path = temp_file_path("wrongpw");
    let _ = std::fs::remove_file(&path);

    let salt = generate_salt();
    let password = "pw1";
    let key = derive_key(password, &salt).expect("kdf");

    let entries: Vec<Entry> = Vec::new();
    save_with_key(&path, &entries, &salt, &key).expect("save");

    let res = load_with_password(&path, "pw2");
    assert!(res.is_err());

    let _ = std::fs::remove_file(&path);
  }
}
