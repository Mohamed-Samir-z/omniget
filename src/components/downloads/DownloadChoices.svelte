<script lang="ts">
  import { t, locale } from "$lib/i18n";
  import type { DownloadSelection } from "$lib/download-display";
  let { selection }: { selection?: DownloadSelection | null } = $props();
  function language(code: string): string {
    try {
      const name = new Intl.DisplayNames([$locale || "en"], { type: "language" }).of(code.split("-")[0]);
      return name && name !== code ? `${name} (${code})` : code;
    } catch { return code; }
  }
  const names = (codes: string[]) => codes.map(language).join(" / ");
</script>
<div class="download-choices" aria-label={$t('downloads.selection_details')}>
  {#if selection}
    <span class="choice">{$t('downloads.audio_requested')}: <bdi>{selection.audio_requested.length ? names(selection.audio_requested) : $t('downloads.source_audio')}</bdi></span>
    <span class="choice">{$t('downloads.subtitles_requested')}: <bdi>{selection.subtitles_enabled === false ? $t('downloads.subtitles_off') : selection.subtitle_requested ? language(selection.subtitle_requested) : $t('downloads.app_setting')}</bdi>{#if selection.subtitles_enabled !== false && selection.automatic_subtitles} · {$t('downloads.automatic_captions')}{/if}</span>
    {#if selection.subtitles_obtained.length}
      <span class="choice saved">{$t('downloads.subtitles_saved')}: <bdi>{names(selection.subtitles_obtained)}</bdi></span>
    {/if}
  {:else}
    <span class="unknown">{$t('downloads.selection_not_recorded')}</span>
  {/if}
</div>
<style>
  .download-choices{display:flex;flex-wrap:wrap;gap:6px;margin-block:10px 4px;font-size:11px;line-height:1.6;color:var(--text-muted);min-width:0}
  .choice{border:1px solid var(--border);border-radius:7px;padding:3px 7px;max-width:100%;overflow-wrap:anywhere;background:var(--fill-1)}
  .saved{color:var(--text)}.unknown{font-size:10px;opacity:.8}
</style>
