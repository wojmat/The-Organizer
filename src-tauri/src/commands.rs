//! Tauri command handlers for The Organizer password manager.
//!
//! This module implements all IPC commands that the frontend can invoke:
//! - `create_vault` / `unlock_vault` / `lock_vault` - Session management
//! - `get_entries` / `add_entry` / `delete_entry` - Entry CRUD operations
//! - `copy_secret` - Secure clipboard operations with auto-clear
//! - `heartbeat` - Activity tracking for auto-lock timeout
//!
//! # Security Notes
//!
//! - Master passwords are wrapped in `Zeroizing<String>` for secure memory handling
//! - Entry passwords are never sent to the frontend (only IDs for clipboard operations)
//! - The vault key is stored in `VaultSession` and cleared on lock
//! - All mutex access follows lock order: session → entries (prevents deadlocks)

use crate::models::{AppState, Entry, VaultSession, VAULT_FILENAME};
use crate::vault;
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use zeroize::{Zeroize, Zeroizing};

/// Resolves the path to the vault file, caching it for subsequent calls.
///
/// The path is constructed from the Tauri app data directory joined with
/// the vault filename. Once resolved, the path is cached in `AppState`
/// to ensure all commands use the same path.
fn resolve_vault_path(app: &AppHandle, state: &AppState) -> Result<PathBuf, String> {
  // Cache the path so commands are consistent.
  if let Ok(guard) = state.vault_path.lock() {
    if let Some(p) = guard.clone() {
      return Ok(p);
    }
  }

  let dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("app_data_dir failed: {e}"))?;

  fs::create_dir_all(&dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
  let path = dir.join(VAULT_FILENAME);

  if let Ok(mut guard) = state.vault_path.lock() {
    *guard = Some(path.clone());
  }

  Ok(path)
}

/// Input data for creating a new password entry.
///
/// This struct is deserialized from the frontend when adding a new entry.
#[derive(Clone, Debug, Deserialize)]
pub struct EntryInput {
  pub title: String,
  pub username: String,
  pub password: String,
  pub url: String,
  pub notes: String,
}

