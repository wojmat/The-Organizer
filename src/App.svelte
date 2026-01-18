<script lang="ts">
  import { onMount } from "svelte";
  import Login from "./components/Login.svelte";
  import Setup from "./components/Setup.svelte";
  import Dashboard from "./components/Dashboard.svelte";
  import { get } from "svelte/store";
  import { entries, isLocked, lastError, setError, view } from "./lib/stores";
  import { getEntries, heartbeat, lockVault } from "./lib/api";

  // Simple throttle so we do not spam heartbeat.
  function throttle(fn: () => void, ms: number) {
    let last = 0;
    return () => {
      const now = Date.now();
      if (now - last >= ms) {
        last = now;
        fn();
      }
    };
  }

  const beat = throttle(() => {
    heartbeat().catch(() => {
      // best-effort
    });
  }, 2000);

  async function refreshEntriesOrLock() {
    try {
      const list = await getEntries();
      entries.set(list);
      isLocked.set(false);
      view.set("dashboard");
      setError(null);
    } catch {
      // If locked, stay on login unless user switches.
      isLocked.set(true);
      if (get(view) === "dashboard") view.set("login");
    }
  }

  let intervalId: number | null = null;
  let refreshIntervalId: number | null = null;

  onMount(() => {
    // Best-effort: see if we are already unlocked (normally not).
    refreshEntriesOrLock();

    // Keep session alive while user is interacting.
    const handler = () => beat();
    window.addEventListener("mousemove", handler);
    window.addEventListener("keydown", handler);
    window.addEventListener("mousedown", handler);

    // Periodic heartbeat (also updates last_interaction).
    intervalId = window.setInterval(() => beat(), 30_000);
    // Periodic backend check to detect auto-lock and update UI.
    refreshIntervalId = window.setInterval(() => {
      if (get(view) === "dashboard") {
        refreshEntriesOrLock();
      }
    }, 15_000);

    return () => {
      window.removeEventListener("mousemove", handler);
      window.removeEventListener("keydown", handler);
      window.removeEventListener("mousedown", handler);
      if (intervalId !== null) window.clearInterval(intervalId);
      if (refreshIntervalId !== null) window.clearInterval(refreshIntervalId);
    };
  });

  async function onLogout() {
    try {
      await lockVault();
    } catch (e) {
      lastError.set((e as Error).message ?? String(e));
    } finally {
      entries.set([]);
      isLocked.set(true);
      view.set("login");
    }
  }
</script>

<div class="min-h-screen bg-neutral-950 text-neutral-100">
  <header class="border-b border-neutral-800">
    <div class="mx-auto flex max-w-5xl items-center justify-between px-4 py-3">
      <div class="flex items-center gap-3">
        <div class="h-9 w-9 rounded-xl bg-neutral-800"></div>
        <div class="leading-tight">
          <div class="text-sm text-neutral-300">The Organizer</div>
          <div class="text-xs text-neutral-500">Local zero-knowledge vault</div>
        </div>
      </div>

      <div class="flex items-center gap-3">
        {#if $view === "dashboard"}
          <button
            class="rounded-xl border border-neutral-800 px-3 py-2 text-sm hover:bg-neutral-900"
            on:click={onLogout}
          >
            Lock
          </button>
        {/if}
      </div>
    </div>
  </header>

  <main class="mx-auto max-w-5xl px-4 py-6">
    {#if $lastError}
      <div class="mb-4 rounded-xl border border-red-900 bg-red-950/40 px-4 py-3 text-sm text-red-200">
        {$lastError}
      </div>
    {/if}

    {#if $view === "setup"}
      <Setup
        onDone={async () => {
          await refreshEntriesOrLock();
        }}
        onSwitchToLogin={() => {
          setError(null);
          view.set("login");
        }}
      />
    {:else if $view === "login"}
      <Login
        onDone={async () => {
          await refreshEntriesOrLock();
        }}
        onSwitchToSetup={() => {
          setError(null);
          view.set("setup");
        }}
      />
    {:else}
      <Dashboard
        onHeartbeat={() => beat()}
        onLocked={() => {
          entries.set([]);
          isLocked.set(true);
          view.set("login");
        }}
      />
    {/if}
  </main>
</div>
