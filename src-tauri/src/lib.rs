use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
#[cfg(desktop)]
use tauri_plugin_deep_link::DeepLinkExt;

use tokio_util::sync::CancellationToken;

pub struct P2pSendHandle {
    pub cancel_token: CancellationToken,
    pub paused: Arc<std::sync::atomic::AtomicBool>,
}
pub type ActiveP2pSends = Arc<tokio::sync::Mutex<HashMap<String, P2pSendHandle>>>;

pub mod commands;
pub mod cookies;
pub mod core;
pub mod extension_storage;
pub mod external_url;
#[cfg(desktop)]
pub mod hotkey;
#[cfg(not(desktop))]
#[path = "hotkey_stub.rs"]
pub mod hotkey;
pub mod local_bridge;
pub mod mcp;
pub mod models;
pub mod platforms;
pub mod plugin_host;
pub mod plugin_loader;
pub mod storage;
#[cfg(desktop)]
pub mod tray;
#[cfg(not(desktop))]
#[path = "tray_stub.rs"]
pub mod tray;
#[cfg(target_os = "android")]
pub mod android_env;

struct DesktopCookieProvider;

impl libreget_core::platforms::cookie_provider::CookieProvider for DesktopCookieProvider {
    fn cookie_path_for(&self, domain: &str) -> Option<std::path::PathBuf> {
        let root = crate::cookies::root_domain_of(domain);
        if root.is_empty() {
            return None;
        }

        let slug = libreget_core::core::log_hook::current_cookie_slug();
        let path = crate::cookies::account_path_for_consumer(&root, slug.as_deref())
            .or_else(|| crate::cookies::account_path_for_consumer(&root, None))?;
        crate::cookies::touch_last_used(&root, slug.as_deref().unwrap_or("_default"));
        Some(path)
    }

    fn cookie_path_for_account(&self, domain: &str, slug: &str) -> Option<std::path::PathBuf> {
        let root = crate::cookies::root_domain_of(domain);
        if root.is_empty() {
            return None;
        }
        let path = crate::cookies::account_path_for_consumer(&root, Some(slug))?;
        crate::cookies::touch_last_used(&root, slug);
        Some(path)
    }

    fn manual_cookie_header(&self, domain: &str) -> Option<String> {
        let root = crate::cookies::root_domain_of(domain);
        if root != "x.com" && root != "twitter.com" {
            return None;
        }

        let raw = storage::config::load_settings_standalone()
            .advanced
            .twitter_manual_cookie;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed = core::cookie_parser::parse_cookie_input(trimmed, "");
        if !parsed.cookie_string.trim().is_empty() {
            Some(parsed.cookie_string)
        } else {
            Some(trimmed.to_string())
        }
    }
}

struct DesktopBilibiliRuntimeProvider;

impl libreget_core::platforms::bilibili::BilibiliRuntimeProvider
    for DesktopBilibiliRuntimeProvider
{
    fn active_account_slug(&self) -> Option<String> {
        let registry = crate::cookies::load_registry();
        let bucket = registry.buckets.get("bilibili.com")?;

        if let Some(selected) = libreget_core::core::log_hook::current_cookie_slug() {
            if bucket
                .accounts
                .iter()
                .any(|a| a.slug == selected && a.slug != "_anonymous" && a.cookie_count > 0)
            {
                return Some(selected);
            }
        }

        let native = bucket
            .accounts
            .iter()
            .filter(|a| a.slug != "_anonymous" && a.slug != "_default" && a.cookie_count > 0)
            .max_by_key(|a| a.last_used_at_ms.unwrap_or(a.captured_at_ms))
            .map(|a| a.slug.clone());
        if native.is_some() {
            return native;
        }

        bucket
            .accounts
            .iter()
            .filter(|a| a.slug == "_default" && a.cookie_count > 0)
            .find(|a| desktop_bilibili_slug_has_session_cookie(&a.slug))
            .map(|a| a.slug.clone())
    }

    fn settings(&self) -> libreget_core::models::settings::AppSettings {
        storage::config::load_settings_standalone()
    }

    fn persist_account(
        &self,
        cookies: &[libreget_core::platforms::bilibili::BilibiliAuthCookie],
        uname: &str,
        source_label: &str,
    ) -> Result<String, String> {
        let slug = libreget_core::platforms::bilibili::auth::slug_from_uname(uname);
        let entries: Vec<crate::extension_storage::ExtensionCookie> = cookies
            .iter()
            .map(|cookie| crate::extension_storage::ExtensionCookie {
                domain: cookie.domain.clone(),
                http_only: cookie.http_only,
                path: cookie.path.clone(),
                secure: cookie.secure,
                expires: cookie.expires,
                name: cookie.name.clone(),
                value: cookie.value.clone(),
                host_only: cookie.host_only,
                same_site: cookie.same_site.clone(),
            })
            .collect();

        crate::cookies::storage::write_account_file("bilibili.com", &slug, &entries)
            .map_err(|e| e.to_string())?;

        let mut registry = crate::cookies::storage::load_registry();
        let now_ms = crate::cookies::storage::current_unix_ms();
        let bucket = registry
            .buckets
            .entry("bilibili.com".to_string())
            .or_insert_with(|| crate::cookies::storage::BucketEntry {
                platform_kind: "bilibili".to_string(),
                enabled: true,
                accounts: Vec::new(),
            });
        bucket.platform_kind = "bilibili".to_string();

        let alias = format!("{} · {}", uname, source_label);
        if let Some(existing) = bucket.accounts.iter_mut().find(|a| a.slug == slug) {
            existing.captured_at_ms = now_ms;
            existing.cookie_count = entries.len();
            existing.source_label = Some(source_label.to_string());
            existing.alias = alias;
        } else {
            bucket.accounts.push(crate::cookies::storage::AccountEntry {
                slug: slug.clone(),
                alias,
                source_url: Some("https://www.bilibili.com".to_string()),
                source_label: Some(source_label.to_string()),
                captured_at_ms: now_ms,
                cookie_count: entries.len(),
                last_used_at_ms: Some(now_ms),
            });
        }

        crate::cookies::storage::save_registry(&registry).map_err(|e| e.to_string())?;
        Ok(slug)
    }
}

