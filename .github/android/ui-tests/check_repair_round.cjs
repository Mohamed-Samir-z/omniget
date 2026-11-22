// Browser UI fixtures with a mocked native bridge. This does not prove Android file import or P2P transfer.
const {chromium}=require('playwright');
const {spawn}=require('child_process'); const fs=require('fs');
(async()=>{const root=process.cwd();const out=process.env.RENDER_DIR||require('path').join(require('os').tmpdir(),'omniget-repair-ui');fs.mkdirSync(out,{recursive:true});const config=require('path').join(out,'vite.preview.config.mjs');fs.writeFileSync(config,`import base from ${JSON.stringify(root+'/vite.config.js')}; export default async(env)=>{const c=typeof base==='function'?await base(env):base;c.server.fs={allow:[${JSON.stringify(root)},${JSON.stringify(require('path').dirname(require('fs').realpathSync(root+'/node_modules')))}]};return c;};`);const s=spawn('node',['node_modules/vite/bin/vite.js','--config',config,'--host','127.0.0.1','--port','5188'],{cwd:root,stdio:['ignore','pipe','pipe']});s.stdout.on('data',d=>fs.appendFileSync(out+'/server.log',d));s.stderr.on('data',d=>fs.appendFileSync(out+'/server.log',d));let b;
try{for(let i=0;i<80;i++){try{if((await fetch('http://127.0.0.1:5188')).ok)break}catch{}await new Promise(r=>setTimeout(r,500));}b=await chromium.launch({headless:true,args:['--no-sandbox']});const p=await b.newPage({viewport:{width:412,height:915},deviceScaleFactor:1,hasTouch:true,isMobile:true,userAgent:'Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 Chrome/130 Mobile Safari/537.36'});p.on('console',msg=>{if(msg.text().startsWith('SWIPE')) console.log(msg.text())});p.on('pageerror',e=>fs.appendFileSync(out+'/errors.log',e.stack+'\n'));
await p.addInitScript(()=>{localStorage.setItem('libreget.changelog.last_seen','0.16.6');const settings={schema_version:1,onboarding_completed:true,legal_acknowledged:true,appearance:{theme:localStorage.getItem('previewTheme')||'light',language:localStorage.getItem('previewLanguage')||'en'},download:{default_output_dir:'/storage/emulated/0/Download',video_quality:'best',filename_template:'%(title)s.%(ext)s',cookie_file:'',sponsorblock_categories:[],extra_ytdlp_flags:[]},advanced:{max_concurrent_downloads:3,max_retries:3,max_concurrent_segments:4,concurrent_fragments:4},telegram:{},rpc:{},proxy:{},accessibility:{}};let n=1;window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>n++,unregisterCallback:()=>{},convertFileSrc:p=>p,invoke:async(cmd,args)=>{window.__calls=window.__calls||[];window.__calls.push({cmd,args});if(cmd==='plugin:app|register_listener'){window.__backHandler=args.handler;return null;}if(cmd==='plugin:app|remove_listener'){window.__backHandler=null;return null;}if(cmd==='open_developer_profile_link'&&window.__profileFail)throw Error('Profile destination unavailable');if(cmd==='plugin:dialog|open')return null;if(cmd==='quick_media_preview'){await new Promise(r=>setTimeout(r,900));return {url:args.url,title:'Independent video preview',author:'Current source',thumbnail_url:null,duration_seconds:null};}if(cmd==='get_media_formats'){window.__formatCalls=(window.__formatCalls||0)+1;await new Promise(r=>setTimeout(r,800));if(window.__formatFail)throw Error('Network unavailable');return [{format_id:'137',ext:'mp4',resolution:'1080p',width:1920,height:1080,fps:30,filesize:null,vcodec:'avc1',acodec:'none',has_video:true,has_audio:false,audio_language:null},{format_id:'140-en',ext:'m4a',resolution:'audio only',has_video:false,has_audio:true,audio_language:'en',audio_track_name:'English',vcodec:'none',acodec:'mp4a'},{format_id:'140-ar',ext:'m4a',resolution:'audio only',has_video:false,has_audio:true,audio_language:'ar',audio_track_name:'Arabic',vcodec:'none',acodec:'mp4a'}];}if(cmd==='detect_platform')return {platform:'youtube',supported:true,content_type:'video',title:'فيديو تجريبي',url:args.url};if(cmd==='get_download_log')return ['ERROR: HTTP Error 403: Forbidden https://example.com/private?token=secret','[ffmpeg] postprocessing /storage/private/video.mp4'];if(cmd==='update_settings'){Object.assign(settings.appearance,JSON.parse(args.partial).appearance||{});return settings;}if(cmd==='get_settings')return settings;if(cmd==='android_schedules'){
window.__scheduleRows=JSON.parse(localStorage.getItem('previewRows')||'[]');
if(args.action==='list')return JSON.stringify(window.__scheduleRows);
if(args.action==='run-now'||args.action==='reschedule'){const c=JSON.parse(args.id);const row=window.__scheduleRows.find(r=>r.id===c.id);row.at=c.at||Date.now();row.state='ENQUEUED';localStorage.setItem('previewRows',JSON.stringify(window.__scheduleRows));return 'next-id';}
if(args.action==='cancel'){window.__scheduleRows.find(r=>r.id===args.id).state='CANCELLED';localStorage.setItem('previewRows',JSON.stringify(window.__scheduleRows));return '';}
return '';}
if(cmd==='plugin:event|listen')return n++;if(cmd==='plugin:app|version')return '0.16.6';if(cmd==='check_ytdlp_available')return true;if(cmd==='nami_check_python_dependencies')return {ok:false,message:'Native dependencies unavailable in browser preview',gallery_dl_version:null};if(cmd==='nami_load_app_settings')return {output_directory:null,cookies_file:null,browser_cookies:'none'};if(cmd==='fetch_marketplace_registry')throw Error('Native plugin registry unavailable in browser preview');if(cmd.includes('update'))return {available:false};if(cmd.includes('history'))return [];return []}};});
const assert=require('assert');
for(const lang of ['en','ar'])for(const theme of ['light','dark'])for(const width of [320,412]) {
 await p.setViewportSize({width,height:915});
 await p.goto('http://127.0.0.1:5188');
 await p.evaluate(({lang,theme})=>{localStorage.setItem('previewLanguage',lang);localStorage.setItem('previewTheme',theme)},{lang,theme});
 await p.reload({waitUntil:'networkidle'});
 const mascot=p.locator('.mascot.stage .mascot-img');
 const size=await mascot.boundingBox();assert.equal(size.width,128);assert.equal(size.height,128);
 const toggle=p.locator('.developer .toggle');
 const toggleStyle=await toggle.evaluate(e=>{const s=getComputedStyle(e);return {border:s.borderTopWidth,background:s.backgroundColor,width:e.getBoundingClientRect().width,height:e.getBoundingClientRect().height}});
 assert.equal(toggleStyle.border,'0px');assert.equal(toggleStyle.background,'rgba(0, 0, 0, 0)');assert(toggleStyle.width>=44&&toggleStyle.height>=44);
 assert.equal((await p.locator('.developer .portrait').boundingBox()).width,48);
 assert(await p.locator('.developer .eyebrow').isVisible());
 const side=await p.locator('.developer .card-top').evaluate(e=>{const r=e.getBoundingClientRect();const photo=e.querySelector('.portrait').getBoundingClientRect();return {gap:Math.abs(r.right-photo.right),align:getComputedStyle(e.querySelector('.identity')).textAlign}});assert(side.gap<2,'portrait must remain at padded right edge');assert.equal(side.align,'right');
 assert.deepEqual(await p.locator('.developer,.developer .identity,.developer .card-bottom').evaluateAll(ns=>ns.filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.className)),[]);
 await toggle.click();assert.equal(await toggle.getAttribute('aria-expanded'),'false');
 await toggle.click();assert.equal(await toggle.getAttribute('aria-expanded'),'true');
 assert(await p.locator('.mac-search-label').isVisible());
 assert.equal(await p.locator('input.librebox').getAttribute('placeholder'),lang==='ar'?'الصق الرابط هنا':'Paste a video link');
 await p.screenshot({path:out+`/home-${lang}-${theme}-${width}.png`});
 const trigger=p.locator('.developer .profile-trigger');
 await trigger.click();
 const profile=p.locator('dialog.developer-profile-modal');await profile.waitFor({state:'visible'});
 assert.equal(await p.locator('.developer-profile-modal .profile-links button').count(),3);
 assert((await p.locator('.profile-name').boundingBox()).height<35,'surname must stay together');
 assert.deepEqual(await p.locator('.developer-profile-modal,.profile-links,.profile-link').evaluateAll(ns=>ns.filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.className)),[]);
 await p.screenshot({path:out+`/profile-modal-${lang}-${theme}-${width}.png`});
 for(const [i,target] of ['whatsapp','portfolio','github'].entries()){await p.locator('.profile-links button').nth(i).click();assert.equal(await p.evaluate(()=>window.__calls.filter(c=>c.cmd==='open_developer_profile_link').at(-1).args.target),target)}
 await p.locator('.project-link').click();assert.equal(await p.evaluate(()=>window.__calls.filter(c=>c.cmd==='open_developer_profile_link').at(-1).args.target),'project');
 await p.locator('.profile-close').click();await profile.waitFor({state:'detached'});
 assert(await trigger.evaluate(e=>document.activeElement===e),'close must restore focus');
 await trigger.click();await profile.waitFor({state:'visible'});await p.mouse.click(2,2);await profile.waitFor({state:'detached'});
 await trigger.click();await profile.waitFor({state:'visible'});await p.keyboard.press('Escape');await profile.waitFor({state:'detached'});
 await trigger.click();await profile.waitFor({state:'visible'});await p.waitForFunction(()=>!!window.__backHandler);await p.evaluate(()=>window.__backHandler.onmessage({canGoBack:true}));await profile.waitFor({state:'detached'});await p.waitForFunction(()=>window.__backHandler===null);
 await p.locator('.developer .contact').click();assert.equal(await profile.count(),0);assert.equal(await p.evaluate(()=>window.__calls.filter(c=>c.cmd==='open_developer_contact').length),1);
 await toggle.click();assert.equal(await profile.count(),0);await trigger.click();await profile.waitFor({state:'visible'});await p.locator('.profile-close').click();await profile.waitFor({state:'detached'});assert.equal(await toggle.getAttribute('aria-expanded'),'false');await toggle.click();
 console.log('PASS profile',lang,theme,width,'X/outside/Escape/native-back callback, cleanup/focus,4 targets, separate card actions');
 const before=await p.locator('.developer').boundingBox();
 await p.locator('.librebox-plus-btn').click();
 const after=await p.locator('.developer').boundingBox();
 const menu=await p.locator('.librebox-plus-menu').boundingBox();
 assert(after.y>before.y+150,'inline menu must reserve height and push developer card');
 assert(menu.y+menu.height<=after.y+1,'developer card must not overlap inline menu');
 const hits=await p.locator('.librebox-plus-item').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return [4,r.width/2,r.width-4].map(x=>{const hit=document.elementFromPoint(r.x+x,r.y+r.height/2);return hit===n||n.contains(hit)})}));
 assert(hits.flat().every(Boolean),'whole menu rows must remain clickable');
 await p.screenshot({path:out+`/home-menu-${lang}-${theme}-${width}.png`});
 await p.locator('.librebox-plus-btn').click();
 const closed=await p.locator('.developer').boundingBox();assert(Math.abs(closed.y-before.y)<1,'collapse must restore card position');
 await p.goto('http://127.0.0.1:5188/downloads',{waitUntil:'networkidle'});
 await p.evaluate(async()=>{
  const store=await import('/src/lib/stores/download-store.svelte.ts');
  const base={url:'https://example.com/video',platform:'youtube',title:'Same video / نفس الفيديو',status:{type:'Complete',data:{success:true}},percent:100,speed_bytes_per_sec:0,downloaded_bytes:123456,total_bytes:123456,file_path:'/storage/emulated/0/Download/saved.mp4',file_size_bytes:123456,file_count:1,thumbnail_url:null,kind:'video'};
  const choice=(audio,subtitle,saved)=>({audio_requested:audio,subtitles_enabled:!!subtitle,subtitle_requested:subtitle,automatic_subtitles:!!subtitle,subtitles_obtained:saved});
  const rows=[{...base,id:1001,completed_at_ms:1000,selection:choice(['en'],null,[])},{...base,id:1002,completed_at_ms:2000,selection:choice(['ar','en'],'ar-en',['ar-en'])},{...base,id:1003,completed_at_ms:3000,selection:choice(['en'],'ar',[]),subtitle_warning:'subtitles_missing'},{...base,id:1004,completed_at_ms:500}];
  window.__fixtureRows=rows;
  store.syncQueueState(rows);
 });
 await p.locator('.download-choices').first().waitFor();
 const cards=await p.locator('.download-item').allTextContents();
 assert(cards[0].includes(lang==='ar'?'غير متاحة':'unavailable')||cards[0].includes(lang==='ar'?'لم تُحفظ':'not saved')||await p.locator('.download-item').first().locator('.subtitle-warning').count(),'newest card must have missing-caption warning');
 assert.equal(await p.locator('.download-choices .saved').count(),1);
 assert.equal(await p.locator('.download-choices .unknown').count(),1);
 const overflow=await p.locator('.download-item,.download-choices,.choice,.home-input-help').evaluateAll(nodes=>nodes.filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.className));assert.deepEqual(overflow,[]);
 await p.screenshot({path:out+`/downloads-${lang}-${theme}-${width}.png`});
 console.log('PASS',lang,theme,width,'mascot128, inline menu push/restore/hit targets, variant/warning/unknown display');
}

