<script lang="ts">
  import { onDestroy } from "svelte";
  import { unlockVault, getEntries } from "../lib/api";
  import { AppError } from "../lib/errors";
  import { entries, setError } from "../lib/stores";

  export let onDone: () => void | Promise<void>;
  export let onSwitchToSetup: () => void;

  let masterPassword = "";
  let busy = false;
  let loginNotice: string | null = null;
  let lockoutUntil: number | null = null;
  let lockoutRemaining: number | null = null;
  let lockoutTimerId: number | null = null;

  function toErrorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function stopLockoutTimer() {
    if (lockoutTimerId !== null) window.clearInterval(lockoutTimerId);
    lockoutTimerId = null;
    lockoutUntil = null;
    lockoutRemaining = null;
  }

  function updateLockoutRemaining() {
    if (lockoutUntil === null) {
      lockoutRemaining = null;
      return;
    }
    const seconds = Math.max(0, Math.ceil((lockoutUntil - Date.now()) / 1000));
    if (seconds <= 0) {
      stopLockoutTimer();
      return;
    }
    lockoutRemaining = seconds;
  }

  function startLockoutTimer(seconds: number) {
    lockoutUntil = Date.now() + seconds * 1000;
    updateLockoutRemaining();
    if (lockoutTimerId !== null) window.clearInterval(lockoutTimerId);
    lockoutTimerId = window.setInterval(updateLockoutRemaining, 1000);
  }

  onDestroy(() => {
    if (lockoutTimerId !== null) window.clearInterval(lockoutTimerId);
  });

  $: isLockedOut = lockoutRemaining !== null;

  async function submit() {
    setError(null);
    loginNotice = null;

    if (!masterPassword) {
      loginNotice = "Enter your master password.";
      return;
    }

    busy = true;
    try {
      await unlockVault(masterPassword);
      // Immediately fetch entries to validate unlocked state.
      const list = await getEntries();
      entries.set(list);

      await onDone();
    } catch (e) {
      const message = toErrorMessage(e);
      if (e instanceof AppError && e.lockoutSeconds !== undefined) {
        startLockoutTimer(e.lockoutSeconds);
      } else {
        stopLockoutTimer();
      }
      loginNotice = message;
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
        disabled={busy || isLockedOut}
        on:input={() => {
          loginNotice = null;
          setError(null);
        }}
      />
    </label>

    {#if lockoutRemaining !== null}
      <div class="rounded-xl border border-amber-900/60 bg-amber-950/40 px-3 py-2 text-xs text-amber-200">
        Too many failed attempts. Try again in {lockoutRemaining}s.
      </div>
    {:else if loginNotice}
      <div class="rounded-xl border border-red-900/60 bg-red-950/40 px-3 py-2 text-xs text-red-200">
        {loginNotice}
      </div>
    {/if}

    <button
      class="w-full rounded-xl bg-neutral-100 px-3 py-2 text-sm font-semibold text-neutral-950 hover:bg-white disabled:opacity-50"
      type="submit"
      disabled={busy || isLockedOut}
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
