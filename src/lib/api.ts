import { invoke } from "@tauri-apps/api/core";
import { friendlyError } from "./errors";

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

function asStringError(e: unknown): string {
  // Normalize any thrown value into a stable string for UI messaging.
  let raw: string;
  if (typeof e === "string") {
    raw = e;
  } else if (e && typeof e === "object" && "message" in e) {
    raw = String((e as any).message);
  } else {
    try {
      raw = JSON.stringify(e);
    } catch {
      raw = String(e);
    }
  }
  return friendlyError(raw);
}

export async function heartbeat(): Promise<void> {
  await invoke("heartbeat");
}

export async function lockVault(): Promise<void> {
  await invoke("lock_vault");
}

/*
Tauri arg key mapping can be confusing across versions/templates.
Your runtime error shows Tauri expects: masterPassword (camelCase).
To be robust, we send BOTH keys. Tauri will use the one it recognizes.
*/
export async function createVault(masterPassword: string): Promise<void> {
  try {
    await invoke("create_vault", {
      masterPassword,
      master_password: masterPassword
    });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function unlockVault(masterPassword: string): Promise<void> {
  try {
    await invoke("unlock_vault", {
      masterPassword,
      master_password: masterPassword
    });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function changeMasterPassword(
  currentPassword: string,
  newPassword: string
): Promise<void> {
  try {
    await invoke("change_master_password", {
      currentPassword,
      current_password: currentPassword,
      newPassword,
      new_password: newPassword
    });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function exportVault(path: string): Promise<void> {
  try {
    await invoke("export_vault", { path });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function importVault(path: string, masterPassword: string): Promise<void> {
  try {
    await invoke("import_vault", {
      path,
      masterPassword,
      master_password: masterPassword
    });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function getEntries(): Promise<EntryPublic[]> {
  try {
    return await invoke<EntryPublic[]>("get_entries");
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export interface EntryUpdateInput {
  id: string;
  title: string;
  username: string;
  password?: string;
  url: string;
  notes: string;
}

export async function addEntry(input: EntryInput): Promise<EntryPublic> {
  try {
    return await invoke<EntryPublic>("add_entry", { input });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function updateEntry(input: EntryUpdateInput): Promise<EntryPublic> {
  try {
    return await invoke<EntryPublic>("update_entry", { input });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function deleteEntry(id: string): Promise<void> {
  try {
    await invoke("delete_entry", { id });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function copySecret(id: string): Promise<void> {
  try {
    await invoke("copy_secret", { id });
  } catch (e) {
    throw new Error(asStringError(e));
  }
}
