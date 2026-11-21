<script lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { showToast } from "$lib/stores/toast-store.svelte";
async function contact() {
  try { await openUrl("https://mohamed-samir-profile.mo2868417.workers.dev/?v=m"); }
  catch (e) { showToast("error", String(e)); }
}
</script>
  <section class="developer" dir="rtl" aria-label="المطور">
   <div class="card-top"><div class="avatar"><img class="portrait" src="/developer/mohamed.jpg" alt="صورة محمد سمير" /></div><div class="identity"><div class="eyebrow">المطور هو</div><h2><span class="honorific">الباشمهندس</span><span>محمد سمير</span></h2><p>مهندس ومطور تطبيق omniget</p></div><span class="tiny-code">&lt;/&gt;</span></div>
   <div class="card-bottom"><p>عندك مشكلة هنا أو فكرة مشروع في بالك؟ تواصل معايا <img class="phone-emoji" src="/developer/phone-emoji.png" alt="📞"/></p><button onclick={contact}>تواصل معايا <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M8 4H4v12h12v-4M11 3h6v6M17 3l-8 8"/></svg></button></div>
   <p class="prayer">يا ريت دعوة منك للباشمهندس محمد سمير، مطور التطبيق</p>
  </section>

<style>
@font-face{font-family:Tajawal;src:url(/developer/tajawal-regular.ttf);font-weight:400;font-display:block}@font-face{font-family:Tajawal;src:url(/developer/tajawal-medium.ttf);font-weight:500;font-display:block}
.developer{font-family:Tajawal,Arial,sans-serif!important}.developer h2{font-weight:500!important;font-size:23px!important}.developer button{font-family:inherit!important;font-weight:500!important}.developer .identity p{font-size:13px!important}.developer .card-bottom p{font-size:12px!important}
.developer{width:100%;box-sizing:border-box;padding:20px;border:1px solid #343034;border-radius:18px;background:linear-gradient(135deg,#222024,#19191d);box-shadow:0 8px 28px #0002;position:relative}.developer:before{content:'';position:absolute;top:18px;right:0;width:2px;height:38px;background:#d9a577;border-radius:2px}.card-top{display:flex;align-items:center;gap:13px}.avatar{width:62px;height:62px;flex-shrink:0;border:1px dashed #5b5051;background:#30292a;border-radius:15px;display:flex;flex-direction:column;align-items:center;justify-content:center;color:#c2a98f;gap:0}.avatar .portrait{width:100%;height:100%;object-fit:contain;border-radius:14px}.identity{flex:1}.eyebrow{font-size:10px;color:#c7a787;letter-spacing:.2px;margin-bottom:5px}h2{font-size:21px;margin:0 0 5px;font-weight:600;letter-spacing:-.3px}h2 span{display:block}.honorific{font-size:12px;font-weight:400;color:#c2b8b1;margin-bottom:3px}.identity p{margin:0;font-size:12px;color:#aaa4aa}.tiny-code{color:#786458;font-family:monospace;font-size:16px;align-self:flex-start;padding-top:4px}.card-bottom{border-top:1px solid #ffffff0c;margin-top:17px;padding-top:15px;display:flex;align-items:center;justify-content:space-between;gap:9px}.phone-emoji{width:12px;height:12px;display:inline-block;vertical-align:-2px;margin-left:3px}.card-bottom p{color:#8b858c;font-size:10px;margin:0;line-height:1.6;max-width:175px}.card-bottom button{padding:9px 12px;border-radius:8px;background:#e7bd94;color:#271d17;display:flex;gap:7px;align-items:center;border:0;font-size:12px;font-weight:600;white-space:nowrap}
.developer{max-width:700px;margin:20px auto;flex-shrink:0}.prayer{text-align:center;font-size:13px;color:var(--text-secondary,#aaa);line-height:1.8;margin:14px 0 0}.developer h2{color:#f4f4f5}
@media(max-width:380px){.developer{padding:16px}.card-bottom{flex-wrap:wrap}.card-bottom p{max-width:none}}</style>
