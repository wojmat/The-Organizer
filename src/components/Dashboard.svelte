<script lang="ts">
  import { onMount } from "svelte";
  import EntryModal from "./EntryModal.svelte";
  import { addEntry, copySecret, deleteEntry, getEntries, heartbeat } from "../lib/api";
  import { entries, setError } from "../lib/stores";
  import type { EntryInput, EntryPublic } from "../lib/api";

  export let onHeartbeat: () => void;
  export let onLocked: () => void;

  let showModal = false;
  let busy = false;
  let toast: string | null = null;
  let q = "";
  let deleteConfirm: { id: string; title: string } | null = null;

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
      // Most likely locked.
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

  function requestDelete(id: string, title: string) {
    deleteConfirm = { id, title };
  }

  async function confirmDelete() {
    if (!deleteConfirm || busy) return;
    const id = deleteConfirm.id;
    deleteConfirm = null;

    busy = true;
    try {
      onHeartbeat();
      await deleteEntry(id);
      await refresh();
      showToast("Deleted.");
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      heartbeat().catch(() => {});
    }
  }

  function cancelDelete() {
    deleteConfirm = null;
  }

  async function doCopy(id: string) {
    if (busy) return;
    busy = true;
    try {
      onHeartbeat();
      await copySecret(id);
      showToast("Copied to clipboard (clears in 15s).");
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
      <div class="text-sm text-neutral-400">Passwords are decrypted only while unlocked.</div>
    </div>

    <div class="flex items-center gap-2">
      <input
        class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600 md:w-72"
        placeholder="Search..."
        bind:value={q}
        on:input={() => onHeartbeat()}
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
          <div class="flex flex-col gap-2 bg-neutral-950 px-4 py-3 md:flex-row md:items-center md:justify-between">
            <div class="min-w-0">
              <div class="truncate text-sm font-semibold">{e.title}</div>
              <div class="truncate text-xs text-neutral-400">
                {e.username}{#if e.url} - {e.url}{/if}
              </div>
            </div>

            <div class="flex items-center gap-2">
              <button
                class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900 disabled:opacity-50"
                on:click={() => doCopy(e.id)}
                disabled={busy}
              >
                Copy
              </button>

              <button
                class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900 disabled:opacity-50"
                on:click={() => requestDelete(e.id, e.title)}
                disabled={busy}
              >
                Delete
              </button>
            </div>
          </div>
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

{#if deleteConfirm}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
    <div class="w-full max-w-md rounded-2xl border border-neutral-800 bg-neutral-950 shadow">
      <div class="border-b border-neutral-800 px-5 py-4">
        <div class="text-base font-semibold">Confirm Deletion</div>
      </div>

      <div class="px-5 py-4">
        <p class="text-sm text-neutral-300">
          Are you sure you want to permanently delete <strong class="text-neutral-100">"{deleteConfirm.title}"</strong>?
        </p>
        <p class="mt-2 text-sm text-neutral-400">
          This action cannot be undone.
        </p>
      </div>

      <div class="flex items-center justify-end gap-2 border-t border-neutral-800 px-5 py-4">
        <button
          class="rounded-xl border border-neutral-800 px-4 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
          on:click={cancelDelete}
          disabled={busy}
        >
          Cancel
        </button>

        <button
          class="rounded-xl bg-red-600 px-4 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:opacity-50"
          on:click={confirmDelete}
          disabled={busy}
        >
          {#if busy}Deleting...{:else}Delete{/if}
        </button>
      </div>
    </div>
  </div>
{/if}
