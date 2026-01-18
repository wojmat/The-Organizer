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

const VAULT_MAGIC: &[u8; 4] = b"TORG";

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
/// # Arguments
///
/// - `master_password`: The user-provided master password
/// - `salt`: A 32-byte salt unique to this vault
///
/// # Returns
///
/// A 32-byte key suitable for XChaCha20-Poly1305.
///
/// # Security
///
/// Uses Argon2id with memory-hard parameters to resist brute force attacks.
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
/// File format: [4B magic][1B version][32B salt][24B nonce][ciphertext+tag]
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

  // New format: [magic][version][salt][nonce][ciphertext]
  let mut out = Vec::with_capacity(4 + 1 + SALT_LEN + NONCE_LEN + ciphertext.len());
  out.extend_from_slice(VAULT_MAGIC);
  out.push(VAULT_FORMAT_VERSION);
  out.extend_from_slice(salt);
  out.extend_from_slice(&nonce);
  out.extend_from_slice(&ciphertext);

  fs::write(path, out)?;
  Ok(())
}

/// Loads the vault, supporting magic versioned (v1+), legacy versioned (v1), and legacy (v0) formats.
/// Magic format:   [4B magic][1B version][32B salt][24B nonce][ciphertext+tag]
/// Versioned:      [1B version][32B salt][24B nonce][ciphertext+tag]
/// Legacy format:  [32B salt][24B nonce][ciphertext+tag]
pub fn load_with_password(
  path: &Path,
  master_password: &str,
) -> Result<VaultLoadResult, VaultError> {
  let bytes = fs::read(path)?;

  // Minimum size check: salt + nonce + AEAD tag (ciphertext may be empty JSON, but tag is required).
  const AEAD_TAG_LEN: usize = 16;
  let min_v0_size = SALT_LEN + NONCE_LEN + AEAD_TAG_LEN;
  if bytes.len() < min_v0_size {
    return Err(VaultError::Format("vault file too small".to_string()));
  }

  // Parse/decrypt helper for different header offsets.
  let parse_at = |offset: usize| -> Result<VaultLoadResult, VaultError> {
    if bytes.len() < offset + SALT_LEN + NONCE_LEN + AEAD_TAG_LEN {
      return Err(VaultError::Format("vault file too small".to_string()));
    }

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

    // We return a copy so caller can keep it while unlocked.
    let key_out = key;
    key.zeroize();

    Ok((entries, salt, key_out))
  };

  // Detect formats:
  // - Magic format:   [4B magic][1B version][salt][nonce][ciphertext]
  // - Versioned:      [1B version][salt][nonce][ciphertext]  (legacy)
  // - Legacy v0:      [salt][nonce][ciphertext]
  //
  // IMPORTANT: legacy v0 can "collide" if salt[0] == VAULT_FORMAT_VERSION.
  // In that case, we must try versioned first, and if decrypt fails, fall back to v0.
  let (version, result) = if bytes.len() >= 5 && bytes[..4] == VAULT_MAGIC[..] {
    // Unambiguous: magic header.
    if bytes.len() < 4 + 1 + SALT_LEN + NONCE_LEN + AEAD_TAG_LEN {
      return Err(VaultError::Format("versioned vault file too small".to_string()));
    }
    (bytes[4], parse_at(5)?)
  } else if bytes[0] == VAULT_FORMAT_VERSION {
    // Ambiguous: could be legacy versioned, or legacy v0 with salt[0] == version byte.
    if bytes.len() < 1 + SALT_LEN + NONCE_LEN + AEAD_TAG_LEN {
      return Err(VaultError::Format("versioned vault file too small".to_string()));
    }

    match parse_at(1) {
      Ok(ok) => (bytes[0], ok),
      Err(e_v1 @ VaultError::Crypto(_)) => {
        // Fallback to legacy v0 parsing to handle version-byte collisions.
        // If v0 parsing also fails, return the original error.
        match parse_at(0) {
          Ok(ok) => (0u8, ok),
          Err(_) => return Err(e_v1),
        }
      }
      Err(e) => return Err(e),
    }
  } else {
    (0u8, parse_at(0)?)
  };

  #[cfg(debug_assertions)]
  eprintln!("Loaded vault format version: {}", version);

  Ok(result)
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

    let loaded = load_with_password(&path, password).expect("load");
    assert_eq!(loaded.0.len(), 1);
    assert_eq!(loaded.1, salt);
    assert_eq!(loaded.0[0].title, "Example");
    assert_eq!(loaded.0[0].username, "alice");
    assert_eq!(loaded.0[0].password, "secret");

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

  #[test]
  fn legacy_v0_compatibility_ignores_version_byte_collision() {
    use std::fs;
    use chacha20poly1305::aead::Aead;
    use chacha20poly1305::XChaCha20Poly1305;

    let path = temp_file_path("legacy-v0");
    let _ = std::fs::remove_file(&path);

    let password = "v0-compat";
    let mut salt = [0u8; SALT_LEN];
    salt[0] = VAULT_FORMAT_VERSION;

    let key = derive_key(password, &salt).expect("kdf");
    let entries: Vec<Entry> = Vec::new();

    let nonce = [0u8; NONCE_LEN];
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = serde_json::to_vec(&entries).expect("json");
    let ciphertext = cipher
      .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
      .expect("encrypt");

    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    fs::write(&path, out).expect("write");

    let loaded = load_with_password(&path, password).expect("load");
    assert_eq!(loaded.0.len(), 0);
    assert_eq!(loaded.1, salt);

    let _ = std::fs::remove_file(&path);
  }
}