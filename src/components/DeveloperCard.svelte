<script lang="ts">
import DeveloperProfileDialog from "./DeveloperProfileDialog.svelte";
import { openDeveloperContact } from "$lib/developer-contact";
import { showToast } from "$lib/stores/toast-store.svelte";
let { compact = false }: { compact?: boolean } = $props();
let collapsed = $state(false);
let profileOpen = $state(false);
async function contact() {
  try { await openDeveloperContact(); }
  catch (e) { showToast("error", String(e)); }
}
</script>
<section class="developer" class:collapsed class:compact dir="rtl" aria-label="المطور">
  <button class="profile-trigger" type="button" aria-label="عرض معلومات المطور محمد سمير" aria-haspopup="dialog" onclick={() => profileOpen = true}></button>
  <button class="toggle" type="button" aria-label={collapsed ? "توسيع كارت المطور" : "تصغير كارت المطور"} aria-expanded={!collapsed} onclick={() => collapsed = !collapsed}>
    <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d={collapsed ? "m5 12 5-5 5 5" : "m5 8 5 5 5-5"}/></svg>
  </button>
  <div class="card-top">
    <img class="portrait" src="/developer/mohamed.jpg" alt="صورة محمد سمير" />
    {#if !collapsed}<div class="identity"><div class="eyebrow">المطور هو</div><h2><span class="honorific">الباشمهندس</span> محمد سمير</h2><p>مهندس ومطور تطبيق omniget</p></div>{/if}
  </div>
  <div class="card-bottom">
    <p>عندك مشكلة هنا أو فكرة مشروع في بالك؟ تواصل معايا <img class="phone-emoji" src="/developer/phone-emoji.png" alt="📞"/></p>
    <button class="contact" type="button" onclick={contact}>تواصل معايا <svg viewBox="0 0 20 20" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M8 4H4v12h12v-4M11 3h6v6M17 3l-8 8"/></svg></button>
  </div>
</section>
{#if profileOpen}<DeveloperProfileDialog onClose={() => profileOpen = false} />{/if}
<style>
@font-face{font-family:Tajawal;src:url(/developer/tajawal-regular.ttf);font-weight:400;font-display:block}
@font-face{font-family:Tajawal;src:url(/developer/tajawal-medium.ttf);font-weight:500;font-display:block}
.developer{font-family:Tajawal,Arial,sans-serif!important;width:100%;max-width:700px;box-sizing:border-box;margin:16px auto;padding:14px;border:1px solid var(--border,#ddd);border-radius:14px;background:var(--surface,#fff);color:var(--secondary,#222);position:relative;flex-shrink:0}
.profile-trigger{position:absolute;inset:0;z-index:0;border:0!important;border-radius:inherit;background:transparent!important;box-shadow:none!important;padding:0!important;cursor:pointer}.profile-trigger:focus-visible{outline:2px solid var(--accent);outline-offset:2px}.card-top,.card-bottom{position:relative;z-index:1;pointer-events:none}.card-bottom .contact{pointer-events:auto}.developer .toggle{z-index:2}
.developer button{font-family:inherit!important;font-weight:500!important;color:inherit}
.toggle{position:absolute;inset-inline-end:6px;top:6px;width:44px;height:44px;display:grid;place-items:center;border:0;border-radius:8px;background:transparent;cursor:pointer}
.developer .toggle{border:0!important;background:transparent!important;box-shadow:none!important}.toggle:hover svg{opacity:.65}.toggle:active svg{transform:scale(.9)}.toggle:focus-visible,.contact:focus-visible{outline:2px solid var(--accent,#ad743c);outline-offset:2px}
.card-top{display:flex;align-items:center;gap:10px;padding-inline-end:44px;min-width:0}
.portrait{width:48px;height:48px;object-fit:contain;border:1px dashed var(--border,#ddd);border-radius:10px;flex-shrink:0}
.identity{min-width:0;text-align:right}.eyebrow{font-size:10px;color:var(--accent,#ad743c);margin-bottom:2px}
h2{font-size:16px!important;line-height:1.4;margin:0;font-weight:500!important;color:inherit!important}.honorific{font-size:12px}
.identity p{margin:2px 0 0;font-size:11px;color:var(--text-muted,#666);line-height:1.5}
.card-bottom{display:flex;align-items:center;gap:12px;justify-content:space-between;margin-top:12px;padding-top:10px;border-top:1px solid var(--border,#ddd)}
.card-bottom p{font-size:12px;line-height:1.6;margin:0;color:var(--text-muted,#666);overflow-wrap:anywhere}
.phone-emoji{width:11px;height:11px;display:inline-block;vertical-align:-1px}
.contact{padding:8px 10px;min-height:36px;border-radius:8px;background:var(--surface-hi,#eee);border:1px solid var(--border,#ddd);display:flex;align-items:center;justify-content:center;gap:5px;white-space:nowrap;font-size:12px;cursor:pointer;flex-shrink:0}
.collapsed{display:flex;align-items:center;gap:8px;padding:10px;padding-inline-end:56px;border-radius:12px}
.collapsed .toggle{top:50%;transform:translateY(-50%)}.collapsed .card-top{padding:0}.collapsed .portrait{width:24px;height:24px;border-radius:7px}
.collapsed .card-bottom{flex:1;min-width:0;flex-wrap:nowrap;gap:8px;margin:0;padding:0;border:0}.collapsed .card-bottom p{min-width:0;font-size:11px;line-height:1.45}.collapsed .contact{font-size:11px;padding:7px 8px;min-height:34px}.collapsed .contact svg{display:none}
@media(max-width:360px){.developer{padding:12px}.collapsed{padding:9px;padding-inline-end:54px;gap:6px}.collapsed .card-bottom{gap:6px}.collapsed .contact{padding:6px}.collapsed .portrait{width:22px;height:22px}.card-bottom{gap:8px}}
@media(max-width:768px){
.compact:not(.collapsed){width:calc(100% - 24px);max-width:560px;padding:14px;margin:12px auto 20px}
.compact:not(.collapsed) .portrait{width:48px;height:48px}
.compact:not(.collapsed) .eyebrow{display:block}
.compact:not(.collapsed) h2{font-size:16px!important}
.compact:not(.collapsed) .card-bottom{margin-top:12px;padding-top:10px;gap:10px}
.compact:not(.collapsed) .card-bottom p{font-size:12px;line-height:1.6}
}
</style>
