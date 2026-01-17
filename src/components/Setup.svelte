<script lang="ts">
  import { createVault, getEntries } from "../lib/api";
  import { entries, setError } from "../lib/stores";

  export let onDone: () => void | Promise<void>;
  export let onSwitchToLogin: () => void;

  let masterPassword = "";
  let confirmPassword = "";
  let busy = false;

  function validate(): string | null {
    if (!masterPassword) return "Enter a master password.";
    if (masterPassword.length < 10) return "Use at least 10 characters.";
    if (masterPassword !== confirmPassword) return "Passwords do not match.";
    return null;
  }

  async function submit() {
    setError(null);
    const msg = validate();
    if (msg) {
      setError(msg);
      return;
    }

    busy = true;
    try {
      await createVault(masterPassword);
      const list = await getEntries();
      entries.set(list);

      masterPassword = "";
      confirmPassword = "";
      await onDone();
    } catch (e) {
      setError((e as Error).message ?? String(e));
    } finally {
      busy = false;
      // Best-effort clear
      masterPassword = "";
      confirmPassword = "";
    }
  }
</script>

<div class="mx-auto max-w-md rounded-2xl border border-neutral-800 bg-neutral-900/30 p-6 shadow">
  <h1 class="text-xl font-semibold">Create a new vault</h1>
  <p class="mt-1 text-sm text-neutral-400">
    The master password is never stored. Keep it safe.
  </p>

  <form class="mt-5 space-y-3" on:submit|preventDefault={submit}>
    <label class="block">
      <div class="mb-1 text-sm text-neutral-300">Master password</div>
      <input
        class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
        type="password"
        autocomplete="new-password"
        bind:value={masterPassword}
        disabled={busy}
      />
    </label>

    <label class="block">
      <div class="mb-1 text-sm text-neutral-300">Confirm password</div>
      <input
        class="w-full rounded-xl border border-neutral-800 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-neutral-600"
        type="password"
        autocomplete="new-password"
        bind:value={confirmPassword}
        disabled={busy}
      />
    </label>

    <button
      class="w-full rounded-xl bg-neutral-100 px-3 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
      type="submit"
      disabled={busy}
    >
      {#if busy}Creating...{:else}Create vault{/if}
    </button>

    <button
      class="w-full rounded-xl border border-neutral-800 px-3 py-2 text-sm hover:bg-neutral-900 disabled:opacity-50"
      type="button"
      disabled={busy}
      on:click={onSwitchToLogin}
    >
      I already have a vault
    </button>
  </form>

  <div class="mt-4 text-xs text-neutral-500">
    Recommendation: use a long passphrase. Avoid reusing passwords from other sites.
  </div>
</div>
