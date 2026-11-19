#!/usr/bin/env python3
"""Patch the generated Tauri Android project so yt-dlp, Python, QuickJS and
ffmpeg (youtubedl-android) are bundled and unpacked at startup."""
import pathlib, re, sys

root = pathlib.Path("src-tauri/gen/android/app")
VER = "0.18.1"

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
        "      androidx.core.content.ContextCompat.startForegroundService(this, android.content.Intent(this, DownloadService::class.java))\n"
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
