<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { onBackButtonPress } from "@tauri-apps/api/app";
  import type { PluginListener } from "@tauri-apps/api/core";
  import { isAndroid } from "$lib/platform";
  import { openDeveloperProfileLink, type DeveloperProfileLink } from "$lib/developer-contact";
  import { showToast } from "$lib/stores/toast-store.svelte";

  let { onClose }: { onClose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let closeButton: HTMLButtonElement;
  let previousFocus: HTMLElement | null = null;
  let backListener: PluginListener | undefined;
  let destroyed = false;
  let opening = $state(false);

  onMount(() => {
    previousFocus = document.activeElement as HTMLElement;
    dialog.showModal();
    closeButton.focus();
    if (isAndroid()) {
      void onBackButtonPress(() => onClose()).then(listener => {
        if (destroyed) void listener.unregister();
        else backListener = listener;
      }).catch(e => showToast("error", String(e)));
    }
  });
  onDestroy(() => {
    destroyed = true;
    if (backListener) void backListener.unregister();
    dialog?.close();
    previousFocus?.focus();
  });

  async function openLink(target: DeveloperProfileLink) {
    if (opening) return;
    opening = true;
    try { await openDeveloperProfileLink(target); }
    catch (e) { showToast("error", String(e)); }
    finally { opening = false; }
  }
</script>

<dialog bind:this={dialog} class="developer-profile-modal" dir="rtl" aria-modal="true" aria-labelledby="developer-profile-title" onclick={e => { if (e.target === dialog) onClose(); }} oncancel={e => { e.preventDefault(); onClose(); }}>
  <section class="profile-body">
    <button bind:this={closeButton} class="profile-close" type="button" aria-label="إغلاق نافذة المطور" onclick={onClose}>×</button>
    <img class="profile-photo" src="/developer/mohamed.jpg" alt="صورة محمد سمير" width="88" height="88" />
    <p class="profile-eyebrow">المطور هو</p>
    <h2 id="developer-profile-title">الباشمهندس <span class="profile-name">محمد سمير</span></h2>
    <p class="profile-role">مهندس ومطور تطبيق omniget</p>
    <p class="profile-bio">بشتغل على تطوير وتجهيز نسخة omniget لأندرويد. عندك مشكلة في التطبيق أو فكرة مشروع؟ تواصل معايا.</p>
    <div class="profile-links">
      <button class="profile-link primary" type="button" disabled={opening} onclick={() => openLink("whatsapp")}><span>تواصل على واتساب</span><span aria-hidden="true">↗</span></button>
      <button class="profile-link" type="button" disabled={opening} onclick={() => openLink("portfolio")}><span>البورتفوليو · أعمالي</span><span aria-hidden="true">↗</span></button>
      <button class="profile-link" type="button" disabled={opening} onclick={() => openLink("github")}><span>حسابي على GitHub</span><span aria-hidden="true">↗</span></button>
    </div>
    <footer class="profile-footer">
      <button class="project-link" type="button" disabled={opening} onclick={() => openLink("project")}>مشروع omniget على GitHub <span aria-hidden="true">↗</span></button>
      <p>مبني على LibreGet · OpenSelena</p>
    </footer>
  </section>
</dialog>

<style>
  .developer-profile-modal{font-family:Tajawal,Arial,sans-serif;width:calc(100% - 40px);max-width:360px;max-height:calc(100dvh - 40px);padding:0;overflow:auto;box-sizing:border-box;border:1px solid var(--border,#ddd);border-radius:22px;background:var(--surface,#fff);color:var(--text,#222);box-shadow:0 20px 70px #0004;text-align:center}
  .developer-profile-modal::backdrop{background:#0008;backdrop-filter:blur(3px)}
  .profile-body{position:relative;padding:28px 22px 20px;box-sizing:border-box}
  .developer-profile-modal button{font-family:inherit!important;cursor:pointer}
  .profile-close{position:absolute;left:7px;top:7px;width:44px;height:44px;border:0!important;background:transparent!important;box-shadow:none!important;color:var(--text-muted);font-size:23px;padding:0!important}
  .profile-photo{width:88px;height:88px;object-fit:contain;border-radius:20px;margin:0 auto 14px;display:block;border:1px solid var(--border);box-sizing:border-box}
  .profile-eyebrow{font-size:12px;color:var(--accent);margin:0 0 6px}
  h2{font-family:inherit!important;font-size:22px!important;line-height:1.5;font-weight:500!important;margin:0;color:inherit!important}.profile-name{white-space:nowrap}
  .profile-role{font-size:13px;color:var(--text-muted);margin:4px 0 16px}.profile-bio{font-size:14px;line-height:1.9;margin:0 auto 20px;max-width:260px;color:var(--text-muted)}
  .profile-links{display:grid;gap:9px}.profile-link{font-size:14px;min-height:46px;border-radius:11px;border:1px solid var(--border);background:var(--surface-hi);color:inherit;width:100%;display:flex;justify-content:space-between;align-items:center;padding:10px 15px;box-sizing:border-box}.profile-link.primary{border-color:var(--accent);color:var(--accent)}
  .profile-footer{margin-top:9px;font-size:11px;line-height:1.7;color:var(--text-muted)}.profile-footer p{margin:0}.project-link{display:block;min-height:44px;margin:0 auto;padding:8px 0!important;font-size:11px!important;border:0!important;background:transparent!important;box-shadow:none!important;color:var(--text)}
  button:focus-visible{outline:2px solid var(--accent);outline-offset:2px}button:hover{filter:brightness(.95)}button:disabled{cursor:wait;opacity:.65}
  @media(max-width:360px){h2{font-size:20px!important}}
</style>
