package @PACKAGE@

import android.content.Context
import android.util.Log
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.ffmpeg.FFmpeg
import java.io.File
import java.security.MessageDigest

/** Process-local readiness. A flag on disk from an earlier process is never readiness. */
object MediaRuntime {
    @Volatile var ready: Boolean = false
        private set
    @Volatile var error: String? = null
        private set
    private var starting = false
    private var prepared = false
    /** Clear previous-process flags before Tauri can start its Rust preparer. */
    @Synchronized fun prepare(context: Context) {
        if (prepared) return
        File(context.filesDir, "runtime-ready").delete()
        File(context.filesDir, "ytdl-init-error.txt").delete()
        prepared = true
    }
    private const val HASH = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3"

    @Synchronized fun start(context: Context) {
        if (starting || ready) return
        prepare(context)
        starting = true
        error = null
        val app = context.applicationContext
        Thread {
            try {
                YoutubeDL.getInstance().init(app)
                val dir = File(app.noBackupFilesDir, "youtubedl-android/yt-dlp")
                dir.mkdirs()
                val script = File(dir, "yt-dlp")
                fun digest(file: File): String {
                    val hash = MessageDigest.getInstance("SHA-256")
                    file.inputStream().use { input ->
                        val buffer = ByteArray(65536)
                        while (true) { val n = input.read(buffer); if (n < 0) break; hash.update(buffer, 0, n) }
                    }
                    return hash.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
                }
                if (!script.exists() || digest(script) != HASH) {
                    val pending = File(dir, "yt-dlp.pending")
                    app.resources.openRawResource(R.raw.omniget_ytdlp).use { input -> pending.outputStream().use { input.copyTo(it) } }
                    check(digest(pending) == HASH) { "Bundled yt-dlp checksum mismatch" }
                    check(File(dir, "app").let { !it.exists() || it.deleteRecursively() }) { "Could not clear stale yt-dlp cache" }
                    check(pending.renameTo(script)) { "Could not replace bundled yt-dlp" }
                }
                File(dir, ".bundle-sha256").writeText(HASH)
                FFmpeg.getInstance().init(app)
                File(app.filesDir, "runtime-ready").writeText("1")
                ready = true
            } catch (e: Throwable) {
                Log.e("libreget", "runtime init failed", e)
                error = NativeText.t(app, "Could not prepare the download engine. Open omniget and check the download log.")
                try { File(app.filesDir, "ytdl-init-error.txt").writeText(Log.getStackTraceString(e)) } catch (_: Throwable) {}
            } finally { synchronized(this) { starting = false } }
        }.start()
    }
}
