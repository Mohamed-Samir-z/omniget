<script lang="ts">
  import { t, locale } from "$lib/i18n";

  type FormatInfo = {
    format_id: string;
    ext: string;
    resolution: string | null;
    width: number | null;
    height: number | null;
    fps: number | null;
    vcodec: string | null;
    acodec: string | null;
    filesize: number | null;
    filesize_is_estimate?: boolean;
    tbr: number | null;
    has_video: boolean;
    has_audio: boolean;
    format_note: string | null;
    audio_language?: string | null;
  };

  let {
    platform,
    isPlaylist = false,
    formats = $bindable([]),
    selectedFormatId = $bindable<string | null>(null),
    selectedAudioLanguage = $bindable<string | null>(null),
    selectedAudioLanguages = $bindable<string[]>([]),
    multipleAudio = $bindable(false),
    loadingFormats = false,
    expanded = true,
    formatError = null as string | null,
    onLoadFormats,
    onSelectFormat,
    onSelectAudioLanguage,
    onClearFormat,
    onPresetBest,
    onPresetMusic,
  } = $props();

  function formatFilesize(bytes: number | null, isEstimate = false): string {
    if (bytes === null) return "—";
    // O `~` é um contrato com o usuário: número sem til é o que o site
    // informou; com til, foi calculado por bitrate × duração.
    const prefix = isEstimate ? "~" : "";
    if (bytes < 1024) return `${prefix}${bytes} B`;
    if (bytes < 1024 * 1024) return `${prefix}${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${prefix}${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${prefix}${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  const languageNames = $derived(
    typeof Intl !== "undefined" && "DisplayNames" in Intl
      ? new Intl.DisplayNames($locale || "en", { type: "language" })
      : null);

  // Distinct dub/audio languages across the loaded formats. Shown only when
  // there is a real choice (dubbed videos carry 2+ audio tracks).
  let audioLanguages = $derived(
    [
      ...new Set(
        formats
          .filter((f) => f.has_audio && f.audio_language)
          .map((f) => f.audio_language as string)
      ),
    ].sort()
  );

  let multitrackLanguages = $derived([...new Set(formats.filter(f => f.has_audio && !f.has_video && f.audio_language).map(f => f.audio_language as string))].sort());
  function toggleMultiple() {
    multipleAudio = !multipleAudio;
    selectedAudioLanguages = [];
    selectedAudioLanguage = null;
    selectedFormatId = null;
  }
  function toggleAudio(language: string) {
    selectedFormatId = null;
    selectedAudioLanguage = null;
    selectedAudioLanguages = selectedAudioLanguages.includes(language) ? selectedAudioLanguages.filter(l => l !== language) : [...selectedAudioLanguages, language];
  }
  function formatAudioLanguage(code: string | null | undefined): string | null {
    if (!code) return null;
    try {
      return languageNames?.of(code) ?? code;
    } catch {
      return code;
    }
  }

  function formatCodec(codec: string | null): string {
    if (!codec || codec === "none") return "—";
    return codec.split(".")[0];
  }

  type Preset = "best" | "music" | "custom" | null;
  let activePreset = $state<Preset>(null);

  let bestVA = $derived([...formats].reverse().find(f => f.has_video && f.has_audio) ?? null);
  let bestAudio = $derived([...formats].reverse().find(f => f.has_audio && !f.has_video) ?? null);
  let bestVideo = $derived([...formats].reverse().find(f => f.has_video && !f.has_audio) ?? null);

  let hasMusicPreset = $derived(bestAudio !== null);

  let selectedFormatLabel = $derived.by(() => {
    if (!selectedFormatId) return null;
    const f = formats.find(fmt => fmt.format_id === selectedFormatId);
    if (!f) return selectedFormatId;
    const parts: string[] = [];
    if (f.resolution) parts.push(f.resolution);
    parts.push(f.ext);
    if (f.has_video && f.has_audio) parts.push("V+A");
    else if (f.has_video) parts.push("V");
    else if (f.has_audio) parts.push("A");
    return parts.join(" · ");
  });

  function isYtdlpPlatform(p: string): boolean {
    return !["hotmart", "telegram", "udemy", "unknown"].includes(p);
  }

  function applyBest() {
    activePreset = "best";
    if (selectedFormatId) onClearFormat();
    onPresetBest?.();
  }

  function applyMusic() {
    activePreset = "music";
    if (selectedFormatId) onClearFormat();
    onPresetMusic?.();
  }

  function toggleCustom() {
    activePreset = activePreset === "custom" ? null : "custom";
    if (selectedFormatId) onClearFormat();
  }

  $effect(() => {
    if (!formats.length) {
      activePreset = null;
      multipleAudio = false;
      selectedAudioLanguages = [];
    }
  });
</script>

{#if isYtdlpPlatform(platform) && isPlaylist}
  <div class="formats-playlist-note">
    {$t('librebox.formats_playlist_note')}
  </div>
{:else if isYtdlpPlatform(platform)}
  <button
    class="button formats-toggle-btn"
    onclick={onLoadFormats}
    disabled={loadingFormats}
    aria-busy={loadingFormats}
    aria-expanded={expanded && formats.length > 0}
  >
    {#if loadingFormats}
      <span class="feedback-spinner small-spinner"></span>
    {:else}
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4 6h16M4 12h16M4 18h16" />
      </svg>
    {/if}
    {#if loadingFormats}
      {$t('librebox.languages_loading')}
    {:else if formats.length > 0 && expanded}
      {$t('librebox.hide_formats')}
    {:else}
      {$t('librebox.view_formats')}
    {/if}
  </button>

  {#if formatError && formats.length === 0}
    <div class="formats-error" role="alert">
      <span class="formats-error-text">{formatError}</span>
      <button class="button formats-retry-btn" onclick={onLoadFormats}>
        {$t('librebox.retry')}
      </button>
    </div>
  {/if}

  {#if loadingFormats}
    <p class="multi-audio-note" role="status" aria-live="polite">{$t("librebox.languages_loading")}</p>
  {/if}

  {#if formats.length > 0 && expanded}
    <div class="formats-panel">
      <p class="multi-audio-note">{$t("librebox.languages_formats_hint")}</p>
      <div class="formats-presets" role="radiogroup" aria-label={$t('librebox.formats_presets_label')}>
        {#if bestVA || bestVideo}
          <button
            type="button"
            class="preset-pill"
            class:active={activePreset === "best"}
            onclick={applyBest}
            role="radio"
            aria-checked={activePreset === "best"}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polygon points="5 3 19 12 5 21 5 3" />
            </svg>
            <span>{$t('librebox.preset_best')}</span>
          </button>
        {/if}
        {#if hasMusicPreset}
          <button
            type="button"
            class="preset-pill"
            class:active={activePreset === "music"}
            onclick={applyMusic}
            role="radio"
            aria-checked={activePreset === "music"}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M9 18V5l12-2v13" />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
            <span>{$t('librebox.preset_music')}</span>
          </button>
        {/if}
        <button
          type="button"
          class="preset-pill"
          class:active={activePreset === "custom"}
          onclick={toggleCustom}
          role="radio"
          aria-checked={activePreset === "custom"}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
          <span>{$t('librebox.preset_custom')}</span>
        </button>
      </div>

      {#if !isPlaylist && multitrackLanguages.length > 1}
        <div class="audio-track-row">
          <button type="button" class="preset-pill" class:active={multipleAudio} aria-pressed={multipleAudio} onclick={toggleMultiple}>{$t('librebox.multi_audio')}</button>
          {#if multipleAudio}
            <p class="multi-audio-note">{$t('librebox.multi_audio_hint')}</p>
            <div class="audio-track-pills">
              {#each multitrackLanguages as language (language)}
                <button type="button" class="preset-pill" class:active={selectedAudioLanguages.includes(language)} aria-pressed={selectedAudioLanguages.includes(language)} onclick={() => toggleAudio(language)}>{formatAudioLanguage(language) ?? language}</button>
              {/each}
            </div>
            <p class="multi-audio-note">{$t('librebox.multi_audio_selected', { count: selectedAudioLanguages.length })}</p>
          {/if}
        </div>
      {/if}
      {#if audioLanguages.length > 1 && !multipleAudio}
        <div class="audio-track-row">
          <span class="audio-track-label">{$t('librebox.audio_track')}</span>
          <div class="audio-track-pills">
            <button
              type="button"
              class="preset-pill"
              class:active={selectedAudioLanguage === null}
              onclick={() => onSelectAudioLanguage(null)}
            >
              <span>{$t('librebox.audio_track_default')}</span>
            </button>
            {#each audioLanguages as lang (lang)}
              <button
                type="button"
                class="preset-pill"
                class:active={selectedAudioLanguage === lang}
                onclick={() => onSelectAudioLanguage(lang)}
              >
                <span>{formatAudioLanguage(lang) ?? lang}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      {#if selectedFormatId}
        <div class="format-selected">
          <span class="format-selected-label">{selectedFormatLabel}</span>
          <button class="format-clear-btn" onclick={() => { onClearFormat(); activePreset = null; }} aria-label={$t('common.close')}>
            <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      {:else}
        <div class="formats-info">
          <span class="formats-note">
            {$t('librebox.formats_merge_note')}
          </span>
        </div>
      {/if}

      {#if activePreset === "custom"}
        <div class="formats-list">
          <div class="format-header">
            <span>ID</span>
            <span>{$t('librebox.fmt_type')}</span>
            <span>{$t('librebox.fmt_quality')}</span>
            <span>{$t('librebox.fmt_streams')}</span>
            <span>{$t('librebox.fmt_video')}</span>
            <span>{$t('librebox.fmt_audio')}</span>
            <span>{$t('librebox.fmt_size')}</span>
            <span>{$t('librebox.fmt_speed')}</span>
          </div>
          {#each formats as fmt (fmt.format_id)}
            <button
              class="format-row"
              class:format-row-selected={selectedFormatId === fmt.format_id}
              onclick={() => onSelectFormat(fmt.format_id)}
            >
              <span class="format-id">{fmt.format_id}</span>
              <span class="format-ext">{fmt.ext}</span>
              <span class="format-res">{fmt.resolution ?? "—"}</span>
              <span class="format-codec">
                {#if fmt.has_video && fmt.has_audio}
                  V+A
                {:else if fmt.has_video}
                  V
                {:else if fmt.has_audio}
                  A
                {:else}
                  —
                {/if}
              </span>
              <span class="format-vcodec">{formatCodec(fmt.vcodec)}</span>
              <span class="format-acodec">
                {formatCodec(fmt.acodec)}{#if formatAudioLanguage(fmt.audio_language)}<span class="format-lang"> · {formatAudioLanguage(fmt.audio_language)}</span>{/if}
              </span>
              <span class="format-size">{formatFilesize(fmt.filesize, fmt.filesize_is_estimate)}</span>
              {#if fmt.tbr}
                <span class="format-tbr">{fmt.tbr.toFixed(0)}k</span>
              {:else}
                <span class="format-tbr">—</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .multi-audio-note { font-size: 12px; line-height: 1.6; margin: 4px 0; color: var(--secondary); overflow-wrap: anywhere; }
  .formats-toggle-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    min-height: 48px;
    box-sizing: border-box;
    padding: 12px 14px;
    margin-top: 6px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-base);
    font-weight: 600;
    line-height: 1.45;
    white-space: normal;
    text-align: center;
    cursor: pointer;
  }
  .formats-toggle-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  .formats-toggle-btn:disabled { opacity: 0.75; cursor: wait; }

  .formats-playlist-note {
    padding: calc(var(--padding) / 2) var(--padding);
    background: var(--button-elevated);
    border-radius: calc(var(--border-radius) - 2px);
    border-left: 3px solid var(--cta);
    font-size: var(--text-sm);
    color: var(--secondary);
    line-height: 1.4;
  }

  .small-spinner {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
  }

  .formats-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--padding);
    padding: calc(var(--padding) / 2) var(--padding);
    background: var(--button-elevated);
    border-radius: calc(var(--border-radius) - 2px);
    border-left: 3px solid var(--danger);
  }

  .formats-error-text {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--secondary);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    overflow-wrap: anywhere;
    white-space: normal;
  }

  .formats-retry-btn {
    font-size: var(--text-sm);
    padding: 4px 10px;
    flex-shrink: 0;
  }

  .formats-panel {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) / 2);
    width: 100%;
    background: var(--button);
    border-radius: var(--border-radius);
    box-shadow: var(--button-box-shadow);
    padding: var(--padding);
    box-sizing: border-box;
    overflow: visible;
  }

  .audio-track-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
  }
  .audio-track-label {
    font-size: 12px;
    opacity: 0.65;
  }
  .audio-track-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .formats-presets {
    display: flex;
    gap: calc(var(--padding) / 2);
    flex-wrap: wrap;
  }

  .preset-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
    background: var(--button);
    border: none;
    border-radius: 999px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  @media (hover: hover) {
    .preset-pill:hover {
      background: var(--button-hover);
      border-color: var(--content-border);
    }
  }

  .preset-pill:active {
    background: var(--button-press);
  }

  .preset-pill.active {
    background: var(--cta);
    border-color: var(--cta);
    color: var(--on-cta);
  }

  .preset-pill:focus-visible {
    outline: 2px solid var(--accent, var(--secondary));
    outline-offset: 2px;
  }

  .formats-info {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .formats-note {
    font-size: 11px;
    color: var(--gray);
    text-align: center;
    line-height: 1.4;
  }

  .formats-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    background: var(--button);
    border-radius: var(--border-radius);
    box-shadow: var(--button-box-shadow);
    scrollbar-width: none;
  }

  .formats-list::-webkit-scrollbar {
    display: none;
  }

  .format-header {
    display: grid;
    grid-template-columns: 48px 48px 80px 32px 64px 64px 64px 48px;
    gap: 2px;
    padding: calc(var(--padding) / 2) calc(var(--padding) / 2 + 4px);
    font-size: 10px;
    font-weight: 600;
    color: var(--gray);
    opacity: 0.7;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: none;
    position: sticky;
    top: 0;
    background: var(--button);
  }

  .format-row {
    display: grid;
    grid-template-columns: 48px 48px 80px 32px 64px 64px 64px 48px;
    align-items: center;
    gap: 2px;
    padding: calc(var(--padding) / 2) calc(var(--padding) / 2 + 4px);
    font-size: 11px;
    font-weight: 500;
    color: var(--gray);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    border-bottom: none;
  }

  .format-row:last-child {
    border-bottom: none;
  }

  @media (hover: hover) {
    .format-row:hover {
      background: var(--button-hover);
      color: var(--secondary);
    }
  }

  .format-row:active {
    background: var(--button-press);
  }

  .format-row-selected {
    background: var(--button-elevated);
    color: var(--secondary);
  }

  .format-lang {
    color: var(--tertiary);
  }

  .format-id {
    color: var(--secondary);
    font-weight: 500;
  }

  .format-ext {
    color: var(--accent);
  }

  .format-res {
    color: var(--secondary);
  }

  .format-codec {
    text-align: center;
  }

  .format-selected {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--padding);
    padding: var(--padding);
    background: var(--button-elevated);
    border-radius: calc(var(--border-radius) - 2px);
    font-size: 13px;
  }

  .format-selected-label {
    color: var(--secondary);
    font-weight: 500;
    flex: 1;
  }

  .format-clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--secondary);
    padding: 0;
  }

  @media (max-width: 535px) {
    .format-header,
    .format-row {
      grid-template-columns: 40px 40px 64px 24px 56px 56px 56px 40px;
      font-size: 10px;
    }

    .format-header {
      font-size: 9px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
  }
</style>
