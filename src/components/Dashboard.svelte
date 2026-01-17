<script lang="ts">
  import { onMount } from "svelte";
  import EntryModal from "./EntryModal.svelte";
  import EntryRow from "./EntryRow.svelte";
  import { addEntry, copySecret, deleteEntry, getEntries, heartbeat } from "../lib/api";
  import { entries, setError } from "../lib/stores";
  import type { EntryInput, EntryPublic } from "../lib/api";

  export let onHeartbeat: () => void;
  export let onLocked: () => void;

  let showModal = false;
  let busy = false;
  let toast: string | null = null;
  let q = "";
  let expandedId: string | null = null;

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
</div>

{#if showModal}
  <EntryModal
    onCancel={() => {
      showModal = false;
    }}
    onCreate={doCreate}
  />
{/if}
