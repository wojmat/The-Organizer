<script lang="ts">
  import type { EntryInput } from "../lib/api";

  export let onCancel: () => void;
  export let onCreate: (input: EntryInput) => Promise<void>;

  let title = "";
  let username = "";
  let password = "";
  let url = "";
  let notes = "";

  let busy = false;
  let localError: string | null = null;

  function reset() {
    title = "";
    username = "";
    password = "";
    url = "";
    notes = "";
    localError = null;
    busy = false;
  }

  function validate(): string | null {
    if (!title.trim()) return "Title is required.";
    if (!password) return "Password is required.";
    return null;
  }

  async function submit() {
    localError = null;

    const msg = validate();
    if (msg) {
      localError = msg;
      return;
    }

    busy = true;
    try {
      await onCreate({
        title: title.trim(),
        username: username.trim(),
        password,
        url: url.trim(),
        notes: notes.trim()
      });

      // Clear sensitive input immediately after use.
      reset();
      onCancel();
    } catch (e) {
      localError = (e as Error).message ?? String(e);
      password = "";
    } finally {
      busy = false;
      // Best-effort clear password
      password = "";
    }
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
  <div class="w-full max-w-lg rounded-2xl border border-neutral-800 bg-neutral-950 shadow">
    <div class="flex items-center justify-between border-b border-neutral-800 px-5 py-4">
      <div class="text-base font-semibold">New entry</div>
      <button
        class="rounded-xl border border-neutral-800 px-3 py-1.5 text-sm hover:bg-neutral-900"
        on:click={() => {
          reset();
          onCancel();
        }}
        disabled={busy}
      >
        Close
      </button>
    </div>

    <form class="space-y-3 px-5 py-4" on:submit|preventDefault={submit}>
      {#if localError}
        <div class="rounded-xl border border-red-900 bg-red-950/40 px-4 py-3 text-sm text-red-200">
          {localError}
        </div>
      {/if}

      <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
        <label class="block">
          <div class="mb-1 text-sm text-neutral-300">Title</div>
          <input
            class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
            bind:value={title}
            disabled={busy}
          />
        </label>

        <label class="block">
          <div class="mb-1 text-sm text-neutral-300">Username</div>
          <input
            class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
            bind:value={username}
            disabled={busy}
          />
        </label>
      </div>

      <label class="block">
        <div class="mb-1 text-sm text-neutral-300">Password</div>
        <input
          class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
          type="password"
          autocomplete="new-password"
          bind:value={password}
          disabled={busy}
        />
      </label>

      <label class="block">
        <div class="mb-1 text-sm text-neutral-300">URL</div>
        <input
          class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
          placeholder="https://example.com"
          bind:value={url}
          disabled={busy}
        />
      </label>

      <label class="block">
        <div class="mb-1 text-sm text-neutral-300">
          Notes
          <span class="ml-2 text-xs text-neutral-500">
            (visible when you expand the row, and on desktop hover)
          </span>
        </div>
        <textarea
          class="min-h-24 w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
          bind:value={notes}
          disabled={busy}
        />
      </label>

      <div class="flex items-center justify-end gap-2 pt-2">
        <button
          class="rounded-xl border border-neutral-800 px-4 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
          type="button"
          on:click={() => {
            reset();
            onCancel();
          }}
          disabled={busy}
        >
          Cancel
        </button>

        <button
          class="rounded-xl bg-neutral-100 px-4 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
          type="submit"
          disabled={busy}
        >
          {#if busy}Saving...{:else}Save{/if}
        </button>
      </div>
    </form>
  </div>
</div>
