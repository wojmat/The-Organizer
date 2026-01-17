import { invoke } from "@tauri-apps/api/core";

export interface EntryPublic {
  id: string;
  title: string;
  username: string;
  url: string;
  notes: string;
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
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as any).message);
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
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

export async function getEntries(): Promise<EntryPublic[]> {
  try {
    return await invoke<EntryPublic[]>("get_entries");
  } catch (e) {
    throw new Error(asStringError(e));
  }
}

export async function addEntry(input: EntryInput): Promise<EntryPublic> {
  try {
    return await invoke<EntryPublic>("add_entry", { input });
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
