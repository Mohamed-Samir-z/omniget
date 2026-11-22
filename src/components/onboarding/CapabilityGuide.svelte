<script lang="ts">
 import { onMount } from "svelte";
 import { onBackButtonPress } from "@tauri-apps/api/app";
 import { locale, loadTranslations, SUPPORTED_LOCALES } from "$lib/i18n";
 import { updateSettings } from "$lib/stores/settings-store.svelte";
 import { completeOnboarding, needsOnboarding } from "$lib/stores/onboarding-store.svelte";
 import { closeGuide } from "$lib/stores/guide-store.svelte";
 import { showToast } from "$lib/stores/toast-store.svelte";
 import { guideEnglish } from "$lib/guide-content";
 import guideArabic from "$lib/guide-ar.json";
 import DialogContainer from "../dialog/DialogContainer.svelte";
 let query=$state("");
 let busy=$state(false);
 const names:Record<string,string>={ar:"العربية",en:"English",ru:"Русский",el:"Ελληνικά",pt:"Português",zh:"中文","zh-TW":"繁體中文",ja:"日本語",it:"Italiano",fr:"Français",es:"Español",fa:"فارسی",lo:"ລາວ"};
 const ar=$derived($locale==="ar");
 const content=$derived(ar?guideArabic:guideEnglish);
 const results=$derived(content.filter(section=>(section.title+" "+section.body).toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())));
 let expanded=$state<string[]>(["guide-1"]);
 const copy=$derived(ar?{title:"دليلك لـ omniget",intro:"كل اللي تقدر تعمله على أندرويد، بمكانه وطريقته وحدوده.",language:"لغة التطبيق",search:"دور في الدليل، زي ترجمة أو قائمة تشغيل",sections:"قسم",expand:"فتح الكل",collapse:"قفل الكل",start:"ابدأ الاستخدام",close:"إغلاق",again:"تقدر ترجع للدليل من عن التطبيق في أي وقت.",none:"مفيش نتيجة. جرّب كلمة تانية."}:{title:"Your omniget guide",intro:"What you can do on Android, where to find it, and what to expect.",language:"App language",search:"Search the guide, e.g. captions or playlists",sections:"sections",expand:"Expand all",collapse:"Collapse all",start:"Start using omniget",close:"Close",again:"Reopen this guide from About at any time.",none:"No matches. Try another word."});
 async function finish(){
  if(busy)return;busy=true;
  try{if(needsOnboarding())await completeOnboarding();closeGuide();}
  catch(e){showToast("error",String(e));}
  finally{busy=false;}
 }
 async function language(event:Event){
  const language=(event.target as HTMLSelectElement).value;
  try{await updateSettings({appearance:{language}});await loadTranslations(language,"/");locale.set(language);query="";}
  catch(e){showToast("error",String(e));}
 }
 function toggle(id:string){expanded=expanded.includes(id)?expanded.filter(x=>x!==id):[...expanded,id];}
 onMount(()=>{
  let live=true;let unregister:(()=>Promise<void>)|undefined;
  void onBackButtonPress(()=>{void finish();}).then(listener=>{if(live)unregister=()=>listener.unregister();else void listener.unregister();}).catch(()=>{});
  return()=>{live=false;if(unregister)void unregister();};
 });
