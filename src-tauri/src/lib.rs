//! The Organizer - Rust backend library for the password manager.
//!
//! This crate provides the core functionality for The Organizer password manager:
//!
//! - [`commands`] - Tauri IPC command handlers for frontend communication
//! - [`models`] - Data structures and application state management
//! - [`vault`] - Encryption, decryption, and key derivation
//!
//! # Architecture
//!
//! ```text
//! Frontend (Svelte)
//!     │ Tauri invoke
//!     ▼
//! commands.rs ─── IPC handlers
//!     │
//!     ▼
//! vault.rs ────── Encryption/decryption
//!     │
//!     ▼
//! models.rs ───── Data structures
//!     │
//!     ▼
//! Disk ────────── vault.dat (encrypted)
//! ```
//!
//! # Security Features
//!
//! - **Argon2id** key derivation (64 MiB memory, 3 iterations)
//! - **XChaCha20-Poly1305** authenticated encryption
//! - **Zeroize** for secure memory cleanup
//! - **Auto-lock** after 5 minutes of inactivity
//! - **Rate limiting** on failed unlock attempts

use tauri::Wry;

pub mod commands;
pub mod extension;
pub mod models;
pub mod vault;

/// Creates the Tauri invoke handler with all registered commands.
///
/// This function must be defined in the library crate (where the commands are defined)
/// because Tauri's `#[tauri::command]` macro generates internal macros that are only
/// accessible within the same crate.
pub fn create_invoke_handler() -> impl Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::heartbeat,
        commands::lock_vault,
        commands::create_vault,
        commands::change_master_password,
        commands::unlock_vault,
        commands::get_entries,
        commands::add_entry,
        commands::update_entry,
        commands::delete_entry,
        commands::copy_secret,
        commands::export_vault,
        commands::import_vault,
        commands::get_extension_config,
        commands::set_extension_enabled,
        commands::rotate_extension_token
    ]
}
