import { invoke } from "@tauri-apps/api/core";
import { AppError, friendlyError } from "./errors";

export interface EntryPublic {
  id: string;
  title: string;
  username: string;
  url: string;
  notes: string;
  // RFC 3339 timestamps serialized by the Rust backend (chrono DateTime<Utc>).
  created_at: string;
  updated_at: string;
}

export interface EntryInput {
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
}

function isErrorWithMessage(value: unknown): value is { message: unknown } {
  return (
    typeof value === "object" &&
    value !== null &&
    "message" in value
  );
}

function asFriendlyError(e: unknown) {
  // Normalize any thrown value into a stable string for UI messaging.
  let raw: string;
  if (typeof e === "string") {
    raw = e;
  } else if (isErrorWithMessage(e)) {
    raw = String(e.message);
  } else {
    try {
      raw = JSON.stringify(e);
    } catch {
      raw = String(e);
    }
  }
  return friendlyError(raw);
}

async function invokeCommand<T = void>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const friendly = asFriendlyError(e);
    throw new AppError(friendly.message, friendly.lockoutSeconds);
  }
}

function aliasPasswordArgs(value: string, camelKey: string, snakeKey: string) {
  return {
    [camelKey]: value,
    [snakeKey]: value
  };
}

export async function heartbeat(): Promise<void> {
  await invokeCommand("heartbeat");
}

export async function lockVault(): Promise<void> {
  await invokeCommand("lock_vault");
}

// Tauri arg key mapping varies across templates, so we send both aliases.
export async function createVault(masterPassword: string): Promise<void> {
  await invokeCommand(
    "create_vault",
    aliasPasswordArgs(masterPassword, "masterPassword", "master_password")
  );
}

export async function unlockVault(masterPassword: string): Promise<void> {
  await invokeCommand(
    "unlock_vault",
    aliasPasswordArgs(masterPassword, "masterPassword", "master_password")
  );
}

export async function changeMasterPassword(
  currentPassword: string,
  newPassword: string
): Promise<void> {
  await invokeCommand("change_master_password", {
    ...aliasPasswordArgs(currentPassword, "currentPassword", "current_password"),
    ...aliasPasswordArgs(newPassword, "newPassword", "new_password")
  });
}

export async function exportVault(path: string): Promise<void> {
  await invokeCommand("export_vault", { path });
}

export async function importVault(path: string, masterPassword: string): Promise<void> {
  await invokeCommand("import_vault", {
    path,
    ...aliasPasswordArgs(masterPassword, "masterPassword", "master_password")
  });
}

export async function getEntries(): Promise<EntryPublic[]> {
  return await invokeCommand<EntryPublic[]>("get_entries");
}

export interface EntryUpdateInput {
  id: string;
  title: string;
  username: string;
  password?: string;
  url: string;
  notes: string;
}

export interface ExtensionConfig {
  enabled: boolean;
  token: string;
  port: number;
}

export async function addEntry(input: EntryInput): Promise<EntryPublic> {
  return await invokeCommand<EntryPublic>("add_entry", { input });
}

export async function updateEntry(input: EntryUpdateInput): Promise<EntryPublic> {
  return await invokeCommand<EntryPublic>("update_entry", { input });
}

export async function deleteEntry(id: string): Promise<void> {
  await invokeCommand("delete_entry", { id });
}

export async function copySecret(id: string): Promise<void> {
  await invokeCommand("copy_secret", { id });
}

export async function getExtensionConfig(): Promise<ExtensionConfig> {
  return await invokeCommand<ExtensionConfig>("get_extension_config");
}

export async function setExtensionEnabled(enabled: boolean): Promise<ExtensionConfig> {
  return await invokeCommand<ExtensionConfig>("set_extension_enabled", { enabled });
}

export async function rotateExtensionToken(): Promise<ExtensionConfig> {
  return await invokeCommand<ExtensionConfig>("rotate_extension_token");
}
