/**
 * Tauri API bindings for The Organizer password manager.
 *
 * This module provides type-safe wrappers around Tauri IPC commands
 * for vault operations, entry management, and session control.
 *
 * @module api
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Public entry data (passwords excluded).
 * This is what the frontend receives - password field is never sent to UI.
 */
export interface EntryPublic {
  /** Unique entry identifier (UUID) */
  id: string;
  /** Entry title/name */
  title: string;
  /** Username/email for this entry */
  username: string;
  /** Optional URL associated with this entry */
  url: string;
  /** Optional notes/additional information */
  notes: string;
  /** ISO 8601 timestamp when entry was created */
  created_at: string;
  /** ISO 8601 timestamp when entry was last modified */
  updated_at: string;
}

/**
 * Input data for creating a new entry.
 * Includes the password field, which is only sent during creation.
 */
export interface EntryInput {
  /** Entry title/name (required) */
  title: string;
  /** Username/email */
  username: string;
  /** Password (encrypted by backend, never stored in frontend) */
  password: string;
  /** Optional URL */
  url: string;
  /** Optional notes */
  notes: string;
}

function asStringError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as any).message);
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

/**
 * Translates technical backend error messages into user-friendly explanations
 * with actionable recovery guidance.
 */
function friendlyErrorMessage(error: string): string {
  const lower = error.toLowerCase();

  // Rate limiting (check first as it's most specific)
  if (lower.includes("too many failed attempts")) {
    return error; // Already user-friendly message from backend
  }

  // Crypto/authentication errors
  if (lower.includes("crypto") || lower.includes("decrypt")) {
    return "Incorrect password. Please try again or create a new vault if you've forgotten your password.";
  }

  // Vault locked
  if (lower.includes("vault is locked")) {
    return "Your session has expired. Please unlock your vault to continue.";
  }

  // Mutex poisoning (internal error, user-friendly message)
  if (lower.includes("mutex poisoned")) {
    return "Internal state error. Please restart the application.";
  }

  // Vault already exists
  if (lower.includes("vault already exists")) {
    return "A vault already exists. Use the unlock screen to access it.";
  }

  // Vault doesn't exist
  if (lower.includes("vault does not exist")) {
    return "No vault found. Please create a new vault first.";
  }

  // Entry not found
  if (lower.includes("entry not found")) {
    return "The requested entry could not be found. It may have been deleted.";
  }

  // Clipboard errors
  if (lower.includes("clipboard")) {
    return "Unable to access clipboard. Please check your system permissions.";
  }

  // KDF errors
  if (lower.includes("kdf") || lower.includes("argon2")) {
    return "Password processing failed. This may be a system resource issue.";
  }

  // File I/O errors
  if (lower.includes("io") || lower.includes("file") || lower.includes("permission")) {
    return "Unable to access vault file. Please check file permissions and disk space.";
  }

  // Default: return original error if no match
  return error;
}

/**
 * Sends a heartbeat signal to update the last interaction timestamp.
 * This prevents auto-lock timeout while the user is actively using the app.
 *
 * @throws Never throws - heartbeat failures are silently ignored
 */
export async function heartbeat(): Promise<void> {
  await invoke("heartbeat");
}

/**
 * Manually locks the vault, clearing session data and requiring re-authentication.
 * Sensitive data (keys, entries) is zeroized from memory.
 *
 * @throws Error if lock operation fails
 */
export async function lockVault(): Promise<void> {
  await invoke("lock_vault");
}

/**
 * Creates a new encrypted vault with the provided master password.
 * Generates a random salt and derives an encryption key using Argon2id.
 *
 * @param masterPassword - Master password (min 10 characters recommended)
 * @throws Error if vault already exists or creation fails
 */
export async function createVault(masterPassword: string): Promise<void> {
  try {
    await invoke("create_vault", { master_password: masterPassword });
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}

/**
 * Unlocks an existing vault with the master password.
 * Decrypts the vault and loads entries into memory.
 *
 * Rate limiting: After 5 failed attempts, enforces a 30-second cooldown.
 *
 * @param masterPassword - Master password for vault decryption
 * @throws Error if password is incorrect, vault doesn't exist, or rate limit exceeded
 */
export async function unlockVault(masterPassword: string): Promise<void> {
  try {
    await invoke("unlock_vault", { master_password: masterPassword });
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}

/**
 * Retrieves all entries from the unlocked vault.
 * Passwords are NOT included in the response (backend security measure).
 *
 * @returns Array of entries with passwords excluded
 * @throws Error if vault is locked
 */
export async function getEntries(): Promise<EntryPublic[]> {
  try {
    return await invoke<EntryPublic[]>("get_entries");
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}

/**
 * Adds a new password entry to the vault.
 * The password is encrypted and saved to disk immediately.
 *
 * @param input - Entry data including password (only sent during creation)
 * @returns The created entry (without password field)
 * @throws Error if vault is locked or save fails
 */
export async function addEntry(input: EntryInput): Promise<EntryPublic> {
  try {
    return await invoke<EntryPublic>("add_entry", { input });
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}

/**
 * Permanently deletes an entry from the vault.
 * This operation cannot be undone.
 *
 * @param id - UUID of the entry to delete
 * @throws Error if vault is locked or entry not found
 */
export async function deleteEntry(id: string): Promise<void> {
  try {
    await invoke("delete_entry", { id });
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}

/**
 * Copies an entry's password to the system clipboard.
 * The clipboard is automatically cleared after 15 seconds.
 *
 * Security note: If the app crashes before cleanup, the password remains in clipboard.
 *
 * @param id - UUID of the entry whose password to copy
 * @throws Error if vault is locked, entry not found, or clipboard access fails
 */
export async function copySecret(id: string): Promise<void> {
  try {
    await invoke("copy_secret", { id });
  } catch (e) {
    throw new Error(friendlyErrorMessage(asStringError(e)));
  }
}
