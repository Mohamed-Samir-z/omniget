<script lang="ts">
  import { t } from "$lib/i18n";
  import { getVersion } from "@tauri-apps/api/app";
  import { open } from "@tauri-apps/plugin-shell";
  import { BUILD_INFO } from "$lib/build-info";
  import { openDeveloperContact } from "$lib/developer-contact";
  import { isDebugEnabled } from "$lib/stores/debug-store.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  let version = $state("");
  let contactBusy = $state(false);
  const copy = $derived({
    name: $t("about.android_edition.name"),
    role: $t("about.android_edition.role"),
    heading: $t("about.android_edition.heading"),
    detail: $t("about.android_edition.detail"),
    contact: $t("about.android_edition.contact"),
    original: $t("about.android_edition.original"),
    based: $t("about.android_edition.based"),
    github: $t("about.android_edition.github"),
    portrait: $t("about.android_edition.portrait"),
  });
  $effect(() => { getVersion().then(v => { version = v; }).catch(() => {}); });
  const buildDetails = $derived([BUILD_INFO.commitShort, BUILD_INFO.branch, BUILD_INFO.date].filter(p => p && p !== "unknown").join(" · "));
  const cards = $derived([
    { href: "/about/changelog", titleKey: "about.card_changelog_title", descKey: "about.card_changelog_desc", icon: "notes" },
    { href: "/about/project", titleKey: "about.card_project_title", descKey: "about.card_project_desc", icon: "project" },
    { href: "/about/terms", titleKey: "about.card_terms_title", descKey: "about.card_terms_desc", icon: "terms" },
    ...(isDebugEnabled() ? [{ href: "/about/debug", titleKey: "about.debug.title", descKey: "about.debug.description", icon: "debug" } as const] : []),
  ] as const);
  async function contact() {
    if (contactBusy) return;
    contactBusy = true;
    try { await openDeveloperContact(); } catch(e) { showToast("error", String(e)); } finally { contactBusy = false; }
  }
  async function external(e: Event, url: string) { e.preventDefault(); try { await open(url); } catch(e) { showToast("error", String(e)); } }
</script>

