//! Core data structures and application state for The Organizer.
//!
//! This module defines all the fundamental types used throughout the application:
//! - [`Entry`] - A password entry with secure memory handling
//! - [`VaultSession`] - Active session containing the derived encryption key
//! - [`FailedAttemptTracker`] - Rate limiting for failed unlock attempts
//! - [`AppState`] - Central application state shared across threads
//!
//! # Security
//!
//! - All sensitive data implements [`Zeroize`] to securely clear memory on drop
//! - The master password is never stored; only the derived key is kept in memory
//! - Session keys are wrapped in [`Zeroizing`] for automatic secure cleanup

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;
use zeroize::Zeroize;
use zeroize::Zeroizing;

/// Filename for the encrypted vault file.
pub const VAULT_FILENAME: &str = "vault.dat";

/// Current vault file format version (v1).
pub const VAULT_FORMAT_VERSION: u8 = 0x01;

/// Length of the salt used for key derivation (32 bytes).
pub const SALT_LEN: usize = 32;

/// Length of the nonce used for XChaCha20-Poly1305 encryption (24 bytes).
pub const NONCE_LEN: usize = 24;

/// How often the inactivity monitor checks for timeout (10 seconds).
pub const INACTIVITY_POLL_SECS: u64 = 10;

/// Auto-lock timeout duration (5 minutes of inactivity).
pub const INACTIVITY_TIMEOUT_SECS: u64 = 300;

/// Maximum failed unlock attempts before lockout.
pub const MAX_FAILED_ATTEMPTS: u32 = 5;

/// Duration of lockout after exceeding failed attempts (30 seconds).
pub const LOCKOUT_DURATION_SECS: u64 = 30;

/// Default port for the browser extension local API bridge.
pub const EXTENSION_DEFAULT_PORT: u16 = 17832;

/// Configuration for the browser extension integration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionConfig {
  pub enabled: bool,
  pub token: String,
  pub port: u16,
}

impl ExtensionConfig {
  pub fn new() -> Self {
    Self {
      enabled: false,
      token: Uuid::new_v4().to_string(),
      port: EXTENSION_DEFAULT_PORT,
    }
  }
}

impl Default for ExtensionConfig {
  fn default() -> Self {
    Self::new()
  }
}

/// A password entry stored in the vault.
///
/// Each entry contains credentials for a single account or service.
/// Sensitive fields (especially `password`) are securely zeroized when
/// the entry is dropped, preventing them from lingering in memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
  /// Unique identifier (UUID v4).
  pub id: String,
  /// Display name for the entry.
  pub title: String,
  /// Username or email for the account.
  pub username: String,
  /// The secret password (zeroized on drop).
  pub password: String,
  /// URL of the service or website.
  pub url: String,
  /// Additional notes about the entry.
  pub notes: String,
  /// Timestamp when the entry was created.
  pub created_at: DateTime<Utc>,
  /// Timestamp of the last modification.
  pub updated_at: DateTime<Utc>,
}

impl Entry {
  /// Creates a new entry with a generated UUID and current timestamp.
  pub fn new(title: String, username: String, password: String, url: String, notes: String) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4().to_string(),
      title,
      username,
      password,
      url,
      notes,
      created_at: now,
      updated_at: now,
    }
  }

  /// Updates the `updated_at` timestamp to the current time.
  pub fn touch(&mut self) {
    self.updated_at = Utc::now();
  }
}

impl Zeroize for Entry {
  fn zeroize(&mut self) {
    self.id.zeroize();
    self.title.zeroize();
    self.username.zeroize();
    self.password.zeroize();
    self.url.zeroize();
    self.notes.zeroize();
  }
}

impl Drop for Entry {
  fn drop(&mut self) {
    self.zeroize();
  }
}

/// An active vault session containing the derived encryption key.
///
/// The session is created when the vault is unlocked and cleared when locked.
/// The master password is never stored; only the derived key is kept in memory,
/// wrapped in [`Zeroizing`] for secure cleanup on drop.
#[derive(Clone)]
pub struct VaultSession {
  /// Salt used for key derivation (stored in the vault file).
  pub salt: [u8; SALT_LEN],
  /// Derived 256-bit encryption key (zeroized on drop).
  pub key: Zeroizing<[u8; 32]>,
}

