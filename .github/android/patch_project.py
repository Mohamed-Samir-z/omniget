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
         "WAKE_LOCK", "REQUEST_IGNORE_BATTERY_OPTIMIZATIONS"]
for perm in PERMS:
    if "android.permission." + perm + '"' not in m:
        m = m.replace("<application", '<uses-permission android:name="android.permission.%s" />\n    <application' % perm, 1)
if "DownloadService" not in m:
    m = m.replace("</application>", '<service android:name=".DownloadService" android:exported="false" android:foregroundServiceType="dataSync" />\n    </application>', 1)
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
        "      if (android.os.Build.VERSION.SDK_INT >= 33) requestPermissions(arrayOf(\"android.permission.POST_NOTIFICATIONS\"), 1)\n"
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
    (act.parent / "DownloadService.kt").write_text(f"""package {pkg}

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager

/** Keeps the process alive while the app is in the background so downloads continue. */
class DownloadService : Service() {{
    private var wake: PowerManager.WakeLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {{
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= 26) {{
            nm.createNotificationChannel(NotificationChannel("downloads", "Downloads", NotificationManager.IMPORTANCE_LOW))
        }}
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(this, "downloads") else Notification.Builder(this)
        val n = builder
            .setContentTitle("OmniGet")
            .setContentText("Downloads keep running in the background")
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setOngoing(true)
            .build()
        try {{
            if (Build.VERSION.SDK_INT >= 29) startForeground(1, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
            else startForeground(1, n)
        }} catch (e: Throwable) {{ android.util.Log.e("libreget", "startForeground failed", e) }}
        if (wake == null) {{
            val pm = getSystemService(POWER_SERVICE) as PowerManager
            wake = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "libreget:downloads").also {{ it.acquire() }}
        }}
        return START_STICKY
    }}

    override fun onDestroy() {{
        try {{ wake?.release() }} catch (_: Throwable) {{}}
        super.onDestroy()
    }}
}}
""")
print(act.read_text())
print(gradle.read_text())
