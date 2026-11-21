<script lang="ts">
 import { invoke } from "@tauri-apps/api/core";
 import { t, locale } from "$lib/i18n";
 import { audioChoice } from "$lib/multi-audio-choice";
 let {id,error}=$props<{id:number;error:string}>();
 const choice=$derived(audioChoice(error));
 let busy=$state(false);let failure=$state("");
 function name(code:string) {try{return new Intl.DisplayNames($locale||"en",{type:"language"}).of(code)||code;}catch{return code;}}
 async function choose(available:boolean) {
  busy=true;failure="";
  try {await invoke("retry_download",{downloadId:id,audioLanguages:available?choice?.available:null});}
  catch{failure=$t("common.error") as string;}finally{busy=false;}
 }
</script>
{#if choice}
 <section class="audio-choice" role="alert" aria-live="polite">
  <strong>{$t('downloads.audio_choice_title')}</strong>
  <p>{$t('downloads.audio_choice_hint')}</p>
  <div class="language-groups">
   <div class="language-group available">
    <h3>{$t('downloads.audio_available')}</h3>
    <div class="badges">{#each choice.available as code}<span class="badge" lang={code}>{name(code)}</span>{:else}<span class="empty">{$t('downloads.audio_none')}</span>{/each}</div>
   </div>
   {#if choice.unavailable.length}<div class="language-group unavailable">
    <h3>{$t('downloads.audio_unavailable')}</h3>
    <div class="badges">{#each choice.unavailable as code}<span class="badge" lang={code}>{name(code)}</span>{/each}</div>
   </div>{/if}
   {#if choice.unknown.length}<div class="language-group unknown">
    <h3>{$t('downloads.audio_unknown')}</h3>
    <div class="badges">{#each choice.unknown as code}<span class="badge" lang={code}>{name(code)}</span>{/each}</div>
   </div>{/if}
  </div>
  <div class="choices">
   <button class="button" disabled={busy} onclick={()=>choose(false)}>{$t('downloads.audio_retry_all')}</button>
   <button class="button" disabled={busy || !choice.available.length} onclick={()=>choose(true)}>{$t('downloads.audio_accept_available')}</button>
  </div>
  {#if failure}<p>{failure}</p>{/if}
 </section>
{/if}
<style>
.audio-choice{border:1px solid var(--accent);border-radius:12px;padding:12px;margin-top:10px;color:var(--secondary);overflow-wrap:anywhere;line-height:1.6;font-size:13px}.audio-choice p{margin:6px 0}.choices{display:flex;flex-wrap:wrap;gap:8px;margin-top:10px}.choices button{white-space:normal;min-height:42px;flex:1 1 150px}
.language-groups{display:grid;gap:8px;margin-top:12px}.language-group{padding:10px;border-radius:10px;border:1px solid;min-width:0}.language-group h3{margin:0 0 7px;font-size:12px;font-weight:600}.badges{display:flex;flex-wrap:wrap;gap:6px}.badge{color:inherit!important;display:inline-block;padding:3px 10px;border-radius:999px;max-width:100%;font-size:13px;background:rgba(255,255,255,.08)}.available{color:#b8f7d1;background:#102e22;border-color:#367754}.unavailable{color:#ffd0d0;background:#351c20;border-color:#925158}.unknown{color:#e2e5eb;background:#272b34;border-color:#656c7a}.empty{opacity:.85;font-size:12px}
</style>