<div class="about-overview">
  <header class="about-hero">
    <img src="/icons/about-64.png" alt="" width="60" height="60" class="about-app-icon" />
    <div class="app-identity">
      <div class="app-name"><h1>omniget</h1><span class="edition">Android</span></div>
      <p class="app-summary">{$t("about.tagline")}</p>
      {#if version}<span class="app-version">{$t("about.version")} {version}</span>{/if}
    </div>
  </header>
  <p class="about-description">{$t("about.description")}</p>

  <section class="android-identity" aria-labelledby="android-developer-name">
    <div class="identity-top">
      <img class="developer-portrait" src="/developer/mohamed.jpg" alt={copy.portrait} width="64" height="64" />
      <div class="developer-details"><span class="section-eyebrow">{copy.role}</span><h2 id="android-developer-name">{copy.name}</h2><p>{copy.heading}</p></div>
    </div>
    <p class="developer-description">{copy.detail}</p>
    <button type="button" class="developer-contact" onclick={contact} disabled={contactBusy} aria-busy={contactBusy}>
      {copy.contact}<svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M14 3h7v7M21 3l-9 9M10 5H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-5"/></svg>
    </button>
  </section>

  <nav class="about-links" aria-label={$t("nav.about")}>
    {#each cards as card}
      <a href={card.href} class="about-link">
        <span class="link-icon" aria-hidden="true"><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          {#if card.icon === "notes"}<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M8 13h8M8 17h6"/>{:else if card.icon === "project"}<path d="M4 19a3 3 0 0 1 3-3h13M7 2h13v20H7a3 3 0 0 1-3-3V5a3 3 0 0 1 3-3z"/>{:else if card.icon === "terms"}<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10zM8 12l3 3 5-6"/>{:else}<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>{/if}
        </svg></span>
        <span class="link-copy"><strong>{$t(card.titleKey)}</strong><span>{$t(card.descKey)}</span></span>
        <span class="link-chevron" aria-hidden="true">›</span>
      </a>
    {/each}
  </nav>

  <footer class="original-project">
    <div class="original-heading"><span class="section-eyebrow">{copy.original}</span><strong>LibreGet <span>·</span> OpenSelena</strong></div>
    <p>{copy.based}</p>
    <p class="original-credit">{$t("about.credit")}</p>
    <div class="original-actions">
      <a href="https://github.com/OpenSelena/libreget" onclick={e => external(e, "https://github.com/OpenSelena/libreget")} target="_blank" rel="noopener">{copy.github}<span aria-hidden="true">↗</span></a>
      <a href="https://t.me/OpenSelena" onclick={e => external(e, "https://t.me/OpenSelena")} target="_blank" rel="noopener">Telegram<span aria-hidden="true">↗</span></a>
    </div>
    <a class="original-author" href="https://github.com/OpenSelena" onclick={e => external(e, "https://github.com/OpenSelena")}>@OpenSelena</a>
    {#if buildDetails}<span class="build-details">{buildDetails}</span>{/if}
  </footer>
</div>

<style>
  .about-overview{display:flex;flex-direction:column;gap:16px;min-width:0;width:100%;box-sizing:border-box}
  .about-hero{display:flex;gap:14px;align-items:center;padding:4px 0}
  .about-app-icon{width:60px;height:60px;object-fit:cover;border-radius:16px;flex-shrink:0;box-shadow:0 0 0 1px var(--border)}
  .app-identity{min-width:0;display:flex;flex-direction:column;gap:5px}
  .app-name{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
  .app-name h1{font-size:26px;line-height:1.1;letter-spacing:-.6px;font-weight:750;margin:0;color:var(--text)}
  .edition{font-size:10px;line-height:1.4;padding:3px 6px;border:1px solid var(--border);border-radius:6px;color:var(--text-muted)}
  .app-summary{font-size:12px;line-height:1.5;margin:0;color:var(--text-muted)}
  .app-version{font-size:11px;color:var(--text-dim)}
  .about-description{font-size:13px;line-height:1.7;margin:0;color:var(--text-muted)}
  .android-identity{border:1px solid color-mix(in srgb,var(--accent) 24%,var(--border));border-radius:17px;padding:16px;background:linear-gradient(135deg,color-mix(in srgb,var(--accent) 7%,var(--surface)),var(--surface));display:flex;flex-direction:column;gap:13px;min-width:0}
  .identity-top{display:flex;gap:13px;align-items:center;min-width:0}
  .developer-portrait{border-radius:15px;object-fit:cover;flex-shrink:0;border:2px solid var(--surface);box-shadow:0 0 0 1px var(--border)}
  .developer-details{min-width:0;display:flex;flex-direction:column;gap:4px}
  .section-eyebrow{font-size:10px;line-height:1.4;color:var(--text-muted);font-weight:600;letter-spacing:.2px}
  .developer-details h2{font-size:22px;line-height:1.35;margin:0;color:var(--text);font-weight:700}
  .developer-details p{margin:0;font-size:12px;line-height:1.5;color:var(--text-muted)}
  .developer-description{font-size:12px;line-height:1.7;color:var(--text-muted);margin:0}
  .developer-contact{min-height:44px;width:100%;display:flex;gap:9px;justify-content:center;align-items:center;border:0;border-radius:10px;background:var(--accent);color:var(--on-accent);padding:10px 14px;font:inherit;font-size:13px;font-weight:600;cursor:pointer}
  .developer-contact:disabled{opacity:.6;cursor:wait}
  .about-links{display:flex;flex-direction:column;overflow:hidden;border:1px solid var(--border);border-radius:15px;background:var(--surface)}
  .about-link{display:flex;align-items:center;gap:12px;text-decoration:none;min-height:74px;padding:13px;box-sizing:border-box;color:var(--text)}
  .about-link+.about-link{border-top:1px solid var(--border)}
  .link-icon{display:flex;justify-content:center;align-items:center;width:34px;height:38px;flex-shrink:0;border-radius:10px;background:var(--fill-1);color:var(--text-muted)}
  .link-copy{min-width:0;display:flex;flex-direction:column;gap:5px;flex:1}
  .link-copy strong{font-size:13px;line-height:1.4;font-weight:600}
  .link-copy>span{font-size:11px;line-height:1.55;color:var(--text-muted);white-space:normal;overflow-wrap:anywhere}
  .link-chevron{font-size:23px;color:var(--text-dim);flex-shrink:0}
  :global([dir="rtl"]) .link-chevron{transform:rotate(180deg)}
  .original-project{display:flex;flex-direction:column;gap:8px;padding:2px 0 12px;color:var(--text-muted)}
  .original-heading{display:flex;flex-direction:column;gap:5px}
  .original-heading strong{font-size:13px;line-height:1.5;font-weight:600;color:var(--text)}
  .original-heading strong span{color:var(--text-dim);padding:0 3px}
  .original-project p{margin:0;font-size:11px;line-height:1.6}
  .original-project .original-credit{color:var(--text-dim);font-size:10px}
  .original-actions{display:flex;gap:9px;margin-top:2px;flex-wrap:wrap}
  .original-actions a{display:flex;gap:10px;align-items:center;justify-content:space-between;min-height:40px;padding:8px 12px;box-sizing:border-box;border:1px solid var(--border);border-radius:9px;color:var(--text-muted);font-size:11px;text-decoration:none;flex:1;white-space:nowrap}
  .original-author{font-size:10px;color:var(--text-dim);text-decoration:none;width:fit-content}
  .build-details{font-size:9px;font-family:var(--font-mono);color:var(--text-dim);overflow-wrap:anywhere;margin-top:3px;direction:ltr;text-align:start}
  a:focus-visible,button:focus-visible{outline:var(--focus-ring);outline-offset:3px}
  @media(min-width:641px){.about-overview{gap:22px}.android-identity{padding:20px}.app-name h1{font-size:30px}}
  @media(max-width:360px){.android-identity{padding:13px}.developer-details h2{font-size:20px}.about-link{gap:9px;padding:11px}.original-actions a{padding:8px 9px;font-size:10px}}
</style>