/// Public representation of a password entry sent to the frontend.
///
/// This struct intentionally excludes the `password` field to prevent
/// accidental exposure. The frontend uses `copy_secret` to copy passwords
/// to the clipboard without ever receiving the actual password value.
#[derive(Clone, Debug, Serialize)]
pub struct EntryPublic {
  pub id: String,
  pub title: String,
  pub username: String,
  pub url: String,
  pub notes: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Entry> for EntryPublic {
  fn from(e: &Entry) -> Self {
    Self {
      id: e.id.clone(),
      title: e.title.clone(),
      username: e.username.clone(),
      url: e.url.clone(),
      notes: e.notes.clone(),
      created_at: e.created_at,
      updated_at: e.updated_at,
    }
  }
}

/// Executes a closure with access to both entries and session while the vault is unlocked.
///
/// This helper ensures consistent lock ordering (session → entries) to prevent deadlocks.
/// The closure receives mutable access to entries and immutable access to the session.
///
/// # Errors
///
/// Returns an error if:
/// - Either mutex is poisoned
/// - The vault is locked (session or entries is `None`)
fn with_unlocked<R>(
  state: &AppState,
  f: impl FnOnce(&mut Vec<Entry>, &VaultSession) -> Result<R, String>,
) -> Result<R, String> {
  let session_guard = state
    .session
    .lock()
    .map_err(|_| "session mutex poisoned".to_string())?;
  let session = session_guard.as_ref().ok_or_else(|| "vault is locked".to_string())?;

  let mut entries_guard = state
    .entries
    .lock()
    .map_err(|_| "entries mutex poisoned".to_string())?;
  let entries = entries_guard.as_mut().ok_or_else(|| "vault is locked".to_string())?;

  f(entries, session)
}

#[tauri::command]
pub fn heartbeat(state: State<'_, AppState>) -> Result<(), String> {
  state.heartbeat();
  Ok(())
}

#[tauri::command]
pub fn lock_vault(state: State<'_, AppState>) -> Result<(), String> {
  state.lock_now();
  Ok(())
}

#[tauri::command]
pub fn create_vault(app: AppHandle, state: State<'_, AppState>, master_password: String) -> Result<(), String> {
  let master = Zeroizing::new(master_password);

  let path = resolve_vault_path(&app, state.inner())?;
  if path.exists() {
    return Err("vault already exists".to_string());
  }

  let salt = vault::generate_salt();
  let key = vault::derive_key(master.as_str(), &salt).map_err(|e| format!("kdf: {:?}", e))?;

  let entries: Vec<Entry> = Vec::new();
  vault::save_with_key(&path, &entries, &salt, &key).map_err(|e| format!("save: {:?}", e))?;

  // Lock order: session then entries.
  {
    let mut s = state.session.lock().map_err(|_| "session mutex poisoned".to_string())?;
    *s = Some(VaultSession::new(salt, key));
  }
  {
    let mut e = state.entries.lock().map_err(|_| "entries mutex poisoned".to_string())?;
    *e = Some(entries);
  }

  state.heartbeat();
  Ok(())
}

#[tauri::command]
pub fn unlock_vault(app: AppHandle, state: State<'_, AppState>, master_password: String) -> Result<(), String> {
  // Check rate limiting before attempting unlock
  {
    let tracker = state.failed_attempts.lock().map_err(|_| "rate limit mutex poisoned".to_string())?;
    if let Some(remaining_secs) = tracker.check_lockout() {
      return Err(format!(
        "Too many failed attempts. Please wait {} seconds before trying again.",
        remaining_secs
      ));
    }
  }

  let master = Zeroizing::new(master_password);

  let path = resolve_vault_path(&app, state.inner())?;
  if !path.exists() {
    return Err("vault does not exist".to_string());
  }

  // Attempt to decrypt vault
  let result = vault::load_with_password(&path, master.as_str());

  match result {
    Ok((entries, salt, key)) => {
      // Successful unlock - reset failed attempt counter
      {
        let mut tracker = state.failed_attempts.lock().map_err(|_| "rate limit mutex poisoned".to_string())?;
        tracker.reset();
      }

      // Lock order: session then entries.
      {
        let mut s = state.session.lock().map_err(|_| "session mutex poisoned".to_string())?;
        *s = Some(VaultSession::new(salt, key));
      }
      {
        let mut e = state.entries.lock().map_err(|_| "entries mutex poisoned".to_string())?;
        *e = Some(entries);
      }

      state.heartbeat();
      Ok(())
    }
    Err(e) => {
      // Failed unlock - record attempt
      let lockout_msg = {
        let mut tracker = state.failed_attempts.lock().map_err(|_| "rate limit mutex poisoned".to_string())?;
        tracker.record_failure().map(|duration| {
          format!(
            " Too many failed attempts. Account locked for {} seconds.",
            duration
          )
        })
      };

      let error_msg = format!("load: {:?}", e);
      if let Some(lockout) = lockout_msg {
        Err(format!("{}{}", error_msg, lockout))
      } else {
        Err(error_msg)
      }
    }
  }
}

#[tauri::command]
pub fn get_entries(state: State<'_, AppState>) -> Result<Vec<EntryPublic>, String> {
  state.heartbeat();

  let entries_guard = state
    .entries
    .lock()
    .map_err(|_| "entries mutex poisoned".to_string())?;

  let entries = entries_guard.as_ref().ok_or_else(|| "vault is locked".to_string())?;
  Ok(entries.iter().map(EntryPublic::from).collect())
}

#[tauri::command]
pub fn add_entry(app: AppHandle, state: State<'_, AppState>, input: EntryInput) -> Result<EntryPublic, String> {
  state.heartbeat();
  let path = resolve_vault_path(&app, state.inner())?;

  with_unlocked(state.inner(), |entries, session| {
    let mut entry = Entry::new(input.title, input.username, input.password, input.url, input.notes);
    entry.touch();
    entries.push(entry);

    vault::save_with_key(&path, entries, &session.salt, session.key_bytes())
      .map_err(|e| format!("save: {:?}", e))?;

    let last = entries.last().ok_or_else(|| "failed to add entry".to_string())?;
    Ok(EntryPublic::from(last))
  })
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.heartbeat();
  let path = resolve_vault_path(&app, state.inner())?;

  with_unlocked(state.inner(), |entries, session| {
    let before = entries.len();
    entries.retain(|e| e.id != id);
    let after = entries.len();

    if before == after {
      return Err("entry not found".to_string());
    }

    vault::save_with_key(&path, entries, &session.salt, session.key_bytes())
      .map_err(|e| format!("save: {:?}", e))?;

    Ok(())
  })
}

#[tauri::command]
pub fn copy_secret(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.heartbeat();

  // Grab password while holding lock, then drop lock quickly.
  let password = {
    let entries_guard = state
      .entries
      .lock()
      .map_err(|_| "entries mutex poisoned".to_string())?;

    let entries = entries_guard.as_ref().ok_or_else(|| "vault is locked".to_string())?;
    let entry = entries.iter().find(|e| e.id == id).ok_or_else(|| "entry not found".to_string())?;
    entry.password.clone()
  };

  let mut clipboard = Clipboard::new().map_err(|e| format!("clipboard init failed: {e}"))?;
  clipboard
    .set_text(password.clone())
    .map_err(|e| format!("clipboard set failed: {e}"))?;

  // Clear clipboard after 15 seconds for improved security.
  // Note: If the app crashes before this thread runs, the password will remain in the clipboard.
  // This is a known limitation of cross-platform clipboard management.
  thread::spawn(move || {
    thread::sleep(Duration::from_secs(15));
    if let Ok(mut cb) = Clipboard::new() {
      let _ = cb.set_text("".to_string());
    }
    let mut p = password;
    p.zeroize();
  });

  Ok(())
}