fn desktop_bilibili_slug_has_session_cookie(slug: &str) -> bool {
    let path = match crate::cookies::account_path_for_consumer("bilibili.com", Some(slug)) {
        Some(p) => p,
        None => return false,
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return false;
        }
        line.split('\t').nth(5) == Some("SESSDATA")
    })
}

pub struct AppState {
    pub active_downloads: Arc<tokio::sync::Mutex<HashMap<u64, CancellationToken>>>,
    pub active_generic_downloads:
        Arc<tokio::sync::Mutex<HashMap<u64, (String, CancellationToken)>>>,
    pub registry: core::registry::PlatformRegistry,
    pub download_queue: Arc<tokio::sync::Mutex<core::queue::DownloadQueue>>,
    pub torrent_session: Arc<tokio::sync::Mutex<Option<Arc<librqbit::Session>>>>,
    pub active_p2p_sends: ActiveP2pSends,
    pub frontend_ready: Arc<tokio::sync::Mutex<bool>>,
    pub pending_external_events: Arc<tokio::sync::Mutex<Vec<external_url::ExternalUrlEvent>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Android has no HOME/XDG variables and a read-only cwd; every `dirs::*`
    // lookup below depends on this having run first.
    #[cfg(target_os = "android")]
    android_env::init();

    tracing_subscriber::fmt::init();

    // Emitido antes do single-instance poder sair em silencio: ver
    // core::portable::startup_banner.
    tracing::info!(
        "{}",
        core::portable::startup_banner(
            env!("CARGO_PKG_VERSION"),
            std::process::id(),
            std::env::var("LIBREGET_PORTABLE").ok().as_deref() == Some("1"),
            core::paths::app_data_dir().as_deref(),
        )
    );

    let mut registry = core::registry::PlatformRegistry::new();
    // gallery-dl first: it claims bulk-listing URLs (profiles, subreddits)
    // that the single-post Twitter/Reddit downloaders would otherwise match
    registry.register(Arc::new(platforms::gallerydl::GalleryDlDownloader::new()));
    registry.register(Arc::new(
        libreget_core::platforms::InstagramDownloader::new(),
    ));
    registry.register(Arc::new(libreget_core::platforms::ThreadsDownloader::new()));
    registry.register(Arc::new(
        libreget_core::platforms::PinterestDownloader::new(),
    ));
    registry.register(Arc::new(libreget_core::platforms::TikTokDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::TwitterDownloader::new()));
    registry.register(Arc::new(
        libreget_core::platforms::TwitchClipsDownloader::new(),
    ));
    registry.register(Arc::new(libreget_core::platforms::BlueskyDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::RedditDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::YouTubeDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::VimeoDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::BilibiliDownloader::new()));
    registry.register(Arc::new(libreget_core::platforms::DouyinDownloader::new()));
    let torrent_session: Arc<tokio::sync::Mutex<Option<Arc<librqbit::Session>>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    registry.register(Arc::new(platforms::magnet::MagnetDownloader::new(
        torrent_session.clone(),
    )));
    registry.register(Arc::new(libreget_core::platforms::P2pDownloader::new()));
    registry.register(Arc::new(
        libreget_core::platforms::DirectFileDownloader::new(),
    ));
    registry.register(Arc::new(
        platforms::generic_ytdlp::GenericYtdlpDownloader::new(),
    ));

