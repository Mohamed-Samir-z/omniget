<script lang="ts">
 import { invoke } from "@tauri-apps/api/core";
 import { t, locale } from "$lib/i18n";
 import { audioChoice } from "$lib/multi-audio-choice";
 let {id,error}=$props<{id:number;error:string}>();
 const choice=$derived(audioChoice(error));
 let busy=$state(false);let failure=$state("");
 function names(values:string[]) {return values.map(code=>{try{return new Intl.DisplayNames($locale||"en",{type:"language"}).of(code)||code;}catch{return code;}}).join("، ") || $t('downloads.audio_none');}
 async function choose(available:boolean) {
  busy=true;failure="";
  try {await invoke("retry_download",{downloadId:id,audioLanguages:available?choice?.available:null});}
  catch(e){failure=String(e);}finally{busy=false;}
 }
</script>
{#if choice}
 <section class="audio-choice" role="alert" aria-live="polite">
  <strong>{$t('downloads.audio_choice_title')}</strong>
  <p>{$t('downloads.audio_choice_hint')}</p>
  <p>{$t('downloads.audio_unavailable')}: {names(choice.unavailable)}</p>
  <p>{$t('downloads.audio_available')}: {names(choice.available)}</p>
  {#if choice.unknown.length}<p>{$t('downloads.audio_unknown')}: {names(choice.unknown)}</p>{/if}
  {#if !choice.unavailable.length && !choice.unknown.length}<p>{$t('downloads.audio_other_failure')}</p>{/if}
  <div class="choices">
   <button class="button" disabled={busy} onclick={()=>choose(false)}>{$t('downloads.audio_retry_all')}</button>
   <button class="button" disabled={busy || !choice.available.length} onclick={()=>choose(true)}>{$t('downloads.audio_accept_available')}</button>
  </div>
  {#if failure}<p>{failure}</p>{/if}
 </section>
{/if}
<style>
.audio-choice{border:1px solid var(--accent);border-radius:12px;padding:12px;margin-top:10px;color:var(--secondary);overflow-wrap:anywhere;line-height:1.6;font-size:13px}.audio-choice p{margin:6px 0}.choices{display:flex;flex-wrap:wrap;gap:8px;margin-top:10px}.choices button{white-space:normal;min-height:42px;flex:1 1 150px}
</style>
