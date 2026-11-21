#!/usr/bin/env python3
"""Patch the generated Tauri Android project so yt-dlp, Python, QuickJS and
ffmpeg (youtubedl-android) are bundled and unpacked at startup."""
import hashlib, io, pathlib, re, subprocess, sys, urllib.request, zipfile

root = pathlib.Path("src-tauri/gen/android/app")
# Android-only source changes live in a reviewable patch instead of changing
# desktop behavior. Fail closed if upstream changed and the patch no longer fits.
runtime_patch = pathlib.Path(__file__).resolve().with_name("runtime.patch")
subprocess.run(["git", "apply", "--check", str(runtime_patch)], check=True)
subprocess.run(["git", "apply", str(runtime_patch)], check=True)
startup_patch = pathlib.Path(__file__).resolve().with_name("startup.patch")
subprocess.run(["git", "apply", "--check", str(startup_patch)], check=True)
subprocess.run(["git", "apply", str(startup_patch)], check=True)
gallery_patch = pathlib.Path(__file__).resolve().with_name("gallery.patch")
subprocess.run(["git", "apply", "--check", str(gallery_patch)], check=True)
subprocess.run(["git", "apply", str(gallery_patch)], check=True)
media_names = pathlib.Path("src-tauri/libreget-core/src/core/media_names.rs")
media_names.parent.mkdir(parents=True, exist_ok=True)
media_names.write_text(pathlib.Path(__file__).resolve().with_name("media_names.rs").read_text())
VER = "0.18.1"
# The native runtime library bundles an older yt-dlp. Keep the Python/ffmpeg
# libraries, but package an independently pinned extractor and its EJS assets.
YTDLP_VERSION = "2026.10.26"
YTDLP_SHA256 = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3"
YTDLP_URL = f"https://github.com/yt-dlp/yt-dlp/releases/download/{YTDLP_VERSION}/yt-dlp"
with urllib.request.urlopen(YTDLP_URL, timeout=60) as response:
    ytdlp_bundle = response.read()
if hashlib.sha256(ytdlp_bundle).hexdigest() != YTDLP_SHA256:
    sys.exit("Pinned yt-dlp checksum mismatch; refusing to bundle")
with zipfile.ZipFile(io.BytesIO(ytdlp_bundle)) as archive:
    for required in ("yt_dlp/version.py", "yt_dlp_ejs/yt/solver/core.min.js",
                     "yt_dlp_ejs/yt/solver/lib.min.js"):
        if required not in archive.namelist():
            sys.exit(f"Pinned yt-dlp is missing required asset: {required}")
raw = root / "src/main/res/raw"
raw.mkdir(parents=True, exist_ok=True)
(raw / "omniget_ytdlp").write_bytes(ytdlp_bundle)
print(f"Bundled verified yt-dlp {YTDLP_VERSION} with EJS solver assets")

# ---- Gradle -----------------------------------------------------------
gradle = root / "build.gradle.kts"
g = gradle.read_text()
if "youtubedl-android" not in g:
    g = re.sub(r"(defaultConfig\s*\{)", r'\1\n        ndk { abiFilters += listOf("arm64-v8a") }', g, count=1)
    g = re.sub(r"(android\s*\{)", r'\1\n    packaging { jniLibs { useLegacyPackaging = true } }', g, count=1)
    g = re.sub(
        r"(dependencies\s*\{)",
        r'\1\n    implementation("io.github.junkfood02.youtubedl-android:library:%s")'
        r'\n    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:%s")' % (VER, VER),
        g, count=1)
    gradle.write_text(g)

# ---- R8: keep the runtime library and its reflection-heavy deps -------
pr = root / "proguard-rules.pro"
with open(pr, "a") as f:
    f.write("\n-keep class com.yausername.** { *; }\n-keep class org.apache.commons.io.** { *; }\n-keep class com.fasterxml.jackson.** { *; }\n-dontwarn org.apache.commons.io.**\n-dontwarn java.beans.**\n-dontwarn javax.xml.**\n-dontwarn org.w3c.dom.**\n-dontwarn com.fasterxml.jackson.**\n-ignorewarnings\n")

