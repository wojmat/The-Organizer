import { writable } from "svelte/store";
import type { EntryPublic } from "./api";

export type View = "login" | "setup" | "dashboard";

const DEFAULT_VIEW: View = "login";

// App-level view state derived from vault lifecycle (locked/setup/unlocked).
export const view = writable<View>(DEFAULT_VIEW);
export const isLocked = writable(true);
export const entries = writable<EntryPublic[]>([]);
export const lastError = writable<string | null>(null);

// Convenience helpers (optional)
export function setError(msg: string | null) {
  lastError.set(msg);
}
