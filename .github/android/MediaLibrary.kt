package @PACKAGE@

import android.content.Context
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Looper
import android.webkit.MimeTypeMap
import java.io.File
import java.io.IOException
import java.util.Locale
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/** Register only finalized files. Called from a Rust blocking worker, never UI. */
object MediaLibrary {
    @JvmStatic
    fun register(context: Context, path: String): String {
        check(Looper.myLooper() != Looper.getMainLooper()) { "Media registration must not block UI" }
        val file = File(path).canonicalFile
        require(file.isFile && file.length() > 0) { "Media file is missing or empty" }
        val mime = MimeTypeMap.getSingleton()
            .getMimeTypeFromExtension(file.extension.lowercase(Locale.ROOT))
        repeat(2) {
            val finished = CountDownLatch(1)
            val result = AtomicReference<Uri?>(null)
            MediaScannerConnection.scanFile(
                context.applicationContext, arrayOf(file.path),
                mime?.let { arrayOf(it) },
                MediaScannerConnection.OnScanCompletedListener { _, uri ->
                    result.set(uri)
                    finished.countDown()
                }
            )
            if (finished.await(12, TimeUnit.SECONDS)) {
                result.get()?.let { uri ->
                    // The scanner has committed this row to MediaStore. Notify
                    // observers only now, not while ffmpeg is replacing the file.
                    try {
                        context.contentResolver.notifyChange(uri, null)
                    } catch (e: Exception) {
                        android.util.Log.w("libreget", "Media observer notification failed", e)
                    }
                    return uri.toString()
                }
            }
        }
        throw IOException("Android media scan did not register the file after two attempts")
    }
}