await p.goto('http://127.0.0.1:5188');
await p.evaluate(()=>{localStorage.setItem('previewLanguage','en');localStorage.setItem('previewTheme','light')});
for(let i=0;i<4;i++) {
 await p.reload({waitUntil:'networkidle'});
 await p.locator('.librebox-plus-btn').click();
 if(i===2){assert(await p.locator('.librebox-plus-item').nth(i).isDisabled());assert.equal(await p.locator('dialog.p2p-dialog').count(),0);console.log('PASS menu action2 Android P2P unavailable');continue;}
 await p.locator('.librebox-plus-item').nth(i).click();
 await p.waitForTimeout(150);
 if(i<2){const calls=await p.evaluate(()=>window.__calls.filter(c=>c.cmd==='plugin:dialog|open'));assert(calls.length>0);assert.deepEqual(calls.at(-1).args.options.filters[0].extensions,[i===0?'txt':'torrent']);assert(await p.locator('input.librebox').isVisible());}
 if(i===2){assert(await p.locator('dialog.p2p-dialog').isVisible());assert.equal(await p.evaluate(()=>window.__calls.filter(c=>c.cmd==='p2p_send_file').length),0);}
 if(i===3){assert(await p.locator('#adv-url').isVisible());assert(await p.locator('.home-secondary-link').isVisible());}
 console.log('PASS menu action',i,'opens its real UI without starting download/transfer');
}
await p.goto('http://127.0.0.1:5188/settings',{waitUntil:'networkidle'});
await p.locator('.developer .profile-trigger').scrollIntoViewIfNeeded();await p.locator('.developer .profile-trigger').click();
await p.locator('dialog.developer-profile-modal').waitFor({state:'visible'});
await p.evaluate(()=>window.__profileFail=true);await p.locator('.profile-links button').first().click();await p.getByText('Profile destination unavailable',{exact:false}).first().waitFor({state:'visible'});assert(await p.locator('dialog.developer-profile-modal').isVisible());await p.evaluate(()=>window.__profileFail=false);
for(let i=0;i<8;i++)await p.keyboard.press('Tab');assert(await p.locator('dialog.developer-profile-modal').evaluate(e=>e.contains(document.activeElement)),'focus must stay in modal');
await p.screenshot({path:out+'/profile-modal-settings.png'});await p.locator('.profile-close').click();await p.locator('dialog.developer-profile-modal').waitFor({state:'detached'});await p.locator('.developer .contact').click();assert.equal(await p.locator('dialog.developer-profile-modal').count(),0);
console.log('PASS Settings shared modal, failure stays visible, focus trap, independent contact');
console.log('ALL BUILD12 VISUAL/INTERACTION CHECKS PASSED');

}finally{if(b)await b.close();s.kill();}})().catch(e=>{console.error(e);process.exitCode=1});
