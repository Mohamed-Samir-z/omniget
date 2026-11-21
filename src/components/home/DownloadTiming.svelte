<script lang="ts">
import {t} from "$lib/i18n";
import ClipTimeFields from "$components/librebox/ClipTimeFields.svelte";
import {isAndroid} from "$lib/platform";
let {clipStart=$bindable(""),clipEnd=$bindable(""),scheduleAt=$bindable(""),scheduleStop=$bindable(""),isPlaylist=false}:{clipStart?:string;clipEnd?:string;scheduleAt?:string;scheduleStop?:string;isPlaylist?:boolean}=$props();
  function schedulePart(v: string, part: "date" | "time"): string {
    if (!v) return "";
    const [date, time = ""] = v.split("T");
    return part === "date" ? date : time;
  }

  function withSchedulePart(current: string, part: "date" | "time", value: string): string {
    const pad = (n: number) => String(n).padStart(2, "0");
    let [date, time = ""] = current ? current.split("T") : ["", ""];
    if (part === "date") date = value;
    else time = value;
    if (!date && !time) return "";
    if (!date) {
      const now = new Date();
      date = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
    }
    if (!time) time = "00:00";
    return `${date}T${time}`;
  }

  function setSchedulePreset(kind: "1h" | "tonight" | "1d") {
    const d = new Date();
    if (kind === "1h") d.setHours(d.getHours() + 1);
    else if (kind === "1d") d.setDate(d.getDate() + 1);
    else {
      d.setHours(22, 0, 0, 0);
      if (d.getTime() < Date.now()) d.setDate(d.getDate() + 1);
    }
    const pad = (n: number) => String(n).padStart(2, "0");
    scheduleAt = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

</script>
<div class="download-timing">                          {#if !isPlaylist}
                            <div class="timerange-wrapper">
                              <span class="timerange-label">{$t('librebox.timerange_label')}</span>
                              <div class="clip-groups">
                                <ClipTimeFields label={$t('librebox.timerange_start') as string} bind:value={clipStart} />
                                <ClipTimeFields label={$t('librebox.timerange_end') as string} optionalEnd bind:value={clipEnd} />
                              </div>
                              <span class="timerange-hint">{$t('clip.hint')}</span>
                            </div>
                          {/if}
                          <div class="timerange-wrapper">
                            <span class="timerange-label">{$t('librebox.schedule_label')}</span>
                            <details class="schedule-help"><summary>{$t('downloads.schedule_help')}</summary><p>{$t(isAndroid() ? 'downloads.schedule_system_note' : 'schedule_app_required')}</p></details>
                            <div class="schedule-presets">
                              <button type="button" class="schedule-preset" onclick={() => setSchedulePreset('1h')}>{$t('librebox.schedule_1h')}</button>
                              <button type="button" class="schedule-preset" onclick={() => setSchedulePreset('tonight')}>{$t('librebox.schedule_tonight')}</button>
                              <button type="button" class="schedule-preset" onclick={() => setSchedulePreset('1d')}>{$t('librebox.schedule_1d')}</button>
                              {#if scheduleAt || scheduleStop}
                                <button type="button" class="schedule-preset" onclick={() => { scheduleAt = ""; scheduleStop = ""; }}>{$t('librebox.schedule_clear')}</button>
                              {/if}
                            </div>
<div class="schedule-groups">
{#each (isAndroid()?['start']:['start','stop']) as kind}
<fieldset class="schedule-group"><legend>{$t(kind==='start'?'librebox.schedule_start':'librebox.schedule_stop')}</legend>
<div class="schedule-fields">
<label><span>{$t('timing.date')}</span><input type="date" dir="ltr" value={schedulePart(kind==='start'?scheduleAt:scheduleStop,'date')} oninput={(e)=>{if(kind==='start')scheduleAt=withSchedulePart(scheduleAt,'date',e.currentTarget.value);else scheduleStop=withSchedulePart(scheduleStop,'date',e.currentTarget.value)}} aria-label={`${$t(kind==='start'?'librebox.schedule_start':'librebox.schedule_stop')}: ${$t('timing.date')}`} /></label>
<label><span>{$t('timing.time')}</span><input type="time" dir="ltr" step="60" value={schedulePart(kind==='start'?scheduleAt:scheduleStop,'time')} oninput={(e)=>{if(kind==='start')scheduleAt=withSchedulePart(scheduleAt,'time',e.currentTarget.value);else scheduleStop=withSchedulePart(scheduleStop,'time',e.currentTarget.value)}} aria-label={`${$t(kind==='start'?'librebox.schedule_start':'librebox.schedule_stop')}: ${$t('timing.time')}`} /></label>
</div></fieldset>
{/each}
</div>
                            <span class="timerange-hint" class:timing-conflict={isAndroid() && !!scheduleAt && (!!clipStart || !!clipEnd)} role="status">{$t(isAndroid()?'timing.android_hint':'librebox.schedule_hint')}</span>
                          </div>
</div>
<style>
.download-timing{container-type:inline-size}.timing-conflict{color:var(--error)!important;font-weight:500}
.timerange-wrapper{display:flex;flex-direction:column;gap:12px;padding:14px;border:1px solid var(--border);border-radius:12px;min-width:0;box-sizing:border-box;width:100%}.timerange-wrapper+.timerange-wrapper{margin-top:14px}.timerange-label{font-size:14px;font-weight:600;color:var(--text)}.timerange-hint,.schedule-help{font-size:12px;line-height:1.6;color:var(--text-muted)}.schedule-help summary{cursor:pointer}.schedule-help p{margin:8px 0 0}.clip-groups,.schedule-groups{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;align-items:start}.schedule-group{min-width:0;border:0;padding:0;margin:0}.schedule-group legend{font-size:12px;color:var(--text);margin-bottom:8px;padding:0}.schedule-fields{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr);gap:8px}.schedule-fields label{display:flex;min-width:0;flex-direction:column;gap:6px;font-size:11px;color:var(--text-muted)}.schedule-fields input{box-sizing:border-box;display:block;width:100%;max-width:100%;min-width:0;min-height:46px;padding:10px 8px;border:1px solid var(--border);border-radius:8px;background:var(--fill-1);color:var(--text);font:inherit;font-size:14px;color-scheme:inherit;text-align:start}.schedule-fields input::-webkit-date-and-time-value{text-align:start;min-height:20px}.schedule-presets{display:flex;flex-wrap:wrap;gap:8px}.schedule-preset{padding:8px 12px;min-height:44px;border:1px solid var(--border);border-radius:8px;background:var(--fill-1);color:var(--text);font:inherit;font-size:12px}.schedule-fields input:focus-visible,.schedule-preset:focus-visible{outline:2px solid var(--accent);outline-offset:2px}@media(max-width:360px){.schedule-fields{grid-template-columns:minmax(0,1fr)}}@media(max-width:600px){.clip-groups,.schedule-groups{grid-template-columns:minmax(0,1fr)}.timerange-wrapper{padding:12px}}
@container(max-width:330px){.schedule-fields{grid-template-columns:minmax(0,1fr)}}
</style>