</script>
<DialogContainer isOpen={true} titleId="capability-guide-title" onClose={()=>{void finish();}}>
 <section class="capability-guide" dir={ar?"rtl":"ltr"} lang={ar?"ar":"en"}>
  <header class="guide-header">
   <div class="guide-heading"><span class="guide-symbol" aria-hidden="true">☰</span><span class="guide-edition">omniget · Android</span><button type="button" class="guide-close" onclick={finish} aria-label={copy.close}>×</button></div>
   <h2 id="capability-guide-title">{copy.title}</h2><p>{copy.intro}</p>
   <label class="guide-language" for="guide-language"><span>{copy.language}</span><select id="guide-language" value={$locale} onchange={language}>{#each SUPPORTED_LOCALES as code}<option value={code}>{names[code]||code}</option>{/each}</select></label>
   {#if !ar && $locale!=="en"}<p class="guide-fallback">This guide is available in Arabic and English. English is shown here; your app language stays selected.</p>{/if}
   <input class="guide-search" type="search" bind:value={query} aria-label={copy.search} placeholder={copy.search}/>
   <div class="guide-tools"><span>{results.length} {copy.sections}</span><button type="button" onclick={()=>expanded=results.map(s=>s.id)}>{copy.expand}</button><button type="button" onclick={()=>expanded=[]}>{copy.collapse}</button></div>
  </header>
  <div class="guide-sections">
   {#each results as section,index (section.id)}
    <article class="guide-section"><h3><button type="button" aria-expanded={expanded.includes(section.id)} aria-controls={section.id} onclick={()=>toggle(section.id)}><span class="guide-number">{String(content.findIndex(s=>s.id===section.id)+1).padStart(2,"0")}</span><span>{section.title}</span><span class="guide-chevron" aria-hidden="true">{expanded.includes(section.id)?"−":"+"}</span></button></h3>{#if expanded.includes(section.id)}<div id={section.id} class="guide-copy">{section.body}</div>{/if}</article>
   {:else}<p class="guide-empty">{copy.none}</p>{/each}
  </div>
  <footer class="guide-footer"><p>{copy.again}</p><button type="button" class="guide-start" onclick={finish} disabled={busy}>{needsOnboarding()?copy.start:copy.close}</button></footer>
 </section>
</DialogContainer>
<style>
 .capability-guide{display:flex;flex-direction:column;max-height:80dvh;min-height:300px;overflow:hidden;color:var(--text);font-family:inherit}.guide-header{padding:19px 20px 0;flex-shrink:0;background:var(--popup-bg)}.guide-heading{display:flex;align-items:center;gap:9px}.guide-symbol{font-size:18px;color:var(--accent)}.guide-edition{font-size:10px;letter-spacing:.04em;color:var(--text-muted)}button,select,input{font:inherit}button{cursor:pointer}.guide-close{margin-inline-start:auto;width:40px;height:40px;border:0!important;box-shadow:none!important;background:transparent!important;font-size:23px;color:var(--text-muted)}h2{font-size:24px!important;line-height:1.45;margin:3px 0 5px;color:inherit!important;font-weight:600!important}.guide-header>p{font-size:12px;line-height:1.8;color:var(--text-muted);margin:0 0 14px}.guide-language{display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:11px;color:var(--text-muted);margin-bottom:12px}.guide-language select{max-width:65%;min-height:38px;border:1px solid var(--border);border-radius:9px;background:var(--surface-hi);color:var(--text);padding:6px 10px}.guide-search{box-sizing:border-box;width:100%;min-height:42px;border-radius:10px;border:1px solid var(--border);background:var(--surface-hi);padding:9px 11px;font-size:11px;color:var(--text)}.guide-tools{display:flex;align-items:center;gap:7px;padding:8px 0;font-size:10px;color:var(--text-muted)}.guide-tools>span{margin-inline-end:auto}.guide-tools button{min-height:36px;padding:5px 6px;border:0!important;box-shadow:none!important;background:transparent!important;color:var(--accent);font-size:10px}.guide-sections{overflow:auto;min-height:0;padding:0 20px;overscroll-behavior:contain}.guide-section{border-top:1px solid var(--border)}h3{margin:0}.guide-section h3 button{display:flex;width:100%;align-items:center;gap:9px;padding:14px 0;text-align:start;border:0!important;box-shadow:none!important;background:transparent!important;font-size:12px;color:var(--text);line-height:1.7;min-height:51px}.guide-number{font-size:10px;color:var(--accent);flex-shrink:0}.guide-chevron{margin-inline-start:auto;color:var(--text-muted);font-size:18px;flex-shrink:0}.guide-copy{white-space:pre-line;overflow-wrap:anywhere;font-size:12px;line-height:1.95;color:var(--text-muted);padding:0 0 18px}.guide-footer{padding:11px 20px 16px;border-top:1px solid var(--border);background:var(--popup-bg);flex-shrink:0}.guide-footer p{font-size:10px;color:var(--text-muted);line-height:1.6;margin:0 0 9px;text-align:center}.guide-start{width:100%;min-height:44px;padding:10px;border-radius:10px;border:1px solid var(--accent);background:var(--accent);color:var(--accent-on,#151515);font-size:12px;font-weight:600}.guide-empty{font-size:12px;padding:18px;text-align:center}button:focus-visible,input:focus-visible,select:focus-visible{outline:var(--focus-ring);outline-offset:2px}@media(max-width:360px){.guide-header{padding-inline:15px}.guide-sections{padding-inline:15px}.guide-footer{padding-inline:15px}h2{font-size:21px!important}.guide-search{font-size:10px}}
</style>
