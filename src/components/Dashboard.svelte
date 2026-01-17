<script lang="ts">
  import { onMount } from "svelte";
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
    importVault
  } from "../lib/api";
  import { entries, setError } from "../lib/stores";
  import type { EntryInput, EntryPublic } from "../lib/api";

  export let onHeartbeat: () => void;
  export let onLocked: () => void;

  let showModal = false;
  let busy = false;
  let toast: string | null = null;
  let q = "";
  let expandedId: string | null = null;
  let currentPassword = "";
  let newPassword = "";
  let confirmNewPassword = "";
  let exportPath = "";
  let importPath = "";
  let importPassword = "";

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
      setError(null);
    } catch {
      onLocked();
    }
  }

  onMount(() => {
    refresh();
  });

  function filterList(list: EntryPublic[]) {
    const qq = q.trim().toLowerCase();
    if (!qq) return list;

    return list.filter((e) => {
      return (
        e.title.toLowerCase().includes(qq) ||
        e.username.toLowerCase().includes(qq) ||
        (e.url || "").toLowerCase().includes(qq) ||
        (e.notes || "").toLowerCase().includes(qq)
      );
    });
  }

  $: visible = filterList($entries);

  async function doCreate(input: EntryInput) {
    busy = true;
    try {
      onHeartbeat();
      await addEntry(input);
      await refresh();
      showToast("Saved.");
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  async function doDelete(id: string) {
    if (busy) return;
    busy = true;
    try {
      onHeartbeat();
      await deleteEntry(id);
      if (expandedId === id) expandedId = null;
      await refresh();
      showToast("Deleted.");
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  async function doCopy(id: string) {
    if (busy) return;
    busy = true;
    try {
      onHeartbeat();
      await copySecret(id);
      showToast("Copied to clipboard (clears in 30s).");
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  async function doChangeMasterPassword() {
    if (busy) return;
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

    busy = true;
    try {
      onHeartbeat();
      await changeMasterPassword(currentPassword, newPassword);
      currentPassword = "";
      newPassword = "";
      confirmNewPassword = "";
      showToast("Master password updated.");
      setError(null);
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  async function doExport() {
    if (busy) return;
    if (!exportPath.trim()) {
      setError("Enter a file path for the encrypted backup.");
      return;
    }

    busy = true;
    try {
      onHeartbeat();
      await exportVault(exportPath.trim());
      showToast("Encrypted backup exported.");
      setError(null);
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  async function doImport() {
    if (busy) return;
    if (!importPath.trim()) {
      setError("Enter a backup file path to import.");
      return;
    }
    if (!importPassword) {
      setError("Enter the master password for the backup.");
      return;
    }

    busy = true;
    try {
      onHeartbeat();
      await importVault(importPath.trim(), importPassword);
      importPath = "";
      importPassword = "";
      await refresh();
      showToast("Encrypted backup imported.");
      setError(null);
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }
</script>

<div class="space-y-4">
  <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
    <div>
      <div class="text-lg font-semibold">Entries</div>
      <div class="text-sm text-neutral-400">
        Search filters the list locally (title, username, url, notes).
      </div>
      {#if $entries.length > 0}
        <div class="mt-1 text-xs text-neutral-500">
          Showing {visible.length} of {$entries.length}
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <input
        class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600 md:w-72 disabled:opacity-50"
        placeholder={$entries.length === 0 ? "Search (add an entry first)" : "Search..."}
        bind:value={q}
        on:input={() => onHeartbeat()}
        disabled={$entries.length === 0}
      />
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
      {:else}
        No matches for "{q}".
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
              expandedId = expandedId === e.id ? null : e.id;
            }}
            onCopy={() => doCopy(e.id)}
            onDelete={() => doDelete(e.id)}
          />
        {/each}
      </div>
    </div>
  {/if}

  <div class="rounded-2xl border border-neutral-800 bg-neutral-900/20 p-6">
    <div class="text-lg font-semibold">Security &amp; backups</div>
    <div class="mt-1 text-sm text-neutral-400">
      Change the master password or export/import an encrypted backup.
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
            />
          </div>
          <div>
            <div class="mb-1 text-xs text-neutral-400">Confirm new password</div>
            <input
              class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
              type="password"
              autocomplete="new-password"
              bind:value={confirmNewPassword}
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
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                placeholder="/path/to/backup.vault"
                bind:value={exportPath}
              />
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
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                placeholder="/path/to/backup.vault"
                bind:value={importPath}
              />
            </div>
            <div>
              <div class="mb-1 text-xs text-neutral-400">Backup master password</div>
              <input
                class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
                type="password"
                autocomplete="current-password"
                bind:value={importPassword}
              />
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
