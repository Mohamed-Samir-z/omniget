<script lang="ts">
  import { onMount } from "svelte";
  import { onBackButtonPress } from "@tauri-apps/api/app";
  import type { PluginListener } from "@tauri-apps/api/core";
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { isAndroid } from "$lib/platform";
  import { getCounts } from "$lib/stores/download-store.svelte";
  import { getSettings } from "$lib/stores/settings-store.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { acknowledgeSupport, canOfferSupport, deferSupport } from "$lib/support-prompt";
  import { openDeveloperProfileLink } from "$lib/developer-contact";
  import DialogContainer from "./DialogContainer.svelte";
  let visible = $state(false);
  let opening = $state(false);
  let counts = $derived(getCounts());
  function check() {
    if (!isAndroid() || visible || page.url.pathname !== "/" || document.visibilityState !== "visible" || !getSettings()?.onboarding_completed || counts.active > 0 || counts.queued > 0) return;
    if (document.querySelector("dialog[open]")) return;
    if (document.activeElement?.matches("input,textarea,select")) return;
    if (document.querySelector<HTMLInputElement>("input.librebox")?.value.trim()) return;
    if (canOfferSupport()) { deferSupport(); visible = true; }
  }
  onMount(() => {
    const timer = setInterval(check, 30000);
    document.addEventListener("visibilitychange", check);
    check();
    return () => { clearInterval(timer); document.removeEventListener("visibilitychange", check); };
  });
  $effect(() => {
    if (!visible || !isAndroid()) return;
    let active = true;
    let back: PluginListener | undefined;
    void onBackButtonPress(() => later()).then(listener => { if (active) back = listener; else void listener.unregister(); }).catch(() => {});
    return () => { active = false; if (back) void back.unregister(); };
  });
  function later() { deferSupport(); visible = false; }
  function already() { acknowledgeSupport(); visible = false; }
  async function openProject() {
    if (opening) return;
    opening = true;
    try { await openDeveloperProfileLink("project"); later(); }
    catch (e) { showToast("error", String(e)); }
    finally { opening = false; }
  }
</script>

<DialogContainer isOpen={visible} titleId="support-prompt-title" onClose={later}>
  <section class="support-prompt">
    <button class="support-close" type="button" aria-label={$t("support.later")} onclick={later}>×</button>
    <span class="support-star" aria-hidden="true">☆</span>
    <h2 id="support-prompt-title">{$t("support.title")}</h2>
    <p>{$t("support.body")}</p>
    <button class="support-primary" type="button" disabled={opening} onclick={openProject}>{$t("support.open")}</button>
    <div class="support-actions"><button type="button" onclick={later}>{$t("support.later")}</button><button type="button" onclick={already}>{$t("support.already")}</button></div>
    <small>{$t("support.optional")}</small>
  </section>
</DialogContainer>

<style>
  .support-prompt{position:relative;padding:28px 22px 20px;text-align:center;box-sizing:border-box}
  .support-close{position:absolute;inset-inline-end:6px;top:6px;width:44px;height:44px;border:0!important;background:transparent!important;box-shadow:none!important;font-size:22px;color:var(--text-muted)}
  .support-star{display:block;font-size:40px;line-height:1;color:var(--accent);margin-bottom:15px}
  h2{font-size:21px!important;line-height:1.4;margin:0 0 10px;color:var(--text)!important}p{font-size:13px;line-height:1.8;color:var(--text-muted);margin:0 0 20px}
  button{font:inherit;cursor:pointer}.support-primary{width:100%;min-height:46px;padding:9px 14px;border-radius:10px;border:1px solid var(--accent);color:var(--accent);background:var(--surface-hi);font-size:13px}
  .support-actions{display:flex;justify-content:center;gap:8px;flex-wrap:wrap;margin-top:8px}.support-actions button{min-height:44px;padding:8px 10px;border:0!important;box-shadow:none!important;background:transparent!important;font-size:12px;color:var(--text-muted)}
  small{display:block;margin-top:8px;font-size:10px;line-height:1.7;color:var(--text-dim)}button:focus-visible{outline:var(--focus-ring);outline-offset:2px}
</style>