    let state = AppState {
        active_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        active_generic_downloads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        registry,
        download_queue: Arc::new(tokio::sync::Mutex::new(core::queue::DownloadQueue::new(2))),
        torrent_session,
        active_p2p_sends: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        frontend_ready: Arc::new(tokio::sync::Mutex::new(false)),
        pending_external_events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let builder = tauri::Builder::default();

    // Single-instance is desktop-only; it must stay the first plugin registered.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(url) =
                external_url::find_external_url_arg(argv.iter().skip(1).map(|arg| arg.as_str()))
            {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        external_url::handle_external_url(&app_handle, url, "command-line").await
                    {
                        tracing::warn!(
                            "Failed to handle external URL from second instance: {}",
                            error
                        );
                    }
                });
            } else {
                tray::show_window(app);
            }
    }));

    let builder = builder
        .manage(state)
        .manage(Arc::new(tokio::sync::RwLock::new(
            plugin_loader::PluginManager::new(
                core::paths::app_data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("plugins"),
            ),
        )))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init());

    // Plugins that only exist on desktop (updater, global shortcuts, autostart,
    // deep-link registration, process restart).
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        hotkey::on_hotkey_pressed(app, shortcut);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));

    builder
        .setup(|app| {
            // A janela principal e criada aqui, e nao pelo `tauri.conf.json`
            // (`"create": false`), porque so daqui da para passar
            // `.data_directory(...)` ao WebView2. O Tauri resolve esse caminho
            // para `LocalData/{identifier}` e cria o diretorio *antes* do setup
            // rodar, entao a variavel de ambiente que o `main.rs` define nunca
            // chega a ser lida — que e a causa da issue #195, o modo portatil
            // deixando uma pasta fora do diretorio do app.
            {
                // `mut` so e usado no ramo Windows abaixo; sem o cfg_attr isto
                // vira warning novo em macOS e Linux e reprova o portao de clippy.
                #[cfg_attr(not(any(windows, target_os = "linux")), allow(unused_mut))]
                let mut builder = tauri::WebviewWindowBuilder::from_config(
                    app.handle(),
                    &app.config().app.windows[0],
                )?;

                // Nao e so Windows. No Linux o wry usa este caminho para
                // `base_data_directory`, `base_cache_directory` e os cookies do
                // WebKitGTK; sem ele o modo portatil deixa
                // `XDG_DATA_HOME/com.openselena.libreget` no perfil do usuario — a
                // mesma #209, fora do Windows. Foi o smoke test do B55 que
                // pegou isso, na primeira vez que rodou.
                //
                // macOS fica de fora porque o wry nao le `data_directory` no
                // WKWebView: incluir daria a impressao de cobrir um caso que
                // nao esta coberto.
                #[cfg(any(windows, target_os = "linux"))]
                if let Some(webview_dir) = core::portable::portable_webview_dir_from_env() {
                    if let Err(e) = std::fs::create_dir_all(&webview_dir) {
                        tracing::warn!(
                            "[portable] nao foi possivel criar {}: {} — o WebView2 vai usar o diretorio padrao",
                            webview_dir.display(),
                            e
                        );
                    } else {
                        tracing::info!(
                            "[portable] WebView2 apontado para {}",
                            webview_dir.display()
                        );
                        builder = builder.data_directory(webview_dir);
                    }
                }

                // macOS nao tem para onde apontar: o wry nao le `data_directory`
                // no WKWebView (nenhuma referencia em `src/wkwebview/`), ao
                // contrario do webkitgtk, que a usa para base_data_directory,
                // base_cache_directory e cookies.
                //
                // Entao o modo portatil nao cumpre o que promete aqui, e dizer
                // isso e melhor do que deixar o usuario achar que o pendrive nao
                // deixou rastro. Issue #227.
                #[cfg(target_os = "macos")]
                if core::portable::portable_webview_dir_from_env().is_some() {
                    tracing::warn!(
                        "[portable] no macOS o WebView guarda dados em ~/Library mesmo em modo \
                         portatil — o wry nao permite redirecionar. Ver github.com/OpenSelena/libreget/issues/227"
                    );
                }

                builder.build()?;

                // A unica prova de que a janela subiu. O CI compila em todas as
                // plataformas mas nunca abriu o app: foi assim que a #209 passou
                // batido. O smoke test do CI casa exatamente esta linha.
                tracing::info!("[startup] main window created");
            }

            // Modo smoke: sobe, prova que a janela existe, e sai sozinho com 0.
            // So existe para o CI conseguir responder "abre?", que e a pergunta
            // que compilar nunca responde.
            if let Ok(raw) = std::env::var("LIBREGET_SMOKE_EXIT_MS") {
                if let Ok(ms) = raw.trim().parse::<u64>() {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(ms));
                        tracing::info!("[startup] smoke mode: exiting cleanly");
                        handle.exit(0);
                    });
                }
            }

            commands::host_queue::register_event_listeners(app.handle());
            {
                let handle = app.handle().clone();
                platforms::bilibili::notify::set_emitter(Box::new(
                    move |event: &str, payload: serde_json::Value| {
                        use tauri::Emitter;
                        let _ = handle.emit(event, payload);
                    },
                ));
            }
            {
                let handle = app.handle().clone();
                libreget_core::platforms::bilibili::notify::set_emitter(Box::new(
                    move |event: &str, payload: serde_json::Value| {
                        use tauri::Emitter;
                        let _ = handle.emit(event, payload);
                    },
                ));
            }
            let settings = storage::config::load_settings(app.handle());
            core::http_client::init_proxy(settings.proxy.clone());
            core::http_fetcher::set_global_max_concurrent_segments(
                settings.advanced.max_concurrent_segments as usize,
            );
            libreget_core::platforms::cookie_provider::set_cookie_provider(Arc::new(
                DesktopCookieProvider,
            ));
            libreget_core::platforms::bilibili::set_runtime_provider(Arc::new(
                DesktopBilibiliRuntimeProvider,
            ));
            core::ytdlp::set_per_domain_cookie_fn(|url| {
                let parsed = url::Url::parse(url).ok()?;
                let host = parsed.host_str()?;
                let root = crate::cookies::root_domain_of(host);
                if root.is_empty() {
                    return None;
                }
                let slug = libreget_core::core::log_hook::current_cookie_slug();
                let path = crate::cookies::account_path_for_consumer(&root, slug.as_deref())?;
                crate::cookies::touch_last_used(&root, slug.as_deref().unwrap_or("_default"));
                Some(path)
            });
            core::ytdlp::set_managed_cookies_only_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .always_use_managed_cookies
            });
            core::ytdlp::set_global_cookie_file_fn(|| {
                let s = storage::config::load_settings_standalone();
                let cf = s.download.cookie_file.clone();
                if !cf.is_empty() && std::path::Path::new(&cf).exists() {
                    Some(cf)
                } else {
                    None
                }
            });
            core::ytdlp::set_cookies_from_browser_fn(|| {
                storage::config::load_settings_standalone()
                    .advanced
                    .cookies_from_browser
            });
            core::ytdlp::set_manual_cookie_header_fn(|| {
                storage::config::load_settings_standalone()
                    .advanced
                    .twitter_manual_cookie
            });
            core::ytdlp::set_ext_referer_fn(|url| {
                extension_storage::read_extension_metadata(url).and_then(|m| m.referer)
            });
            core::ytdlp::set_include_auto_subs_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .include_auto_subtitles
            });
            core::ytdlp::set_caption_locale_fn(|| {
                let s = storage::config::load_settings_standalone();
                let locale = s.download.caption_locale.trim().to_string();
                if locale.is_empty() || locale.eq_ignore_ascii_case("auto") {
                    let lang = s.appearance.language.trim().to_string();
                    if lang.is_empty() {
                        "en".to_string()
                    } else {
                        lang
                    }
                } else {
                    locale
                }
            });
            core::ytdlp::set_keep_vtt_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .keep_vtt
            });
            core::ytdlp::set_translate_metadata_fn(|| {
                let s = storage::config::load_settings_standalone();
                if s.download.translate_metadata {
                    let lang = s.appearance.language.trim();
                    if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    }
                } else {
                    None
                }
            });
            core::ytdlp::set_sponsorblock_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .youtube_sponsorblock
            });
            core::ytdlp::set_sponsorblock_mode_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .sponsorblock_mode
            });
            core::ytdlp::set_sponsorblock_categories_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .sponsorblock_categories
            });
            core::ytdlp::set_split_chapters_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .split_by_chapters
            });
            core::ytdlp::set_embed_metadata_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .embed_metadata
            });
            core::ytdlp::set_embed_thumbnail_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .embed_thumbnail
            });
            core::ytdlp::set_speed_limit_fn(|| {
                let v = storage::config::load_settings_standalone()
                    .download
                    .speed_limit;
                let t = v.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            });
            core::ytdlp::set_live_from_start_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .live_from_start
            });
            core::ytdlp::set_insecure_tls_fn(|| {
                storage::config::load_settings_standalone()
                    .advanced
                    .insecure_tls
            });
            core::ytdlp::set_concurrent_fragments_fn(|| {
                storage::config::load_settings_standalone()
                    .advanced
                    .concurrent_fragments
            });
            core::ytdlp::set_user_agent_fn(|| {
                let v = storage::config::load_settings_standalone()
                    .advanced
                    .user_agent;
                let t = v.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            });
            core::ytdlp::set_extra_ytdlp_flags_fn(|| {
                storage::config::load_settings_standalone()
                    .download
                    .extra_ytdlp_flags
            });
            {
                let app_handle = app.handle().clone();
                libreget_core::core::log_hook::set_log_sink(std::sync::Arc::new(move |id, line| {
                    // B33: caixa-preta. Toda linha de log de download passa por
                    // aqui, e o `record` redige antes de guardar — e o unico
                    // ponto onde da para capturar o historico sem instrumentar
                    // cada chamada uma por uma.
                    core::flight_recorder::record(line);
                    let should_emit = core::download_log::push_line(id, line);
                    if should_emit {
                        // A linha vai junto para o card mostrar "a última
                        // coisa que o yt-dlp disse" sem pedir o log inteiro
                        // a cada evento.
                        let _ = tauri::Emitter::emit(
                            &app_handle,
                            "download-log-update",
                            serde_json::json!({ "id": id, "line": line }),
                        );
                    }
                }));
            }
            core::recovery::init_from_disk();
            core::queue_history::init_from_disk();
            core::channels::init_from_disk();
            core::channel_poller::start(app.handle().clone());
            core::queue::start_scheduler(app.handle().clone());
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let snapshot = {
                        let mut q = state.download_queue.lock().await;
                        q.hydrate_from_history();
                        q.get_state()
                    };
                    if !snapshot.is_empty() {
                        core::queue::emit_queue_state_from_state(&app_handle, snapshot);
                    }
                });
            }
            {
                let pending = core::recovery::list();
                if !pending.is_empty() {
                    let app_handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        let _ = tauri::Emitter::emit(
                            &app_handle,
                            "recovery-available",
                            serde_json::json!({ "count": pending.len() }),
                        );
                    });
                }
            }
            #[cfg(desktop)]
            {
                let app_handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let raw = url.to_string();
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) =
                                external_url::handle_external_url(&handle, raw, "deep-link").await
                            {
                                tracing::warn!("Failed to handle deep-link URL: {}", error);
                            }
                        });
                    }
                });
            }
            // Make sure the OS routes `libreget://` to this binary even when the
            // package didn't ship a desktop file with `MimeType=x-scheme-handler/libreget;`
            // or when the user is running from an AppImage / unpacked build.
            // No-op on macOS (which uses Info.plist registered by the bundler).
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                if let Err(error) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register libreget:// scheme: {}", error);
                }
            }
            tray::setup(app.handle())?;
            hotkey::register_from_settings(app.handle());

            // Migration: drop the manifests / binary copies the previous
            // native-messaging code left under `~/.config/...` so Chrome and
            // Firefox stop trying to start the legacy host process.
            extension_storage::cleanup_legacy_native_messaging();

            // Cookie Manager: reparticiona `chrome-extension-cookies.txt` legacy em
            // `cookies/<domain>/_default.txt` (CK-1). Quando o registry já tem
            // buckets, a migration vira no-op. Após migration bem-sucedida, o
            // arquivo legacy é deletado — a partir de CK-7b o multi-file é a
            // única fonte de verdade.
            if !cookies::storage::has_been_migrated() {
                match cookies::migrate_legacy_if_needed() {
                    Ok(_) => {
                        let legacy = extension_storage::extension_cookie_file_path();
                        if legacy.exists() {
                            if let Err(err) = std::fs::remove_file(&legacy) {
                                tracing::warn!(
                                    "cookies: failed to remove legacy file {}: {}",
                                    legacy.display(),
                                    err
                                );
                            }
                        }
                    }
                    Err(err) => tracing::warn!("cookies: legacy migration skipped: {err}"),
                }
            } else {
                // Registry já populado — significa migration anterior já rolou; se
                // o legacy ainda existe (instalação intermediária), remove agora.
                let legacy = extension_storage::extension_cookie_file_path();
                if legacy.exists() {
                    if let Err(err) = std::fs::remove_file(&legacy) {
                        tracing::warn!(
                            "cookies: failed to remove stale legacy file {}: {}",
                            legacy.display(),
                            err
                        );
                    }
                }
            }

            // Start the localhost HTTP bridge for the browser extension. This
            // replaces the previous Chrome native-messaging path, which forced
            // us to maintain a hard-coded extension-ID allowlist. Bridge auth
            // is via a per-installation token shown in the Settings UI.
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    local_bridge::spawn(app_handle).await;
                });
            }
            {
                let plugins_dir = core::paths::app_data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("plugins");
                let host: std::sync::Arc<dyn libreget_plugin_sdk::PluginHost> = std::sync::Arc::new(
                    plugin_host::PluginHostImpl::new(app.handle().clone(), plugins_dir),
                );
                let plugin_mgr = app
                    .handle()
                    .state::<std::sync::Arc<tokio::sync::RwLock<plugin_loader::PluginManager>>>();
                let mgr_for_plugins = std::sync::Arc::clone(&*plugin_mgr);
                let app_emit = app.handle().clone();
                std::thread::Builder::new()
                    .name("plugins-bootstrap".into())
                    .spawn(move || {
                        use tauri::Emitter;

                        // Load already-installed plugins first so they are
                        // usable immediately (and offline), without waiting
                        // on any of the network calls below.
                        {
                            let mut mgr = mgr_for_plugins.blocking_write();
                            let _ = mgr.register_builtin(
                                libreget_plugin_convert::manifest(),
                                Box::new(libreget_plugin_convert::ConvertPlugin::new()),
                                std::sync::Arc::clone(&host),
                            );
                            let _ = mgr.register_builtin(
                                libreget_plugin_telegram::manifest(),
                                Box::new(libreget_plugin_telegram::TelegramPlugin::new()),
                                std::sync::Arc::clone(&host),
                            );
                            let _ = mgr.register_builtin(
                                libreget_plugin_courses::manifest(),
                                Box::new(
                                    libreget_plugin_courses::CoursesPlugin::new()
                                        .with_app(app_emit.clone()),
                                ),
                                std::sync::Arc::clone(&host),
                            );
                            mgr.load_all(std::sync::Arc::clone(&host));
                        }
                        let _ = app_emit.emit("plugins-changed", ());

                        let rt = match tokio::runtime::Runtime::new() {
                            Ok(rt) => rt,
                            Err(e) => {
                                tracing::warn!("plugins-bootstrap runtime failed: {}", e);
                                return;
                            }
                        };
                        rt.block_on(commands::plugins::ensure_default_plugins(
                            std::sync::Arc::clone(&mgr_for_plugins),
                        ));
                        rt.block_on(commands::plugins::auto_update_plugins(
                            std::sync::Arc::clone(&mgr_for_plugins),
                        ));

                        // Load anything newly installed above. load_all is
                        // not idempotent (re-inserting drops the previously
                        // loaded plugin and its dylib), so only load entries
                        // that are not loaded yet.
                        {
                            let mut mgr = mgr_for_plugins.blocking_write();
                            let to_load: Vec<String> = mgr
                                .installed_plugins()
                                .iter()
                                .filter(|p| p.enabled && !mgr.is_loaded(&p.id))
                                .map(|p| p.id.clone())
                                .collect();
                            for id in &to_load {
                                let _ = mgr.load_one(id, std::sync::Arc::clone(&host));
                            }
                        }
                        let _ = app_emit.emit("plugins-changed", ());
                    })
                    .ok();
            }

            std::thread::Builder::new()
                .name("startup-checks".into())
                .spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("startup runtime");
                    rt.block_on(async {
                        // Garante o yt-dlp (zipapp quando há Python, senão o
                        // onefile) antes do primeiro download pedir por ele.
                        if let Err(e) = core::ytdlp::ensure_ytdlp().await {
                            tracing::warn!("yt-dlp not available at startup: {}", e);
                        }
                        core::dependencies::ensure_js_runtime().await;
                        let _ = tokio::task::spawn_blocking(
                            core::ytdlp::cleanup_stale_pyinstaller_dirs,
                        )
                        .await;
                        // O `--update-to` é um bootstrap inteiro do yt-dlp
                        // mais rede, e trocar o binário com download ativo
                        // quebra o processo (B2). Espera o app assentar e só
                        // roda sem yt-dlp em andamento; se a fila está cheia,
                        // tenta de novo a cada 5 min por até uma hora.
                        tokio::time::sleep(std::time::Duration::from_secs(45)).await;
                        for _ in 0..12 {
                            let Some(ytdlp) = core::ytdlp::find_ytdlp_cached().await else {
                                break;
                            };
                            match core::ytdlp::check_ytdlp_update_detailed(&ytdlp).await {
                                Ok(core::ytdlp::UpdateCheck::Updated) => {
                                    tracing::info!("yt-dlp updated successfully");
                                    break;
                                }
                                Ok(core::ytdlp::UpdateCheck::UpToDate)
                                | Ok(core::ytdlp::UpdateCheck::AlreadyChecked) => {
                                    tracing::debug!("yt-dlp already up to date");
                                    break;
                                }
                                Ok(core::ytdlp::UpdateCheck::Busy) => {
                                    tracing::debug!(
                                        "yt-dlp update check deferred: downloads running"
                                    );
                                    tokio::time::sleep(std::time::Duration::from_secs(300))
                                        .await;
                                }
                                Err(e) => {
                                    tracing::warn!("yt-dlp update check failed: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                })
                .ok();

            if let Some(url) = external_url::find_external_url_arg(std::env::args().skip(1)) {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        external_url::handle_external_url(&app_handle, url, "command-line").await
                    {
                        tracing::warn!("Failed to handle startup external URL: {}", error);
                    }
                });
            }

            if settings.start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            local_bridge::media_stream_url,
            commands::auth_webview::open_auth_webview,
            commands::bilibili_auth::bilibili_qr_generate,
            commands::bilibili_auth::bilibili_qr_poll,
            commands::bilibili_auth::bilibili_captcha_challenge,
            commands::bilibili_auth::bilibili_sms_send,
            commands::bilibili_auth::bilibili_sms_verify,
            commands::bilibili_auth::bilibili_account_status,
            commands::bilibili_auth::bilibili_import_watch_later,
            commands::bilibili_auth::bilibili_import_history,
            commands::bilibili_auth::bilibili_preview_info,
            commands::bilibili_auth::bilibili_webview_login,
            commands::browser_extension::browser_extension_status,
            commands::browser_extension::browser_extension_export,
            commands::browser_extension::browser_extension_open_folder,
            cookies::commands::cookies_list,
            cookies::commands::cookies_read,
            cookies::commands::cookies_import,
            cookies::commands::cookies_clear,
            cookies::commands::cookies_clear_batch,
            cookies::commands::cookies_rename,
            cookies::commands::cookies_accounts_for_url,
            cookies::commands::cookies_read_as_json,
            cookies::commands::cookies_import_file,
            cookies::commands::cookies_export_to,
            cookies::commands::cookies_add_account,
            cookies::commands::cookies_set_bucket_enabled,
            cookies::commands::cookies_health,
            cookies::commands::cookies_test,
            commands::clip::clip_video,
            commands::reencode::reencode_video,
            commands::diagnostics::get_hwaccel_info,
            commands::diagnostics::diagnose_download_error,
            commands::downloads::detect_platform,
            commands::downloads::check_cookie_error,
            commands::downloads::validate_output_path,
            #[cfg(not(target_os = "android"))]
            commands::downloads::get_media_formats,
            #[cfg(not(target_os = "android"))]
            commands::downloads::prefetch_media_info,
            #[cfg(not(target_os = "android"))]
            commands::downloads::download_from_url,
            #[cfg(not(target_os = "android"))]
            commands::downloads::playlist_entries,
            #[cfg(not(target_os = "android"))]
            commands::downloads::torrent_contents,
            commands::channels::channels_list,
            commands::channels::channel_add,
            commands::channels::channel_remove,
            commands::channels::channel_update,
            commands::channels::channel_check_now,
            commands::channels::sync_channels_tray,
            commands::tray::sync_tray_labels,
            commands::ai::ai_get_config,
            commands::ai::ai_set_config,
            commands::ai::ai_test,
            #[cfg(not(target_os = "android"))]
            commands::ai::ai_summarize_url,
            #[cfg(not(target_os = "android"))]
            commands::ai::whisper_generate,
            commands::ai::ai_history_list,
            commands::ai::ai_history_clear,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::video_op_silence_estimate,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::video_op_preset,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::video_op_propose,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::video_op_run,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::detect_shot_changes,
            #[cfg(not(target_os = "android"))]
            commands::video_ops::waveform_peaks,
            #[cfg(not(target_os = "android"))]
            commands::subtitle_ws::subtitle_load,
            #[cfg(not(target_os = "android"))]
            commands::subtitle_ws::subtitle_save,
            #[cfg(not(target_os = "android"))]
            commands::subtitle_ws::subtitle_translate,
            #[cfg(not(target_os = "android"))]
            commands::subtitle_ws::subtitle_grammar_fix,
            #[cfg(not(target_os = "android"))]
            commands::downloads::metadata_fetch,
            #[cfg(not(target_os = "android"))]
            commands::downloads::thumbnails_list,
            #[cfg(not(target_os = "android"))]
            commands::downloads::thumbnail_save,
            #[cfg(not(target_os = "android"))]
            commands::downloads::subtitles_list,
            #[cfg(not(target_os = "android"))]
            commands::downloads::subtitles_save,
            #[cfg(not(target_os = "android"))]
            commands::downloads::subtitles_merge,
            #[cfg(not(target_os = "android"))]
            commands::downloads::comments_fetch,
            #[cfg(not(target_os = "android"))]
            commands::downloads::chapters_fetch,
            #[cfg(not(target_os = "android"))]
            commands::downloads::tools_save_text,
            #[cfg(not(target_os = "android"))]
            commands::downloads::livechat_fetch,
            #[cfg(not(target_os = "android"))]
            commands::downloads::download_with_custom_args,
            commands::downloads::cancel_generic_download,
            commands::yt_templates::yt_templates_list,
            commands::yt_templates::yt_templates_save,
            commands::yt_templates::yt_templates_delete,
            commands::downloads::pause_download,
            commands::downloads::resume_download,
            commands::downloads::pause_all_downloads,
            commands::downloads::resume_all_downloads,
            commands::downloads::reorder_queue,
            commands::downloads::retry_download,
            commands::downloads::remove_download,
            commands::downloads::update_max_concurrent,
            commands::downloads::clear_finished_downloads,
            commands::downloads::get_download_log,
            commands::downloads::get_download_command,
            commands::downloads::retry_download_with_command,
            commands::downloads::parse_batch_file,
            commands::downloads::get_recovery_items,
            commands::downloads::discard_recovery,
            #[cfg(not(target_os = "android"))]
            commands::downloads::restore_recovery,
            commands::downloads::get_download_history,
            commands::downloads::clear_download_history,
            #[cfg(not(target_os = "android"))]
            commands::downloads::reveal_file,
            #[cfg(not(target_os = "android"))]
            commands::downloads::open_path_default,
            commands::host_queue::host_queue_enqueue_external,
            commands::host_queue::host_queue_report_progress,
            commands::host_queue::host_queue_report_complete,
            commands::integration::register_external_frontend,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::settings::mark_onboarding_complete,
            commands::settings::mark_legal_acknowledged,
            commands::rpc::rpc_test_connection,
            commands::rpc::rpc_set_source,
            commands::rpc::rpc_clear_source,
            commands::rpc::rpc_set_idle_stats,
            commands::settings::get_bridge_info,
            commands::settings::rotate_bridge_token,
            commands::settings::bridge_open_pairing,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::check_dependencies,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_status,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_install,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_action,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_set_theme,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_remove_addon,
            #[cfg(not(target_os = "android"))]
            commands::spicetify::spicetify_install_marketplace,
            #[cfg(not(target_os = "android"))]
            commands::tools::text::tool_humanize,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_ollama_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_ollama_recommended,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_ollama_pull,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_ollama_delete,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_pricing_info,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_pricing_search,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_pricing_for,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_usage_report,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_usage_clear,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_slideshare,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_gdocs_parse,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_gdocs_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_calameo,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_gallery_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_gallery_install,
            #[cfg(not(target_os = "android"))]
            commands::tools::documents::tool_gallery_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::downloads::tool_aria2_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::downloads::tool_aria2_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::downloads::tool_manifest_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_dupes_scan,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_shred,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_dupes_delete,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_rename_plan,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_rename_apply,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_file_search_backend,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_file_search,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_awake_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::files::tool_awake_get,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_upscale_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_upscale_install,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_upscale_run,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_resize,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_ocr_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_ocr_run,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_exif_read,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_exif_strip,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_icon_pack,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_img_dupes,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_img_compress,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_video_compress,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_video_restore,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_video_gif,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_video_silence,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_subtitle_convert,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_subtitle_burn,
            #[cfg(not(target_os = "android"))]
            commands::tools::audio::tool_audio_clean,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_hash,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_hmac,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_hash_id,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_magic,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_cipher,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_caesar_brute,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_xor,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_encode,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_detect,
            #[cfg(not(target_os = "android"))]
            commands::tools::ctf::tool_ctf_freq,
            #[cfg(not(target_os = "android"))]
            commands::tools::phone::tool_kde_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::phone::tool_kde_share,
            #[cfg(not(target_os = "android"))]
            commands::tools::phone::tool_kde_ping,
            #[cfg(not(target_os = "android"))]
            commands::tools::phone::tool_kde_refresh,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_whisper_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_whisper_install,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_whisper_model_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_whisper_model_remove,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_whisper_transcribe,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_tts_voices,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_tts_speak,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_srt_translate,
            #[cfg(not(target_os = "android"))]
            commands::tools::speech::tool_dub,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_kinds,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_save,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_delete,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_test,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_balance,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_models,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_keys_use,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_mcp_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_mcp_set_enabled,
            #[cfg(not(target_os = "android"))]
            commands::tools::ai::tool_mcp_selftest,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_hotkeys_get,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_hotkey_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_autoclick_start,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_autoclick_stop,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_autoclick_state,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_autoclick_mouse,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_devices,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_options,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_set_options,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_state,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_start,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_dictation_stop,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_record_sources,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_record_state,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_record_start,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_record_stop,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_record_save_replay,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_vs_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_vs_launch,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_vs_clone,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_vs_design,
            #[cfg(not(target_os = "android"))]
            commands::tools::desktop::tool_vs_isolate,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_info,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_merge,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_split,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_render,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_text,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_from_images,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_compress,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_sanitize,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_ocr,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_office,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_repair,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_redaction_check,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_password,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_watermark,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_crop,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_outline_read,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_outline_write,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_win_tweaks_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_win_tweak_apply,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_clean_scan,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_clean_run,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_disk_volumes,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_disk_scan,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_disk_trash,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_startup_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_startup_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_uninstall_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_uninstall_leftovers,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_uninstall_run,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_debloat_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_debloat_remove,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_debloat_restore,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_registry_scan,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_registry_fix,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_registry_backups_dir,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_updater_status,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_updater_upgrade,
            #[cfg(not(target_os = "android"))]
            commands::tools::youtube::tool_sponsorblock,
            #[cfg(not(target_os = "android"))]
            commands::tools::youtube::tool_ryd,
            #[cfg(not(target_os = "android"))]
            commands::tools::youtube::tool_yt_video_id,
            #[cfg(not(target_os = "android"))]
            commands::tools::youtube::tool_save_url,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_session,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_query_ids_refresh,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_cancel,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_post,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_thread,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_export_posts,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_export_users,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_render_posts,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_profile,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_profile_lookup,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_media,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_media_posts,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_search,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_trends,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_bookmarks_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_follows_audit,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_unfollow,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_whitelist_get,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_whitelist_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_archive_open,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_archive_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_grok_config,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_grok_config_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_grok_ask,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_data_url,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_save_data_url,
            #[cfg(not(target_os = "android"))]
            commands::tools::x::tool_x_write_text,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_inspect,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_related,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_boards_search,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_download_many,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_backup,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_dupes,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_unsave,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_palette,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_keywords,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_source,
            #[cfg(not(target_os = "android"))]
            commands::tools::pinterest::tool_pin_expand,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_accounts,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_whoami,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_parse,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_cancel,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_post,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_resolve,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_download_bulk,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_profile,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_friendship,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_profile_media,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_stories,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_stories_tray,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_highlights,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_highlight_items,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_story_viewers,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_follow_lists,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_whitelist_get,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_whitelist_set,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_actions_today,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_actions,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_resolve_users,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_snapshot_take,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_snapshots,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_snapshot_diff,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_snapshot_delete,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_ghosts,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_write_csv,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_read_text,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_analytics,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_hashtag,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_comments,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_likers,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_giveaway,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_publish,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_publish_graph,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_schedule_list,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_schedule_add,
            #[cfg(not(target_os = "android"))]
            commands::tools::instagram::tool_ig_schedule_remove,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::check_ytdlp_available,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::install_dependency,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::dependency_archived_versions,
            commands::diagnostics::flight_recorder_dump,
            commands::diagnostics::flight_recorder_clear,
            commands::diagnostics::preflight_batch,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::rollback_dependency,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::clear_dependency_path,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::dependency_custom_path,
            commands::rules::list_rules,
            commands::rules::save_rules,
            commands::rules::preview_rule_match,
            commands::media_history::check_media_changed,
            commands::media_history::record_media_snapshot,
            commands::dedupe::deduplicate_files,
            commands::dedupe::content_store_stats,
            commands::smart_speed::compute_silence_map,
            commands::smart_speed::silence_skip_target,
            commands::smart_speed::forget_silence_map,
            commands::torrent_playback::torrent_playback_readiness,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::dependency_variants,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::dependency_install_dir,
            #[cfg(not(target_os = "android"))]
            commands::dependencies::set_dependency_path,
            #[cfg(not(target_os = "android"))]
            commands::search::search_videos,
            commands::plugins::list_plugins,
            commands::plugins::get_plugin_frontend_path,
            commands::plugins::set_plugin_enabled,
            commands::plugins::uninstall_plugin,
            commands::plugins::get_loaded_plugin_manifests,
            commands::plugins::plugin_command,
            commands::plugins::fetch_marketplace_registry,
            commands::plugins::install_plugin_from_registry,
            commands::plugins::get_plugin_i18n,
            commands::plugins::check_plugin_updates,
            commands::plugins::update_plugin,
            #[cfg(not(target_os = "android"))]
            commands::p2p::p2p_send_file,
            #[cfg(not(target_os = "android"))]
            commands::p2p::p2p_cancel_send,
            #[cfg(not(target_os = "android"))]
            commands::p2p::p2p_pause_send,
            #[cfg(not(target_os = "android"))]
            commands::p2p::p2p_resume_send,
            commands::app_lifecycle::force_exit_app,
            commands::app_lifecycle::get_debug_info,
            commands::app_lifecycle::get_portable_info,
            #[cfg(not(target_os = "android"))]
            commands::tools::twitch::tool_tw_emotes,
            #[cfg(not(target_os = "android"))]
            commands::tools::twitch::tool_tw_chat_replay,
            #[cfg(not(target_os = "android"))]
            commands::tools::music::tool_music_playlist,
            #[cfg(not(target_os = "android"))]
            commands::tools::music::tool_lyrics_sync,
            #[cfg(not(target_os = "android"))]
            commands::tools::music::tool_music_history,
            #[cfg(not(target_os = "android"))]
            commands::tools::reddit::tool_rd_download,
            #[cfg(not(target_os = "android"))]
            commands::tools::reddit::tool_rd_thread,
            #[cfg(not(target_os = "android"))]
            commands::tools::reddit::tool_rd_gdpr,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_img_stitch,
            #[cfg(not(target_os = "android"))]
            commands::tools::images::tool_img_sprite,
            #[cfg(not(target_os = "android"))]
            commands::tools::bilibili::tool_bili_danmaku_export,
            #[cfg(not(target_os = "android"))]
            commands::tools::bilibili::tool_bili_danmaku_burn,
            #[cfg(not(target_os = "android"))]
            commands::tools::games::tool_switch_album,
            #[cfg(not(target_os = "android"))]
            commands::tools::games::tool_clip_organizer,
            #[cfg(not(target_os = "android"))]
            commands::tools::games::tool_protondb,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_hosts_read,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_hosts_apply,
            #[cfg(not(target_os = "android"))]
            commands::tools::system::tool_hosts_restore,
            #[cfg(not(target_os = "android"))]
            commands::tools::video::tool_sticker,
            #[cfg(not(target_os = "android"))]
            commands::tools::pdf::tool_pdf_markdown,
            #[cfg(not(target_os = "android"))]
            commands::tools::linkedin::tool_li_overview,
            #[cfg(not(target_os = "android"))]
            commands::tools::linkedin::tool_li_connections,
            #[cfg(not(target_os = "android"))]
            commands::tools::linkedin::tool_li_messages,
            #[cfg(not(target_os = "android"))]
            commands::tools::linkedin::tool_li_checklist,
            commands::nami::nami_check_python_dependencies,
            commands::nami::http::nami_http_fetch,
            commands::nami::nami_run_gallery_dl_download,
            commands::nami::nami_cancel_download,
            commands::nami::nami_save_app_settings,
            commands::nami::nami_load_app_settings,
            commands::nami::nami_load_profiles,
            commands::nami::nami_save_profile,
            commands::nami::nami_delete_profile,
            commands::nami::nami_get_history,
            commands::nami::nami_reconcile_disk,
            commands::nami::nami_delete_history_record,
            commands::nami::nami_pull_creator,
            commands::nami::nami_pull_all_creators,
            commands::nami::nami_detect_browsers,
            commands::nami::nami_extract_browser_cookies,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = &event {
                let state = app_handle.state::<AppState>();
                let session_mutex = state.torrent_session.clone();
                tauri::async_runtime::block_on(async move {
                    let session_guard = session_mutex.lock().await;
                    let session = session_guard.as_ref().cloned();
                    drop(session_guard);
                    if let Some(session) = session {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            session.stop(),
                        )
                        .await
                        {
                            Ok(()) => tracing::info!("torrent session stopped cleanly"),
                            Err(_) => tracing::warn!(
                                "torrent session stop timed out after 5s; exiting anyway"
                            ),
                        }
                    }
                });
            }
        });
}
