<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import EntryModal from "./EntryModal.svelte";
  import EntryRow from "./EntryRow.svelte";
  import {
    addEntry,
    changeMasterPassword,
    copySecret,
    deleteEntry,
    exportVault,
    getEntries,
    heartbeat,
    importVault,
    updateEntry
  } from "../lib/api";
  import { entries, setError } from "../lib/stores";
  import type { EntryInput, EntryPublic, EntryUpdateInput } from "../lib/api";

  export let onHeartbeat: () => void;
  export let onLocked: () => void;

  let showModal = false;
  let editingEntry: EntryPublic | null = null;
  let busy = false;
  let toast: string | null = null;
  let expandedId: string | null = null;
  let sortMode: "most-used" | "recent-updated" | "recent-created" | "title" = "most-used";
  let currentPassword = "";
  let newPassword = "";
  let confirmNewPassword = "";
  let exportPath = "";
  let importPath = "";
  let importPassword = "";
  let auditLog: { id: string; action: string; detail: string; at: number }[] = [];
  let sortEpoch = 0;

  type InteractionStats = {
    clicked: number;
    managed: number;
    copied: number;
    last: number;
  };

  const interactionStats = new Map<string, InteractionStats>();

  function toErrorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  async function runWithBusy(task: () => Promise<void>) {
    if (busy) return;
    busy = true;
    try {
      onHeartbeat();
      await task();
    } catch (e) {
      setError(toErrorMessage(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  function showToast(msg: string) {
    toast = msg;
    window.setTimeout(() => {
      toast = null;
    }, 2500);
  }

  async function refresh() {
    try {
      const list = await getEntries();
      entries.set(list);
      const ids = new Set(list.map((entry) => entry.id));
      for (const id of interactionStats.keys()) {
        if (!ids.has(id)) interactionStats.delete(id);
      }
      if (expandedId && !ids.has(expandedId)) {
        expandedId = null;
      }
      setError(null);
    } catch {
      onLocked();
    }
  }

  onMount(() => {
    refresh();
  });

  function getStats(id: string): InteractionStats {
    const existing = interactionStats.get(id);
    if (existing) return existing;
    const fresh = { clicked: 0, managed: 0, copied: 0, last: 0 };
    interactionStats.set(id, fresh);
    return fresh;
  }

  function recordInteraction(
    id: string,
    type: "clicked" | "managed" | "copied"
  ) {
    const stats = getStats(id);
    stats[type] += 1;
    stats.last = Date.now();
  }

  function getUsageScore(id: string) {
    const stats = interactionStats.get(id);
    if (!stats) return 0;
    return stats.clicked + stats.managed + stats.copied;
  }

  function getSortKey(entry: EntryPublic, key: "updated_at" | "created_at") {
    const ts = Date.parse(entry[key]);
    return Number.isNaN(ts) ? 0 : ts;
  }

  function sortEntries(list: EntryPublic[], _tick = 0) {
    const sorted = [...list];
    sorted.sort((a, b) => {
      if (sortMode === "title") {
        return a.title.localeCompare(b.title);
      }
      if (sortMode === "recent-updated") {
        return getSortKey(b, "updated_at") - getSortKey(a, "updated_at");
      }
      if (sortMode === "recent-created") {
        return getSortKey(b, "created_at") - getSortKey(a, "created_at");
      }
      const scoreDiff = getUsageScore(b.id) - getUsageScore(a.id);
      if (scoreDiff !== 0) return scoreDiff;
      const lastDiff = getStats(b.id).last - getStats(a.id).last;
      if (lastDiff !== 0) return lastDiff;
      return getSortKey(b, "updated_at") - getSortKey(a, "updated_at");
    });
    return sorted;
  }

  function recordAudit(action: string, detail: string) {
    const item = { id: crypto.randomUUID(), action, detail, at: Date.now() };
    auditLog = [item, ...auditLog].slice(0, 8);
    window.setTimeout(() => {
      auditLog = auditLog.filter((entry) => entry.id !== item.id);
    }, 10_000);
  }

  function formatTimestamp(ts: number) {
    return new Date(ts).toLocaleString();
  }

  function getEntryTitle(id: string) {
    const list = get(entries);
    return list.find((entry) => entry.id === id)?.title ?? "Entry";
  }

  $: visible = sortEntries($entries, sortEpoch);

  async function doCreate(input: EntryInput) {
    await runWithBusy(async () => {
      await addEntry(input);
      await refresh();
      recordAudit("Entry created", input.title.trim());
      sortEpoch += 1;
      showToast("Saved.");
    });
  }

  async function doUpdate(input: EntryUpdateInput) {
    await runWithBusy(async () => {
      await updateEntry(input);
      await refresh();
      recordInteraction(input.id, "managed");
      recordAudit("Entry updated", input.title.trim());
      sortEpoch += 1;
      showToast("Updated.");
      editingEntry = null;
    });
  }

  async function doDelete(id: string) {
    await runWithBusy(async () => {
      await deleteEntry(id);
      if (expandedId === id) expandedId = null;
      recordInteraction(id, "managed");
      recordAudit("Entry deleted", getEntryTitle(id));
      await refresh();
      sortEpoch += 1;
      showToast("Deleted.");
    });
  }

  async function doCopy(id: string) {
    await runWithBusy(async () => {
      await copySecret(id);
      recordInteraction(id, "copied");
      recordAudit("Password copied", getEntryTitle(id));
      showToast("Copied to clipboard (clears in 15s).");
    });
  }

  async function doChangeMasterPassword() {
    setError(null);
    if (!currentPassword) {
      setError("Enter your current master password.");
      return;
    }
    if (!newPassword) {
      setError("Enter a new master password.");
      return;
    }
    if (newPassword.length < 10) {
      setError("New master password must be at least 10 characters.");
      return;
    }
    if (newPassword !== confirmNewPassword) {
      setError("New master passwords do not match.");
      return;
    }

    await runWithBusy(async () => {
      await changeMasterPassword(currentPassword, newPassword);
      recordAudit("Master password updated", "Security settings updated.");
      showToast("Master password updated.");
      setError(null);
    });
    currentPassword = "";
    newPassword = "";
    confirmNewPassword = "";
  }

  async function doExport() {
    if (!exportPath.trim()) {
      setError("Enter a file path for the encrypted backup.");
      return;
    }

    const trimmedPath = exportPath.trim();
    await runWithBusy(async () => {
      await exportVault(trimmedPath);
      recordAudit("Backup exported", trimmedPath);
      showToast("Encrypted backup exported.");
      setError(null);
    });
  }

  async function doImport() {
    if (!importPath.trim()) {
      setError("Enter a backup file path to import.");
      return;
    }
    if (!importPassword) {
      setError("Enter the master password for the backup.");
      return;
    }

    const auditPath = importPath.trim();
    await runWithBusy(async () => {
      await importVault(auditPath, importPassword);
      importPath = "";
      importPassword = "";
      await refresh();
      recordAudit("Backup imported", auditPath);
      showToast("Encrypted backup imported.");
      setError(null);
    });
    importPassword = "";
  }

  async function browseExportPath() {
    try {
      const selected = await save({
        filters: [{ name: "Vault Backup", extensions: ["vault"] }],
        defaultPath: "backup.vault"
      });
      if (selected) {
        exportPath = selected;
        onHeartbeat();
      }
    } catch (e) {
      setError(toErrorMessage(e));
    }
  }

  async function browseImportPath() {
    try {
      const selected = await open({
        filters: [{ name: "Vault Backup", extensions: ["vault"] }],
        multiple: false
      });
      if (selected && typeof selected === "string") {
        importPath = selected;
        onHeartbeat();
      }
    } catch (e) {
      setError(toErrorMessage(e));
    }
  }
</script>

<div class="space-y-4">
  <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
    <div>
      <div class="text-lg font-semibold">Entries</div>
      <div class="text-sm text-neutral-400">
        Sort by most used (clicked, managed, copied), recency, or title.
      </div>
      {#if $entries.length > 0}
        <div class="mt-1 text-xs text-neutral-500">
          Showing {visible.length} of {$entries.length}
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <div class="flex flex-col gap-1 md:flex-row md:items-center">
        <label class="text-xs text-neutral-500">Sort by</label>
        <select
          class="rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600 disabled:opacity-50"
          bind:value={sortMode}
          on:change={() => onHeartbeat()}
          disabled={$entries.length === 0}
        >
          <option value="most-used">Most used</option>
          <option value="recent-updated">Recently updated</option>
          <option value="recent-created">Recently created</option>
          <option value="title">Title A–Z</option>
        </select>
      </div>
      <button
        class="rounded-xl bg-neutral-100 px-3 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
        on:click={() => {
          onHeartbeat();
          showModal = true;
        }}
        disabled={busy}
      >
        Add
      </button>
    </div>
  </div>

  {#if toast}
    <div class="rounded-xl border border-neutral-800 bg-neutral-900/40 px-4 py-3 text-sm text-neutral-200">
      {toast}
    </div>
  {/if}

  {#if visible.length === 0}
    <div class="rounded-2xl border border-neutral-800 bg-neutral-900/20 p-6 text-sm text-neutral-400">
      {#if $entries.length === 0}
        No entries yet. Click "Add" to create your first entry.
      {/if}
    </div>
  {:else}
    <div class="overflow-hidden rounded-2xl border border-neutral-800">
      <div class="divide-y divide-neutral-800">
        {#each visible as e (e.id)}
          <EntryRow
            entry={e}
            busy={busy}
            expanded={expandedId === e.id}
            onHeartbeat={onHeartbeat}
            onToggle={() => {
              recordInteraction(e.id, "clicked");
              expandedId = expandedId === e.id ? null : e.id;
            }}
            onCopy={() => doCopy(e.id)}
            onModify={() => {
              recordInteraction(e.id, "managed");
              editingEntry = e;
            }}
            onDelete={() => doDelete(e.id)}
          />
        {/each}
      </div>
    </div>
  {/if}

  <div class="rounded-2xl border border-neutral-800 bg-neutral-900/20 p-6">
    <div class="text-lg font-semibold">Session activity</div>
    <div class="mt-1 text-sm text-neutral-400">
      Recent actions from this session (sorted by most recent).
    </div>

    {#if auditLog.length === 0}
      <div class="mt-4 rounded-xl border border-neutral-800 bg-neutral-950/40 px-4 py-3 text-sm text-neutral-400">
        No activity yet. Actions like copy, update, export, and import will appear here.
      </div>
    {:else}
      <div class="mt-4 space-y-2">
        {#each auditLog as item (item.id)}
          <div class="rounded-xl border border-neutral-800 bg-neutral-950/40 px-4 py-3 text-sm">
            <div class="flex flex-wrap items-center justify-between gap-2 text-neutral-200">
              <div class="font-semibold">{item.action}</div>
              <div class="text-xs text-neutral-500">{formatTimestamp(item.at)}</div>
            </div>
            <div class="mt-1 text-xs text-neutral-400">{item.detail}</div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="rounded-2xl border border-neutral-800 bg-neutral-900/20 p-6">
    <div class="text-lg font-semibold">Security &amp; backups</div>
    <div class="mt-1 text-sm text-neutral-400">
      Change the master password or export/import an encrypted backup. Export uses your current master
      password; imports require the master password that encrypted the backup.
    </div>

    <div class="mt-5 grid gap-6 lg:grid-cols-2">
      <div class="space-y-3 rounded-2xl border border-neutral-800 bg-neutral-950/40 p-4">
        <div class="text-sm font-semibold text-neutral-200">Change master password</div>
        <div>
          <div class="mb-1 text-xs text-neutral-400">Current master password</div>
            <input
              class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
              type="password"
              autocomplete="current-password"
              bind:value={currentPassword}
              on:input={() => setError(null)}
            />
          </div>
          <div class="grid gap-3 sm:grid-cols-2">
            <div>
              <div class="mb-1 text-xs text-neutral-400">New master password</div>
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                type="password"
                autocomplete="new-password"
                bind:value={newPassword}
                on:input={() => setError(null)}
              />
            </div>
            <div>
              <div class="mb-1 text-xs text-neutral-400">Confirm new password</div>
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                type="password"
                autocomplete="new-password"
                bind:value={confirmNewPassword}
                on:input={() => setError(null)}
              />
            </div>
          </div>
        <button
          class="rounded-xl bg-neutral-100 px-3 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
          on:click={doChangeMasterPassword}
          disabled={busy}
        >
          Update password
        </button>
      </div>

      <div class="space-y-5 rounded-2xl border border-neutral-800 bg-neutral-950/40 p-4">
        <div>
          <div class="text-sm font-semibold text-neutral-200">Export encrypted backup</div>
          <div class="mt-2 space-y-2">
            <div>
              <div class="mb-1 text-xs text-neutral-400">Export file path</div>
              <div class="flex gap-2">
                <input
                  class="flex-1 rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                  placeholder="/path/to/backup.vault"
                  bind:value={exportPath}
                />
                <button
                  class="rounded-xl border border-neutral-800 px-3 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
                  on:click={browseExportPath}
                  disabled={busy}
                  type="button"
                >
                  Browse
                </button>
              </div>
            </div>
            <button
              class="rounded-xl border border-neutral-800 px-3 py-2 text-sm text-neutral-100 hover:bg-neutral-900 disabled:opacity-50"
              on:click={doExport}
              disabled={busy}
            >
              Export backup
            </button>
          </div>
        </div>

        <div>
          <div class="text-sm font-semibold text-neutral-200">Import encrypted backup</div>
          <div class="mt-2 space-y-2">
            <div>
              <div class="mb-1 text-xs text-neutral-400">Backup file path</div>
              <div class="flex gap-2">
                <input
                  class="flex-1 rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                  placeholder="/path/to/backup.vault"
                  bind:value={importPath}
                />
                <button
                  class="rounded-xl border border-neutral-800 px-3 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
                  on:click={browseImportPath}
                  disabled={busy}
                  type="button"
                >
                  Browse
                </button>
              </div>
            </div>
            <div>
              <div class="mb-1 text-xs text-neutral-400">Master password for this backup</div>
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                type="password"
                autocomplete="current-password"
                bind:value={importPassword}
              />
              <div class="mt-1 text-xs text-neutral-500">
                Enter the master password used when the backup was created (it may differ from your current one).
              </div>
            </div>
            <button
              class="rounded-xl border border-neutral-800 px-3 py-2 text-sm text-neutral-100 hover:bg-neutral-900 disabled:opacity-50"
              on:click={doImport}
              disabled={busy}
            >
              Import backup
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

{#if showModal}
  <EntryModal
    onCancel={() => {
      showModal = false;
    }}
    onCreate={doCreate}
  />
{/if}

{#if editingEntry}
  <EntryModal
    existingEntry={editingEntry}
    onCancel={() => {
      editingEntry = null;
    }}
    onUpdate={doUpdate}
  />
{/if}
