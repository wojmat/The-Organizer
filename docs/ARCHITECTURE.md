# Architecture

This document summarizes how The Organizer is structured and how data moves through the system.

## High-Level Overview

The Organizer is a Tauri desktop application with a Svelte frontend and a Rust backend:

- **Frontend (Svelte 5 + TypeScript)**: Renders UI, collects user input, and invokes Tauri commands.
- **Backend (Rust)**: Performs encryption/decryption, manages vault state in memory, and writes the vault file.

```
UI (Svelte) → Tauri IPC → Rust commands → vault.dat (encrypted)
```

## Frontend Responsibilities

- Collects the master password during setup/unlock.
- Collects entry data on creation.
- Displays non-secret entry data returned from the backend.
- Sends periodic heartbeats based on user interaction to keep the session alive.

The frontend never receives decrypted passwords from the backend. Clipboard copy is performed by the backend.

## Backend Responsibilities

- Derives the encryption key using Argon2id.
- Encrypts/decrypts vault contents with XChaCha20-Poly1305.
- Stores unlocked entries and the derived key in memory while the session is active.
- Enforces lockout after repeated failed unlock attempts.
- Clears sensitive memory on lock (best-effort via `zeroize`).
- Re-encrypts the vault for master password changes and encrypted backup import/export.

## Vault File Format

The vault is stored as `vault.dat` in the app data directory.

```
[1 byte version][32 bytes salt][24 bytes nonce][ciphertext + auth tag]
```

The loader supports a legacy format without the version byte for backward compatibility.

## Session and Auto-Lock

When the user interacts with the UI, the frontend sends a heartbeat to the backend. The backend records the last interaction time and a background task checks for inactivity:

- **Poll interval**: every 10 seconds
- **Timeout**: 5 minutes

If the timeout is exceeded, the backend clears the session and entries.

## Rate Limiting

Failed unlock attempts are tracked in memory:

- **Threshold**: 5 failed attempts
- **Cooldown**: 30 seconds

This state resets on app restart.

## Clipboard Handling

When the user taps "Copy", the backend places the password on the clipboard and clears it after 15 seconds. If the app crashes before the cleanup thread runs, the clipboard may retain the password.