# R8 breaks commons-compress' static initialisers (ExceptionInInitializerError
# while unpacking Python), so ship the release build unobfuscated.
g = gradle.read_text()
g = re.sub(r"optimization\s*\{\s*enable\s*=\s*true", "optimization { enable = false", g)
g = g.replace("isMinifyEnabled = true", "isMinifyEnabled = false")
gradle.write_text(g)
with open(pr, "a") as f:
    f.write("-keep class org.apache.commons.compress.** { *; }\n-dontwarn org.apache.commons.compress.**\n")

# ---- Manifest ---------------------------------------------------------
manifest = root / "src/main/AndroidManifest.xml"
m = manifest.read_text()
if "extractNativeLibs" not in m:
    m = m.replace("<application", '<application\n        android:extractNativeLibs="true"', 1)
if "MANAGE_EXTERNAL_STORAGE" not in m:
    m = m.replace("<application", '<uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />\n    <application', 1)
PERMS = ["FOREGROUND_SERVICE", "FOREGROUND_SERVICE_DATA_SYNC", "POST_NOTIFICATIONS",
         "WAKE_LOCK", "REQUEST_IGNORE_BATTERY_OPTIMIZATIONS", "REQUEST_INSTALL_PACKAGES", "ACCESS_NETWORK_STATE"]
for perm in PERMS:
    if "android.permission." + perm + '"' not in m:
        m = m.replace("<application", '<uses-permission android:name="android.permission.%s" />\n    <application' % perm, 1)
if "DownloadService" not in m:
    m = m.replace("</application>", '<service android:name=".DownloadService" android:exported="false" android:foregroundServiceType="dataSync" />\n    </application>', 1)
manifest.write_text(m)

# ---- Share target: omniget appears in Android's share sheet --------------
m = manifest.read_text()
if "android.intent.action.SEND" not in m:
    act_tag = re.search(r'<activity[^>]*android:name="\.MainActivity"[^>]*>', m)
    if not act_tag:
        sys.exit("MainActivity activity tag not found in AndroidManifest.xml")
    share_filter = (
        "\n            <intent-filter>\n"
        '                <action android:name="android.intent.action.SEND" />\n'
        '                <category android:name="android.intent.category.DEFAULT" />\n'
        '                <data android:mimeType="text/plain" />\n'
        "            </intent-filter>"
    )
    m = m.replace(act_tag.group(0), act_tag.group(0) + share_filter, 1)
    manifest.write_text(m)

# ---- MainActivity: unpack runtimes on a background thread -------------
acts = list(root.glob("src/main/java/**/MainActivity.kt"))
if not acts:
    sys.exit("MainActivity.kt not found")
