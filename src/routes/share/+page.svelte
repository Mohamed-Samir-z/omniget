<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import {
    getPendingExternalShare,
    clearPendingExternalShare,
    queueExternalPrefill,
  } from "$lib/stores/external-url-store.svelte";
  import { getSettings } from "$lib/stores/settings-store.svelte";

  type DownloadStarted = {
    id: number;
    title: string;
  };

  let sharedUrl = $state("");
  let hostLabel = $state("");
  let mode = $state<"video" | "audio" | "mp3">("video");
  let quality = $state("best");
  let scheduledAt = $state("");
  let busy = $state(false);
  let done = $state(false);
  let error = $state("");

  const qualityOptions = ["best", "1080p", "720p", "480p", "360p"];

  onMount(() => {
    const share = getPendingExternalShare();
    if (!share) {
      goto("/");
      return;
    }
    clearPendingExternalShare(share.id);
    sharedUrl = share.url;
    try {
      hostLabel = new URL(share.url).hostname.replace(/^www\./, "");
    } catch {
      hostLabel = "";
    }
  });

  function qualityLabel(value: string): string {
    if (value === "best") return $t("librebox.quality_best") as string;
    return value;
  }

  async function startDownload() {
    if (busy || done) return;
    error = "";
    const outputDir = getSettings()?.download.default_output_dir ?? "";
    if (!outputDir) {
      error = $t("share.no_output_dir") as string;
      return;
    }
    if (scheduledAt && (!Number.isFinite(new Date(scheduledAt).getTime()) || new Date(scheduledAt).getTime() <= Date.now())) {
      error = $t("schedule_future_required") as string;
      return;
    }
    busy = true;
    try {
      await invoke<DownloadStarted>("download_from_url", {
        url: sharedUrl,
        outputDir,
        downloadMode: mode === "video" ? null : mode,
        quality,
        formatId: null,
        referer: null,
        cookieSlug: null,
        timeRange: null,
        audioLanguage: null,
        playlistItems: null,
        torrentFiles: null,
        scheduledAt: scheduledAt ? new Date(scheduledAt).getTime() : null,
        stopAt: null,
      });
      done = true;
    } catch (e: any) {
      error = typeof e === "string" ? e : e?.message ?? ($t("share.failed") as string);
    } finally {
      busy = false;
    }
  }

  function openInApp() {
    queueExternalPrefill({ url: sharedUrl, source: "android-share", action: "prefill" });
    goto("/");
  }
</script>

<svelte:head>
  <title>{$t("share.title")}</title>
</svelte:head>

<div class="share-shell">
  <div class="share-card">
    <div class="share-header">
      <span class="share-title">{$t("share.title")}</span>
      {#if hostLabel}
        <span class="share-host">{hostLabel}</span>
      {/if}
    </div>
    <span class="share-url" title={sharedUrl}>{sharedUrl}</span>

    {#if done}
      <div class="share-done">
        <span class="share-done-text">{$t("share.added")}</span>
        <button class="share-secondary-btn" onclick={() => goto("/downloads")}>{$t("share.open_downloads")}</button>
        <button class="share-secondary-btn" onclick={() => goto("/")}>{$t("share.back_home")}</button>
      </div>
    {:else}
      <div class="share-row">
        <span class="share-label">{$t("share.type")}</span>
        <div class="share-pills">
          <button class="share-pill" class:active={mode === "video"} onclick={() => (mode = "video")}>{$t("share.video")}</button>
          <button class="share-pill" class:active={mode === "audio"} onclick={() => (mode = "audio")}>{$t("share.audio")}</button>
          <button class="share-pill" class:active={mode === "mp3"} onclick={() => (mode = "mp3")}>MP3</button>
        </div>
      </div>

      {#if mode === "video"}
        <div class="share-row">
          <span class="share-label">{$t("librebox.quality")}</span>
          <div class="share-pills">
            {#each qualityOptions as q}
              <button class="share-pill" class:active={quality === q} onclick={() => (quality = q)}>{qualityLabel(q)}</button>
            {/each}
          </div>
        </div>
      {/if}

      <details class="share-schedule">
        <summary>{$t("librebox.schedule_label")}</summary>
        <input type="datetime-local" bind:value={scheduledAt} aria-label={$t("librebox.schedule_start") as string} />
        <p>{$t("schedule_app_required")}</p>
      </details>

      {#if error}
        <span class="share-error">{error}</span>
      {/if}

      <div class="share-actions">
        <button class="share-primary-btn" disabled={busy} onclick={startDownload}>
          {busy ? $t("share.starting") : $t("librebox.download")}
        </button>
        <button class="share-secondary-btn" onclick={openInApp}>{$t("share.more_options")}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .share-schedule { font-size: 13px; color: var(--text-muted); }
  .share-schedule input { width: 100%; margin-top: 8px; padding: 10px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 8px; }
  .share-schedule p { margin: 8px 0 0; line-height: 1.4; }
  .share-shell {
    min-height: 0;
    flex: 1;
    width: 100%;
    overflow-y: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: transparent;
  }
  .share-card {
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    border-radius: 18px;
    background: var(--surface, #1c1c1f);
    border: 1px solid var(--border, rgba(255, 255, 255, 0.08));
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  }
  .share-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }
  .share-title {
    font-size: 16px;
    font-weight: 700;
  }
  .share-host {
    font-size: 12px;
    opacity: 0.6;
  }
  .share-url {
    font-size: 12px;
    opacity: 0.55;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .share-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .share-label {
    font-size: 12px;
    opacity: 0.65;
  }
  .share-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .share-pill {
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    background: transparent;
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .share-pill.active {
    background: var(--accent, #4f8cff);
    border-color: var(--accent, #4f8cff);
    color: #fff;
  }
  .share-error {
    font-size: 12px;
    color: var(--danger, #ff6b6b);
  }
  .share-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 4px;
  }
  .share-primary-btn {
    padding: 12px;
    border-radius: 12px;
    border: none;
    background: var(--accent, #4f8cff);
    color: #fff;
    font-size: 15px;
    font-weight: 700;
    cursor: pointer;
  }
  .share-primary-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .share-secondary-btn {
    padding: 10px;
    border-radius: 12px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    background: transparent;
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .share-done {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .share-done-text {
    font-size: 14px;
    font-weight: 600;
    color: var(--success, #34c759);
  }
</style>
