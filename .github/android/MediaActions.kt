package @PACKAGE@

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.os.Build
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import java.util.Locale

/** Content URIs with temporary read grants, never file:// URLs. */
object MediaActions {
    internal fun intent(context: Context, path: String, share: Boolean): Intent {
        val file = File(path).canonicalFile
        require(file.isFile && file.length() > 0) { NativeText.t(context, "The downloaded file is missing or empty") }
        val uri = FileProvider.getUriForFile(context, context.packageName + ".fileprovider", file)
        val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase(Locale.ROOT)) ?: "application/octet-stream"
        return Intent(if (share) Intent.ACTION_SEND else Intent.ACTION_VIEW).apply {
            if (share) { type = mime; putExtra(Intent.EXTRA_STREAM, uri) } else setDataAndType(uri, mime)
            clipData = ClipData.newRawUri(file.name, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        }
    }
    @JvmStatic fun act(context: Context, path: String, share: Boolean) {
        val action = intent(context, path, share)
        context.startActivity(if (share) Intent.createChooser(action, NativeText.t(context, "Share downloaded file")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) else action)
    }
    @JvmStatic fun complete(context: Context, path: String, title: String) {
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(NotificationChannel("completed", NativeText.t(context, "Completed downloads"), NotificationManager.IMPORTANCE_DEFAULT))
        val id = (path.hashCode() and 0x1fffffff) + 100
        fun pending(share: Boolean): PendingIntent {
            val action = intent(context, path, share)
            val target = if (share) Intent.createChooser(action, NativeText.t(context, "Share downloaded file")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) else action
            // Separate identities for Open and Share, and for each completed file.
            return PendingIntent.getActivity(context, id * 2 + if (share) 1 else 0, target, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }
        val open = pending(false)
        val share = pending(true)
        postCompletion(context, id, title, open, share)
    }
    internal fun postCompletion(context: Context, id: Int, title: String, open: PendingIntent, share: PendingIntent) {
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, "completed") else Notification.Builder(context)
        manager.notify(id, builder.setSmallIcon(R.drawable.ic_omniget_download)
            .setContentTitle(NativeText.t(context, "Download complete")).setContentText(title.take(160))
            .setStyle(Notification.BigTextStyle().bigText(title.take(160)))
            .setContentIntent(open).setAutoCancel(true)
            .addAction(Notification.Action.Builder(null, NativeText.t(context, "Open"), open).build())
            .addAction(Notification.Action.Builder(null, NativeText.t(context, "Share"), share).build()).build())
        // Post only after the normal completion notification, never instead of it.
        // A distinct tag keeps the prayer from replacing Open/Share actions.
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(NotificationChannel("developer-prayer", NativeText.t(context, "Developer appreciation"), NotificationManager.IMPORTANCE_LOW))
        val prayer = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, "developer-prayer") else Notification.Builder(context)
        manager.notify("developer-prayer", id, prayer
            .setSmallIcon(R.drawable.ic_omniget_download)
            .setContentTitle("دعوة حلوة منك")
            .setContentText("يا ريت دعوة منك للباشمهندس محمد سمير، مطور التطبيق")
            .setStyle(Notification.BigTextStyle().bigText("يا ريت دعوة منك للباشمهندس محمد سمير، مطور التطبيق"))
            .setContentIntent(open).setAutoCancel(true).setOnlyAlertOnce(true).build())
    }
}