impl VaultSession {
  /// Creates a new vault session with the given salt and key.
  pub fn new(salt: [u8; SALT_LEN], key_bytes: [u8; 32]) -> Self {
    Self {
      salt,
      key: Zeroizing::new(key_bytes),
    }
  }

  /// Returns a reference to the encryption key as a fixed-size array.
  ///
  /// This method exists because calling `.as_ref()` on `Zeroizing<[u8; 32]>`
  /// returns `&[u8]` (a slice) rather than `&[u8; 32]` (a fixed-size array).
  #[inline]
  pub fn key_bytes(&self) -> &[u8; 32] {
    &self.key
  }
}

/// Tracks failed unlock attempts for rate limiting.
/// After MAX_FAILED_ATTEMPTS, enforces a cooldown period.
#[derive(Clone, Debug, Default)]
pub struct FailedAttemptTracker {
  pub count: u32,
  pub locked_until: Option<Instant>,
}

impl FailedAttemptTracker {
  /// Records a failed unlock attempt. Returns lockout duration if threshold exceeded.
  pub fn record_failure(&mut self) -> Option<u64> {
    self.count += 1;
    if self.count >= MAX_FAILED_ATTEMPTS {
      let lockout_until = Instant::now() + std::time::Duration::from_secs(LOCKOUT_DURATION_SECS);
      self.locked_until = Some(lockout_until);
      Some(LOCKOUT_DURATION_SECS)
    } else {
      None
    }
  }

  /// Checks if currently in lockout period. Returns remaining seconds if locked.
  ///
  /// If the lockout has expired, resets the tracker so the user gets
  /// a fresh set of attempts.
  pub fn check_lockout(&mut self) -> Option<u64> {
    if let Some(until) = self.locked_until {
      let now = Instant::now();
      if now < until {
        return Some(until.duration_since(now).as_secs());
      }
      self.count = 0;
      self.locked_until = None;
    }
    None
  }

  /// Resets the tracker after successful unlock.
  pub fn reset(&mut self) {
    self.count = 0;
    self.locked_until = None;
  }
}

/// Central application state shared across threads.
///
/// All fields are wrapped in `Arc<Mutex<>>` for thread-safe access.
/// The state is managed by Tauri and accessed via `State<AppState>` in commands.
#[derive(Clone)]
pub struct AppState {
  /// Unlocked entries (zeroized via `Entry::Drop` when cleared).
  pub entries: Arc<Mutex<Option<Vec<Entry>>>>,

  /// Active session with derived key (cleared on lock).
  pub session: Arc<Mutex<Option<VaultSession>>>,

  /// Timestamp of last user interaction (for auto-lock timeout).
  pub last_interaction: Arc<Mutex<Instant>>,

  /// Cached vault file path (resolved once on first access).
  pub vault_path: Arc<Mutex<Option<PathBuf>>>,

  /// Rate limiting tracker for failed unlock attempts.
  pub failed_attempts: Arc<Mutex<FailedAttemptTracker>>,

  /// Browser extension integration settings.
  pub extension_config: Arc<Mutex<ExtensionConfig>>,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      entries: Arc::new(Mutex::new(None)),
      session: Arc::new(Mutex::new(None)),
      last_interaction: Arc::new(Mutex::new(Instant::now())),
      vault_path: Arc::new(Mutex::new(None)),
      failed_attempts: Arc::new(Mutex::new(FailedAttemptTracker::default())),
      extension_config: Arc::new(Mutex::new(ExtensionConfig::default())),
    }
  }
}

impl AppState {
  /// Immediately locks the vault, clearing all sensitive data.
  ///
  /// Lock order: session → entries (prevents deadlocks).
  pub fn lock_now(&self) {
    if let Ok(mut s) = self.session.lock() {
      *s = None;
    }
    if let Ok(mut e) = self.entries.lock() {
      *e = None;
    }
    if let Ok(mut t) = self.last_interaction.lock() {
      *t = Instant::now();
    }
  }

  /// Updates the last interaction timestamp, resetting the auto-lock timer.
  pub fn heartbeat(&self) {
    if let Ok(mut t) = self.last_interaction.lock() {
      *t = Instant::now();
    }
  }
}