act = acts[0]
k = act.read_text()
if "YoutubeDL" not in k:
    k = k.replace("import ", "import com.yausername.youtubedl_android.YoutubeDL\nimport com.yausername.ffmpeg.FFmpeg\nimport ", 1)
    svc = (
        "\n    try {\n"
        "      if (intent?.action != android.content.Intent.ACTION_SEND && android.os.Build.VERSION.SDK_INT >= 33) requestPermissions(arrayOf(\"android.permission.POST_NOTIFICATIONS\"), 1)\n"
        "      val prefs = getSharedPreferences(\"libreget\", 0)\n"
        "      val pm = getSystemService(android.content.Context.POWER_SERVICE) as android.os.PowerManager\n"
        "      if (!prefs.getBoolean(\"battery_asked\", false) && !pm.isIgnoringBatteryOptimizations(packageName)) {\n"
        "        prefs.edit().putBoolean(\"battery_asked\", true).apply()\n"
        "        startActivity(android.content.Intent(android.provider.Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, android.net.Uri.parse(\"package:\" + packageName)))\n"
        "      }\n"
        "    } catch (e: Throwable) { android.util.Log.e(\"libreget\", \"service start failed\", e) }\n"
    )
    init = svc + (
        "\n    Thread {\n"
        "      try {\n"
        "        YoutubeDL.getInstance().init(applicationContext)\n"
        "        val runtimeDir = java.io.File(applicationContext.noBackupFilesDir, \"youtubedl-android/yt-dlp\")\n"
        "        val script = java.io.File(runtimeDir, \"yt-dlp\")\n"
        f"        val expectedHash = \"{YTDLP_SHA256}\"\n"
        "        fun digest(file: java.io.File): String {\n"
        "          val hash = java.security.MessageDigest.getInstance(\"SHA-256\")\n"
        "          file.inputStream().use { input -> val buffer = ByteArray(65536); while (true) { val n = input.read(buffer); if (n < 0) break; hash.update(buffer, 0, n) } }\n"
        "          return hash.digest().joinToString(\"\") { \"%02x\".format(it.toInt() and 255) }\n"
        "        }\n"
        "        if (!script.exists() || digest(script) != expectedHash) {\n"
        "          val pending = java.io.File(runtimeDir, \"yt-dlp.pending\")\n"
        "          applicationContext.resources.openRawResource(R.raw.omniget_ytdlp).use { input -> pending.outputStream().use { output -> input.copyTo(output) } }\n"
        "          check(digest(pending) == expectedHash) { \"Bundled yt-dlp checksum mismatch\" }\n"
        "          check(java.io.File(runtimeDir, \"app\").let { !it.exists() || it.deleteRecursively() }) { \"Could not clear stale yt-dlp cache\" }\n"
        "          check(pending.renameTo(script)) { \"Could not replace bundled yt-dlp\" }\n"
        "        }\n"
        "        java.io.File(runtimeDir, \".bundle-sha256\").writeText(expectedHash)\n"
        "        FFmpeg.getInstance().init(applicationContext)\n        java.io.File(applicationContext.filesDir, \"runtime-ready\").writeText(\"1\")\n"
        "      } catch (e: Throwable) {\n"
        "        android.util.Log.e(\"libreget\", \"runtime init failed\", e)\n"
        "        try { java.io.File(applicationContext.filesDir, \"ytdl-init-error.txt\").writeText(android.util.Log.getStackTraceString(e)) } catch (_: Throwable) {}\n"
        "      }\n"
        "    }.start()\n"
    )
    if "onCreate" in k:
        k = re.sub(r"(super\.onCreate\([^)]*\)[^\n]*\n)", lambda mm: mm.group(1) + init, k, count=1)
    else:
        if "import android.os.Bundle" not in k:
            k = k.replace("import ", "import android.os.Bundle\nimport ", 1)
        body = "{\n  override fun onCreate(savedInstanceState: Bundle?) {\n    super.onCreate(savedInstanceState)" + init + "  }\n}"
        if re.search(r"TauriActivity\(\)\s*\{", k):
            k = re.sub(r"(TauriActivity\(\)\s*)\{", r"\1" + body.replace("\\", "\\\\")[:-1], k, count=1)
        else:
            k = re.sub(r"(TauriActivity\(\))", r"\1 " + body.replace("\\", "\\\\"), k, count=1)
    act.write_text(k)
    pkg = re.search(r"^package\s+([\w.]+)", k, re.M).group(1)
    media_template = pathlib.Path(__file__).resolve().with_name("MediaLibrary.kt").read_text()
    (act.parent / "MediaLibrary.kt").write_text(media_template.replace("@PACKAGE@", pkg))
    service_template = pathlib.Path(__file__).resolve().with_name("DownloadService.kt").read_text()
    (act.parent / "DownloadService.kt").write_text(service_template.replace("@PACKAGE@", pkg))
    drawable = root / "src/main/res/drawable"
    drawable.mkdir(parents=True, exist_ok=True)
    (drawable / "ic_omniget_download.xml").write_text(
        pathlib.Path(__file__).resolve().with_name("ic_omniget_download.xml").read_text())
    # Preserve the existing duck/penguin artwork chosen by the owner.
    import shutil
    icon = pathlib.Path("src-tauri/icons/icon.png")
    for density in ("mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"):
        target = root / f"src/main/res/mipmap-{density}"
        target.mkdir(parents=True, exist_ok=True)
        for name in ("ic_launcher", "ic_launcher_round"):
            for old in target.glob(f"{name}.*"):
                old.unlink()
            shutil.copyfile(icon, target / f"{name}.png")
    # Do not leave generated adaptive placeholders overriding the chosen icon.
    for name in ("ic_launcher.xml", "ic_launcher_round.xml"):
        for old in (root / "src/main/res").glob(f"mipmap-anydpi*/{name}"):
            old.unlink()
    m = manifest.read_text()
    for attr, value in (("label", "omniget"), ("icon", "@mipmap/ic_launcher"), ("roundIcon", "@mipmap/ic_launcher_round")):
        if f"android:{attr}=" in m:
            m = re.sub(rf'android:{attr}="[^"]*"', f'android:{attr}="{value}"', m)
        else:
            m = m.replace("<application", f'<application android:{attr}="{value}"', 1)
    manifest.write_text(m)
    # Android UI branding only; package ID and signing/update identity stay put.
    for locale in pathlib.Path("src/lib/i18n").glob("*.json"):
        text = locale.read_text()
        text = re.sub(r'("hero_title"\s*:\s*)"LibreGet"', r'\1"omniget"', text)
        locale.write_text(text)
