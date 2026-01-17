<script lang="ts">
  import { unlockVault, getEntries } from "../lib/api";
  import { entries, setError } from "../lib/stores";

  export let onDone: () => void | Promise<void>;
  export let onSwitchToSetup: () => void;

  let masterPassword = "";
  let busy = false;

  async function submit() {
    setError(null);

    if (!masterPassword) {
      setError("Enter your master password.");
      return;
    }

    busy = true;
    try {
      await unlockVault(masterPassword);
      // Immediately fetch entries to validate unlocked state.
      const list = await getEntries();
      entries.set(list);

      masterPassword = "";
      await onDone();
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      // Best-effort clear
      masterPassword = "";
    }
  }
</script>

<div class="mx-auto max-w-md rounded-2xl border border-neutral-800 bg-neutral-900/30 p-6 shadow">
  <h1 class="text-xl font-semibold">Unlock vault</h1>
  <p class="mt-1 text-sm text-neutral-400">
    Enter your master password to decrypt the local vault.
  </p>

  <form class="mt-5 space-y-3" on:submit|preventDefault={submit}>
    <label class="block">
      <div class="mb-1 text-sm text-neutral-300">Master password</div>
      <input
        class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
        type="password"
        autocomplete="current-password"
        bind:value={masterPassword}
        disabled={busy}
      />
    </label>

    <button
      class="w-full rounded-xl bg-neutral-100 px-3 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
      type="submit"
      disabled={busy}
    >
      {#if busy}Unlocking...{:else}Unlock{/if}
    </button>

    <button
      class="w-full rounded-xl border border-neutral-800 px-3 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
      type="button"
      disabled={busy}
      on:click={onSwitchToSetup}
    >
      First time? Create a new vault
    </button>
  </form>

  <div class="mt-4 text-xs text-neutral-500">
    Tip: If you forget the master password, this vault cannot be recovered.
  </div>
</div>
