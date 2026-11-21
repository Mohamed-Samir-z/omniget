<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import CompletedFileActions from "./CompletedFileActions.svelte";
  type Scheduled = { id: string; url: string; at: number; state: string; percent: number; path: string; error: string };
  const android = typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);
  let items = $state<Scheduled[]>([]);
  let error = $state("");
  let loading = $state(false);
  let busy = $state<string | null>(null);
  const active = (state: string) => ['ENQUEUED', 'RUNNING', 'BLOCKED'].includes(state);
  async function refresh() {
    if (loading) return;
    loading = true;
    try { items = JSON.parse(await invoke<string>('android_schedules', { action: 'list', id: null })).sort((a: Scheduled,b: Scheduled) => b.at-a.at); error = ''; }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }
  async function action(item: Scheduled) {
    busy = item.id;
    try { await invoke('android_schedules', { action: active(item.state) ? 'cancel' : 'forget', id: item.id }); await refresh(); }
    catch (e) { error = String(e); }
    finally { busy = null; }
  }
  function stateLabel(item: Scheduled) {
    const keys: Record<string,string> = { ENQUEUED:'schedule_waiting', RUNNING:'schedule_running', BLOCKED:'schedule_waiting', SUCCEEDED:'schedule_complete', FAILED:'schedule_failed', CANCELLED:'schedule_cancelled', INTERRUPTED:'schedule_interrupted' };
    return $t(`downloads.${keys[item.state] || 'schedule_interrupted'}`);
  }
  onMount(() => {
    if (!android) return;
    void refresh();
    // The app's native WorkManager database, not an external website.
    const timer = setInterval(() => { if (document.visibilityState === 'visible') void refresh(); }, 10000);
    return () => clearInterval(timer);
  });
</script>
{#if android}
<section class="os-schedules" aria-label={$t('downloads.os_schedules')}>
  <div class="schedule-heading"><h2>{$t('downloads.os_schedules')}</h2><button class="button" onclick={refresh} disabled={loading}>{$t(loading ? 'settings.update_checking' : 'downloads.schedule_refresh')}</button></div>
  <p class="schedule-note">{$t('downloads.schedule_system_note')}</p>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if !items.length && !loading}<p>{$t('downloads.schedule_empty')}</p>{/if}
  {#each items as item (item.id)}
    <article class="schedule-item">
      <div class="schedule-info"><span class="schedule-url">{item.url}</span><time datetime={new Date(item.at).toISOString()}>{new Date(item.at).toLocaleString()}</time>
        <span>{stateLabel(item)}{item.state === 'RUNNING' && item.percent >= 0 ? ` · ${item.percent}%` : ''}</span>
        {#if item.error}<p class="schedule-error">{item.error}</p>{/if}
      </div>
      <div class="schedule-actions">{#if item.path}<CompletedFileActions path={item.path} />{/if}<button class="button" disabled={busy === item.id} onclick={() => action(item)}>{$t(active(item.state) ? 'downloads.schedule_cancel' : 'downloads.schedule_remove')}</button></div>
    </article>
  {/each}
</section>
{/if}
<style>
.os-schedules{margin:20px 0;padding:18px;border:1px solid var(--border-subtle);border-radius:14px;background:var(--surface-raised)}.schedule-heading{display:flex;justify-content:space-between;align-items:center;gap:12px}.schedule-heading h2{font-size:16px;margin:0}.schedule-note,.os-schedules>p{font-size:12px;line-height:1.6;color:var(--text-secondary)}.schedule-item{display:flex;align-items:center;gap:12px;padding:14px 0;border-top:1px solid var(--border-subtle)}.schedule-info{display:flex;flex:1;min-width:0;flex-direction:column;gap:6px;font-size:12px}.schedule-url{overflow-wrap:anywhere;color:var(--text-primary)}time{color:var(--text-secondary)}.schedule-actions{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.schedule-error{color:var(--danger,#ed7373);overflow-wrap:anywhere;white-space:pre-wrap;margin:0}@media(max-width:480px){.schedule-item{align-items:flex-start;flex-direction:column}.schedule-actions{align-self:flex-end}}
</style>
