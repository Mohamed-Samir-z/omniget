package @PACKAGE@

import android.content.Context
import android.net.Uri
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.UUID
import org.json.JSONObject

/** Read the granted document URI through its provider, never guess a filesystem path. */
object DocumentImports {
    private const val BATCH_LIMIT = 1024 * 1024
    private const val TORRENT_LIMIT = 16 * 1024 * 1024

    private fun source(context: Context, value: String): InputStream {
        val uri = Uri.parse(value)
        return when (uri.scheme?.lowercase()) {
            "content", "file" -> context.contentResolver.openInputStream(uri)
                ?: throw IllegalArgumentException("The selected document provider is unavailable")
            null -> FileInputStream(File(value))
            else -> throw IllegalArgumentException("Select a local document, not a web address")
        }
    }

    private fun read(context: Context, value: String, limit: Int): ByteArray =
        source(context, value).use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            var total = 0
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                if (count == 0) continue
                total += count
                require(total <= limit) { "The selected document is too large" }
                output.write(buffer, 0, count)
            }
            output.toByteArray()
        }

    private fun response(action: () -> String): String = try {
        JSONObject().put("ok", true).put("value", action()).toString()
    } catch (e: Exception) {
        JSONObject().put("ok", false).put("error", e.message ?: "Could not read the selected document").toString()
    }

    @JvmStatic fun readBatch(context: Context, value: String): String = response {
        val bytes = read(context, value, BATCH_LIMIT)
        Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
    }

    /** Persist only the selected torrent metadata so queued/retried jobs outlive URI grants. */
    @JvmStatic fun copyTorrent(context: Context, value: String): String = response {
        val bytes = read(context, value, TORRENT_LIMIT)
        require(bytes.isNotEmpty()) { "The selected torrent file is empty" }
        val directory = File(context.filesDir, "torrent-imports")
        require(directory.isDirectory || directory.mkdirs()) { "Cannot prepare private import storage" }
        val temp = File(directory, "${UUID.randomUUID()}.part")
        val target = File(directory, "${UUID.randomUUID()}.torrent")
        try {
            temp.outputStream().use { output -> output.write(bytes) }
            require(temp.renameTo(target)) { "Cannot save the selected torrent metadata" }
            target.absolutePath
        } finally {
            temp.delete()
        }
    }
}
