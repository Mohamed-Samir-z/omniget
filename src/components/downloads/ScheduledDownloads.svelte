<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, locale } from "$lib/i18n";
  import CompletedFileActions from "./CompletedFileActions.svelte";
  export type Scheduled = { id: string; url: string; at: number; finishedAt?: number; state: string; percent: number; path: string; error: string };
  let { filter = "all", mode = "active", onitems = (_items: Scheduled[]) => {} }: { filter?: string; mode?: string; onitems?: (items: Scheduled[]) => void } = $props();
  const android = typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);
  let items = $state<Scheduled[]>([]);
  let error = $state("");
  let loading = $state(false);
  let busy = $state<string | null>(null);
  let editing = $state<string | null>(null);
  let editDate = $state("");
  let editTime = $state("");
  const visible = (state: string) => !['SUCCEEDED','CANCELLED'].includes(state);
  const editable = (state: string) => state !== 'RUNNING';
  let shown = $derived(items.filter(item => filter === 'completed' ? item.state === 'SUCCEEDED' : filter === 'failed' ? ['FAILED','INTERRUPTED'].includes(item.state) : visible(item.state)));
  function edit(item: Scheduled) {
    const d = new Date(item.at);const pad=(n:number)=>String(n).padStart(2,'0');
    editDate=`${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}`;editTime=`${pad(d.getHours())}:${pad(d.getMinutes())}`;editing=item.id;
  }
  async function change(item: Scheduled, now: boolean) {
    if(busy) return;
    const at=new Date(`${editDate}T${editTime}`).getTime();
    if(!now && (!Number.isFinite(at)||at<=Date.now())) {error=$t('schedule_future_required');return;}
    busy=item.id;error='';
    try {await invoke('android_schedules',{action:now?'run-now':'reschedule',id:JSON.stringify({id:item.id,...(!now?{at}:{})})});editing=null;await refresh();}
    catch(e){await refresh();error=String(e);}finally{busy=null;}
  }
  const active = (state: string) => ['ENQUEUED', 'RUNNING', 'BLOCKED'].includes(state);
  async function refresh() {
    if (loading) return;
    loading = true;
    try { items = JSON.parse(await invoke<string>('android_schedules', { action: 'list', id: null })).sort((a: Scheduled,b: Scheduled) => b.at-a.at); error = ''; onitems(items); }
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
{#if android && mode === "active" && (shown.length > 0 || !!error)}
<section class="os-schedules" aria-label={$t('downloads.os_schedules')}>
  <div class="schedule-heading"><h2>{$t(filter==='completed'?'downloads.schedule_completed_heading':'downloads.os_schedules')}</h2><button class="button" onclick={refresh} disabled={loading}>{$t(loading ? 'settings.update_checking' : 'downloads.schedule_refresh')}</button></div>
  <details class="schedule-details"><summary>{$t('downloads.schedule_help')}</summary><p class="schedule-note">{$t('downloads.schedule_system_note')}</p></details>
  {#if error}<p role="alert">{error}</p>{/if}
  {#each shown as item (item.id)}
    <article class="schedule-item"><div class="schedule-body">
      <div class="schedule-info"><span class="schedule-url" dir="ltr">{item.url}</span><time datetime={new Date(item.at).toISOString()}>{new Date(item.at).toLocaleString($locale,{dateStyle:"medium",timeStyle:"short"})}</time>
        <span>{stateLabel(item)}{item.state === 'RUNNING' && item.percent >= 0 ? ` · ${item.percent}%` : ''}</span>
        {#if item.error}<details class="schedule-error"><summary>{$t("downloads.schedule_error_details")}</summary><p dir="auto">{item.error}</p></details>{/if}
      </div>
      {#if editing === item.id}<div class="schedule-editor"><label>{$t('timing.date')}<input type="date" dir="ltr" bind:value={editDate}/></label><label>{$t('timing.time')}<input type="time" dir="ltr" bind:value={editTime}/></label><button class="button" disabled={busy===item.id} onclick={()=>change(item,false)}>{$t('downloads.schedule_save')}</button><button class="button" onclick={()=>editing=null}>{$t('common.close')}</button></div>{/if}
      <div class="schedule-actions">{#if visible(item.state) && editable(item.state)}<button class="button" disabled={busy===item.id} onclick={()=>change(item,true)}>{$t('downloads.schedule_now')}</button><button class="button" disabled={busy===item.id} onclick={()=>edit(item)}>{$t('downloads.schedule_edit')}</button>{/if}{#if item.path}<CompletedFileActions path={item.path} />{/if}<button class="button" disabled={busy === item.id} onclick={() => action(item)}>{$t(active(item.state) ? 'downloads.schedule_cancel' : 'downloads.schedule_remove')}</button></div>
    </div></article>
  {/each}
</section>
{/if}
<style>
.schedule-item{background:var(--surface);border:1px solid var(--border)!important;border-radius:12px;padding:14px!important;margin-top:12px}.schedule-info{line-height:1.7}.schedule-url{font-size:12px;text-align:start}.schedule-info time{font-variant-numeric:tabular-nums}.schedule-error summary{cursor:pointer;min-height:36px;display:flex;align-items:center}.schedule-error p{margin:4px 0 0;font:11px/1.6 var(--font-mono);white-space:pre-wrap;overflow-wrap:anywhere}.schedule-actions .button{min-height:44px;border-radius:8px;font-size:12px;padding:8px 12px}.schedule-actions .button:first-child{background:var(--accent-soft);color:var(--accent-hi)}.schedule-body{width:100%;min-width:0;display:flex;flex-direction:column;gap:10px}.schedule-editor{display:flex;flex-wrap:wrap;gap:8px}.schedule-editor label{display:flex;flex-direction:column;gap:4px;font-size:12px;flex:1;min-width:110px}.schedule-editor input{width:100%;box-sizing:border-box;min-width:0;padding:8px;border:1px solid var(--border);border-radius:8px;background:var(--surface);color:var(--text)}
.schedule-details{font-size:12px;color:var(--text-muted);line-height:1.6}.schedule-details summary{cursor:pointer;padding:6px 0}.os-schedules{margin:12px 0;padding:12px;min-width:0;box-sizing:border-box;border:1px solid var(--border-subtle);border-radius:14px;background:var(--surface-raised)}.schedule-heading{display:flex;justify-content:space-between;align-items:center;gap:12px}.schedule-heading h2{font-size:14px;margin:0;min-width:0}.schedule-heading .button{flex-shrink:0;font-size:12px;padding:4px 10px}.schedule-note,.os-schedules>p{font-size:12px;line-height:1.6;color:var(--text-secondary)}.schedule-item{display:flex;align-items:center;gap:12px;padding:14px 0;border-top:1px solid var(--border-subtle)}.schedule-info{display:flex;flex:1;min-width:0;flex-direction:column;gap:6px;font-size:12px}.schedule-url{overflow-wrap:anywhere;color:var(--text-primary)}time{color:var(--text-secondary)}.schedule-actions{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.schedule-error{color:var(--danger,#ed7373);overflow-wrap:anywhere;white-space:pre-wrap;margin:0}@media(max-width:480px){.schedule-item{align-items:flex-start;flex-direction:column}.schedule-actions{align-self:flex-end}}
</style>
