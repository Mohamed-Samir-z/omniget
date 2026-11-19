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

# ---- Manifest ---------------------------------------------------------
manifest = root / "src/main/AndroidManifest.xml"
m = manifest.read_text()
if "extractNativeLibs" not in m:
    m = m.replace("<application", '<application\n        android:extractNativeLibs="true"', 1)
if "MANAGE_EXTERNAL_STORAGE" not in m:
    m = m.replace("<application", '<uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />\n    <application', 1)
manifest.write_text(m)

# ---- MainActivity: unpack runtimes on a background thread -------------
acts = list(root.glob("src/main/java/**/MainActivity.kt"))
if not acts:
    sys.exit("MainActivity.kt not found")
act = acts[0]
k = act.read_text()
if "YoutubeDL" not in k:
    k = k.replace("import ", "import com.yausername.youtubedl_android.YoutubeDL\nimport com.yausername.ffmpeg.FFmpeg\nimport ", 1)
    init = (
        "\n    Thread {\n"
        "      try {\n"
        "        YoutubeDL.getInstance().init(applicationContext)\n"
        "        FFmpeg.getInstance().init(applicationContext)\n"
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
print(act.read_text())
print(gradle.read_text())
