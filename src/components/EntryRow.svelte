<script lang="ts">
  import type { EntryPublic } from "../lib/api";

  export let entry: EntryPublic;
  export let busy: boolean;
  export let expanded: boolean;

  export let onToggle: () => void;
  export let onCopy: () => void | Promise<void>;
  export let onModify: () => void | Promise<void>;
  export let onDelete: () => void | Promise<void>;
  export let onHeartbeat: () => void;

  function hasText(value: string | null | undefined) {
    return (value ?? "").trim().length > 0;
  }

  function hasNotes(e: EntryPublic) {
    return hasText(e.notes);
  }

  function hasUrl(e: EntryPublic) {
    return hasText(e.url);
  }
</script>

<div class="bg-neutral-950 px-4 py-3">
  <div class="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
    <button
      type="button"
      class="min-w-0 text-left"
      on:click={() => {
        onHeartbeat();
        onToggle();
      }}
      disabled={busy}
      title="Click to expand details"
    >
      <div class="truncate text-sm font-semibold">{entry.title}</div>
      <div class="truncate text-xs text-neutral-400">
        {entry.username}
        {#if hasUrl(entry)}
          <span class="text-neutral-600"> - </span>{entry.url}
        {/if}
      </div>

      {#if hasNotes(entry)}
        <div class="mt-1 truncate text-xs text-neutral-500">
          Notes: {entry.notes}
        </div>
      {/if}
    </button>

    <div class="flex items-center gap-2 md:flex-shrink-0">
      <button
        class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900 disabled:opacity-50"
        on:click={() => {
          onHeartbeat();
          onCopy();
        }}
        disabled={busy}
      >
        Copy
      </button>

      <button
        class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900 disabled:opacity-50"
        on:click={() => {
          onHeartbeat();
          onModify();
        }}
        disabled={busy}
      >
        Modify
      </button>

      <button
        class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900 disabled:opacity-50"
        on:click={() => {
          onHeartbeat();
          onDelete();
        }}
        disabled={busy}
      >
        Delete
      </button>
    </div>
  </div>

  {#if expanded}
    <div class="mt-3 rounded-xl border border-neutral-800 bg-neutral-900/20 p-3 text-xs text-neutral-200">
      <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
        <div>
          <div class="text-neutral-500">Username</div>
          <div class="break-words">{entry.username || "-"}</div>
        </div>
        <div>
          <div class="text-neutral-500">URL</div>
          <div class="break-words">{entry.url || "-"}</div>
        </div>
      </div>

      <div class="mt-3">
        <div class="text-neutral-500">Notes</div>
        <div class="mt-1 whitespace-pre-wrap break-words text-neutral-200">
          {entry.notes || "-"}
        </div>
      </div>

    </div>
  {/if}
</div>