# ---- Native popup: keep the opaque app UI hidden above the source app -----
# Use the existing single Tauri runtime. A second TauriActivity would create
# competing runtimes on warm shares, while a separate cold native activity
# would have no initialized download queue.
k = act.read_text()
pkg = re.search(r"^package\s+([\w.]+)", k, re.M).group(1)
if pkg != "com.openselena.libreget":
    sys.exit("Native share JNI package mismatch")
if "nativeShareAction" not in k:
    share_code = r'''
  private var sharePopup: SharePopup? = null
  private var shareWebView: android.webkit.WebView? = null
  private var shareContentHidden = false

  fun setShareContentVisible(visible: Boolean) {
    // Keep queued downloads alive, but do not leave a blank share-only task
    // in Recents. Opening the full app restores its normal task entry.
    val manager = getSystemService(android.content.Context.ACTIVITY_SERVICE) as android.app.ActivityManager
    manager.appTasks.firstOrNull { it.taskInfo.taskId == taskId }?.setExcludeFromRecents(!visible)
    shareContentHidden = !visible
    findViewById<android.view.View>(android.R.id.content)?.alpha = if (visible) 1f else 0f
    shareWebView?.alpha = if (visible) 1f else 0f
  }

  override fun onWebViewCreate(webView: android.webkit.WebView) {
    super.onWebViewCreate(webView)
    shareWebView = webView
    if (shareContentHidden) setShareContentVisible(false)
  }

  override fun onNewIntent(intent: android.content.Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    handleShareIntent(intent)
  }

  override fun onDestroy() {
    sharePopup?.dispose()
    super.onDestroy()
  }

  external fun nativeShareReady(): Boolean
  external fun nativeShareAction(text: String, mode: String, quality: String, action: String): String

  private fun handleShareIntent(intent: android.content.Intent?) {
    if (intent?.action != android.content.Intent.ACTION_SEND) {
      sharePopup?.openFullApp()
      setShareContentVisible(true)
      return
    }
    val shared = intent.getStringExtra(android.content.Intent.EXTRA_TEXT) ?: return
    if (sharePopup == null) sharePopup = SharePopup(this)
    sharePopup!!.show(shared)
  }
'''
    k = re.sub(r"(super\.onCreate\([^)]*\)[^\n]*\n)", r"\1    handleShareIntent(intent)\n", k, count=1)
    idx = k.rstrip().rfind("}")
    k = k.rstrip()[:idx] + share_code + "}\n"
    # Don't pull the user into Android battery settings while sharing.
    k = k.replace('if (!prefs.getBoolean("battery_asked", false)',
                  'if (intent?.action != android.content.Intent.ACTION_SEND && !prefs.getBoolean("battery_asked", false)')
    act.write_text(k)
