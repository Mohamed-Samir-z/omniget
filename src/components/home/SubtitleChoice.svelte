<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  let { url, selection = $bindable("") }: { url: string; selection?: string } = $props();
  type Track = { lang: string; name: string; auto: boolean };
  let tracks = $state<Track[]>([]);
  let loaded = $state(false);
  let loading = $state(false);
  let error = $state("");
  let generation = 0;
  $effect(() => { void url; generation++; selection = ""; tracks = []; loaded = false; error = ""; loading = false; });
  async function load() {
    const current = generation;
    loading = true; error = "";
    try {
      const result = await invoke<{ tracks: Track[] }>("subtitles_list", { url });
      if (current === generation) { tracks = result.tracks.filter(x => /^[A-Za-z0-9_-]{1,40}$/.test(x.lang)); loaded = true; }
    } catch (e) { if (current === generation) error = String(e); }
    finally { if (current === generation) loading = false; }
  }
</script>
<div class="subtitle-choice">
  <label for="per-download-subtitles">{$t('librebox.subtitle_label')}</label>
  <p>{$t('librebox.subtitle_source_only')}</p>
  <button type="button" class="button" onclick={load} disabled={loading}>{$t(loading ? 'librebox.subtitle_loading' : error ? 'settings.update_retry' : 'librebox.subtitle_load')}</button>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if loaded}
    <select id="per-download-subtitles" bind:value={selection}>
      <option value="">{$t('librebox.subtitle_off')}</option>
      {#each tracks as track (`${track.auto}:${track.lang}`)}
        <option value={`${track.auto ? 'auto' : 'manual'}:${track.lang}`}>{track.name || track.lang} ({track.lang}){track.auto ? ` · ${$t('librebox.subtitle_auto')}` : ''}</option>
      {/each}
    </select>
    {#if !tracks.length}<p>{$t('librebox.subtitle_none')}</p>{/if}
  {/if}
</div>
<style>
.subtitle-choice{display:flex;flex-wrap:wrap;align-items:center;gap:8px;padding:12px 0}.subtitle-choice label{font-size:13px;font-weight:600;flex-basis:100%}.subtitle-choice p{font-size:12px;color:var(--text-secondary);margin:0;flex-basis:100%;line-height:1.5}.subtitle-choice select{width:100%;max-width:100%;padding:10px;border:1px solid var(--border-subtle);border-radius:8px;background:var(--surface-raised);color:var(--text-primary)}
</style>
