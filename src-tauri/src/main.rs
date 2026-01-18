//! The Organizer - A secure, local password manager.
//!
//! This is the main entry point for the Tauri application. It:
//! - Initializes the application state
//! - Registers all IPC command handlers
//! - Starts the inactivity monitor for auto-lock functionality
//!
//! # Auto-Lock
//!
//! A background thread monitors user inactivity. If the vault is unlocked
//! and no user interaction occurs for 5 minutes, the vault is automatically
//! locked to protect sensitive data.

use std::thread;
use std::time::{Duration, Instant};
use tauri::Manager;

use the_organizer::create_invoke_handler;
use the_organizer::extension;
use the_organizer::models::{AppState, INACTIVITY_POLL_SECS, INACTIVITY_TIMEOUT_SECS};

fn main() {
  let builder = tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(AppState::default())
    .invoke_handler(create_invoke_handler())
    .setup(|app| {
      let state: AppState = app.state::<AppState>().inner().clone();
      let poll = Duration::from_secs(INACTIVITY_POLL_SECS);
      let timeout = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);

      match extension::load_or_create_config(&app.handle()) {
        Ok(config) => {
          if let Ok(mut guard) = state.extension_config.lock() {
            *guard = config;
          }
        }
        Err(err) => {
          eprintln!("extension config load failed: {err}");
        }
      }
      extension::start_extension_server(&app.handle(), state.clone());

      thread::spawn(move || loop {
        thread::sleep(poll);

        let last = match state.last_interaction.lock() {
          Ok(g) => *g,
          Err(_) => {
            // Poisoned mutex: safest behavior is to lock.
            state.lock_now();
            continue;
          }
        };

        let is_unlocked = match state.session.lock() {
          Ok(g) => g.is_some(),
          Err(_) => {
            state.lock_now();
            continue;
          }
        };

        if is_unlocked && Instant::now().duration_since(last) > timeout {
          state.lock_now();
        }
      });

      Ok(())
    });

  // Do not unwrap/expect.
  let result = builder.run(tauri::generate_context!());
  if let Err(e) = result {
    eprintln!("tauri run error: {e}");
  }
}