(act.parent / "SharePopup.kt").write_text(pathlib.Path(__file__).with_name("SharePopup.kt").read_text().replace("@PACKAGE@", pkg))

# A translucent native window exposes the source task while the web content
# is hidden. Full-app entry remains opaque because its web content is visible.
m = manifest.read_text()
act_tag = re.search(r'<activity[^>]*android:name="\.MainActivity"[^>]*>', m)
if not act_tag:
    sys.exit("MainActivity missing for share host theme")
old_tag = act_tag.group(0)
parent = re.search(r'android:theme="([^"]+)"', old_tag)
if parent:
    parent = parent.group(1)
else:
    app_tag = re.search(r'<application[^>]*>', m).group(0)
    match = re.search(r'android:theme="([^"]+)"', app_tag)
    parent = match.group(1) if match else "@style/Theme.AppCompat.DayNight.NoActionBar"
new_tag = re.sub(r'\s+android:theme="[^"]+"', '', old_tag)
new_tag = new_tag[:-1] + ' android:theme="@style/OmnigetShareHost">'
m = m.replace(old_tag, new_tag, 1)
manifest.write_text(m)
values = root / "src/main/res/values"
values.mkdir(parents=True, exist_ok=True)
(values / "omniget_share_host.xml").write_text(
    '<resources><style name="OmnigetShareHost" parent="' + parent + '">'
    '<item name="android:windowIsTranslucent">true</item>'
    '<item name="android:windowBackground">@android:color/transparent</item>'
    '<item name="android:windowDisablePreview">true</item>'
    '<item name="android:windowNoTitle">true</item>'
    '</style></resources>')

# ---- In-app updates: FileProvider so the installer can read the APK ------
# install_android_update (src-tauri/src/android_update.rs) downloads the new
# APK into the app cache and opens it with ACTION_VIEW; the provider grants
# the system installer one-shot read access to that private file.
m = manifest.read_text()
if "FILE_PROVIDER_PATHS" not in m:
    provider = (
        '<provider android:name="androidx.core.content.FileProvider" '
        'android:authorities="com.openselena.libreget.fileprovider" '
        'android:exported="false" android:grantUriPermissions="true">'
        '<meta-data android:name="android.support.FILE_PROVIDER_PATHS" '
        'android:resource="@xml/file_paths" />'
        "</provider>"
    )
    m = m.replace("</application>", "        " + provider + "\n    </application>", 1)
    manifest.write_text(m)
xml_dir = root / "src/main/res/xml"
xml_dir.mkdir(parents=True, exist_ok=True)
(xml_dir / "file_paths.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n'
    "<paths>\n"
    '    <cache-path name="updates" path="." />\n'
    "</paths>\n"
)

# ---- In-app updates: ApkInstaller Kotlin helper called over JNI -----------
installer_target = act.parent / "ApkInstaller.kt"
if not installer_target.exists():
    pkg_match = re.search(r"^package\s+([\w.]+)", act.read_text(), re.M)
    if not pkg_match or pkg_match.group(1) != "com.openselena.libreget":
        sys.exit(
            "MainActivity package is not com.openselena.libreget; "
            "the ApkInstaller JNI bridge in android_update.rs would break"
        )
    installer_template = pathlib.Path(__file__).resolve().with_name("ApkInstaller.kt").read_text()
    installer_target.write_text(installer_template.replace("@PACKAGE@", pkg_match.group(1)))

# FileProvider lives in androidx.core, which the generated project does not
# depend on directly.
g = gradle.read_text()
if "androidx.core:core" not in g:
    g = re.sub(
        r"(dependencies\s*\{)",
        r'\1\n    implementation("androidx.core:core:1.13.1")',
        g,
        count=1,
    )
    gradle.write_text(g)

print(act.read_text())
print(gradle.read_text())
