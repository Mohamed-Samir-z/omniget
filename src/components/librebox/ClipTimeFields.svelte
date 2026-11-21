<script lang="ts">
  import { t } from "$lib/i18n";
  import { splitTime, joinTime, timeSeconds } from "$lib/clip-time";

  let { value = $bindable(""), label, disabled = false, onchange = undefined }:
    { value?: string; label: string; disabled?: boolean; onchange?: (value: string) => void } = $props();
  let parts = $state<string[]>(["", "", ""]);
  let lastEmitted: string | undefined;
  $effect(() => {
    if (value !== lastEmitted) {
      parts = splitTime(value);
      lastEmitted = value;
    }
  });
  const invalid = $derived(!!value && timeSeconds(value) === null);
  function change(index: number, text: string) {
    const next = [...parts];
    next[index] = text;
    parts = next;
    lastEmitted = joinTime(next);
    value = lastEmitted;
    onchange?.(value);
  }
</script>

<fieldset class="clip-time" {disabled} aria-invalid={invalid}>
  <legend>{label}</legend>
  <div class="clock">
    {#each ['hours', 'minutes', 'seconds'] as unit, index}
      {#if index > 0}<span class="colon" aria-hidden="true">:</span>{/if}
      <label>
        <span>{$t(`clip.${unit}` as 'clip.hours' | 'clip.minutes' | 'clip.seconds')}</span>
        <input type="text" inputmode={index === 2 ? 'decimal' : 'numeric'}
          placeholder="00" value={parts[index]} maxlength={index === 0 ? 4 : index === 1 ? 2 : 6}
          aria-label={`${label}: ${$t(`clip.${unit}` as 'clip.hours' | 'clip.minutes' | 'clip.seconds')}`}
          aria-invalid={invalid} autocomplete="off" spellcheck="false"
          oninput={(event) => change(index, event.currentTarget.value)} />
      </label>
    {/each}
  </div>
</fieldset>

<style>
  .clip-time { border: 0; padding: 0; margin: 0; min-width: 0; flex: 1 1 150px; }
  legend { padding: 0; margin-bottom: 6px; color: var(--text-muted, #aaa); font-size: 12px; }
  .clock { display: flex; gap: 4px; align-items: flex-end; }
  label { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 4px; }
  label span { font-size: 10px; color: var(--text-muted, #aaa); white-space: nowrap; }
  input { box-sizing: border-box; width: 100%; min-width: 0; min-height: 44px; padding: 8px 3px; text-align: center; border: 1px solid var(--border, #555); border-radius: 8px; background: var(--fill-2, #191919); color: var(--text, #eee); font: 15px ui-monospace, monospace; }
  input[aria-invalid='true'] { border-color: var(--danger, #e86969); }
  input:focus-visible { outline: 2px solid var(--accent-hi, #c1f26f); outline-offset: 1px; }
  .colon { line-height: 44px; color: var(--text-muted, #aaa); }
</style>
