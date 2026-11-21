<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  let { path }: { path: string } = $props();
  const android = typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);
  async function act(share: boolean) {
    try { await invoke(share ? "share_downloaded_file" : "open_path_default", { path }); }
    catch (e) { showToast("error", String(e)); }
  }
</script>
{#if android}
  <button class="action-icon-btn" onclick={() => act(false)} aria-label={$t('downloads.file_open')} title={$t('downloads.file_open')}>
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 5 10 7-10 7Z"/></svg>
  </button>
  <button class="action-icon-btn" onclick={() => act(true)} aria-label={$t('downloads.file_share')} title={$t('downloads.file_share')}>
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="m9 10 6-4M9 14l6 4"/></svg>
  </button>
{/if}
<style>
.action-icon-btn{display:inline-flex;align-items:center;justify-content:center;width:34px;height:34px;flex-shrink:0;border:1px solid var(--border-subtle,#444);border-radius:8px;background:transparent;color:var(--text-secondary,#aaa);cursor:pointer}.action-icon-btn:hover{background:var(--surface-hover,#333);color:var(--text-primary,#fff)}.action-icon-btn:focus-visible{outline:2px solid var(--accent,#ffab00);outline-offset:2px}
</style>
