<script lang="ts">
  import { t } from "$lib/i18n";
  import { isAndroid } from "$lib/platform";
  import LibreboxInput from "$components/librebox/LibreboxInput.svelte";
  import type { HomeInputMode } from "$lib/home/librebox-controller";

  let {
    url = $bindable(""),
    mode = $bindable<HomeInputMode>("url"),
    variant = "bar",
    advanced = false,
    onInput,
    onModeChange,
    onAdvanced,
  }: {
    url?: string;
    mode?: HomeInputMode;
    variant?: "bar" | "stage";
    advanced?: boolean;
    onInput: () => void;
    onModeChange?: (mode: HomeInputMode) => void;
    onAdvanced?: () => void;
  } = $props();

  function pick(next: HomeInputMode) {
    mode = next;
    onModeChange?.(next);
  }
</script>

<div class="home-url-bar" class:stage={variant === "stage"}>
  {#if mode === "url"}
    <LibreboxInput
      bind:url
      onInput={onInput}
      prominent={variant === "stage"}
      {advanced}
      onModeSelect={pick}
      {onAdvanced}
    />
    {#if isAndroid() && variant === "stage"}<p class="home-input-help" id="home-input-help">{$t('home.url_hint')}</p>{/if}
  {/if}
</div>

<style>
  .home-input-help { margin: 2px 0 0; padding-inline: 6px; font-size: 11px; line-height: 1.6; color: var(--text-muted); text-align: center; overflow-wrap: anywhere; }

  .home-url-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
  }

  .home-url-bar.stage {
    align-items: center;
    max-width: 560px;
  }
</style>
