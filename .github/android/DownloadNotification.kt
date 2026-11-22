package @PACKAGE@

import android.app.*
import android.content.Context
import android.os.Build
import android.text.BidiFormatter
import android.widget.RemoteViews

/** App-owned centered body; Android still owns the header, icon and actions. */
object DownloadNotification {
    internal fun content(context: Context, title: String, detail: String, percent: Int, expanded: Boolean): RemoteViews {
        val view = RemoteViews(context.packageName, if (expanded) R.layout.omniget_download_expanded else R.layout.omniget_download_compact)
        val bidi = BidiFormatter.getInstance(NativeText.arabic(context))
        view.setTextViewText(R.id.omniget_download_title, bidi.unicodeWrap(title))
        view.setTextViewText(R.id.omniget_download_detail, bidi.unicodeWrap(detail))
        view.setContentDescription(R.id.omniget_download_progress, detail)
        view.setProgressBar(R.id.omniget_download_progress, 100, percent.coerceAtLeast(0), percent < 0)
        return view
    }

    internal fun build(context: Context, title: String, percent: Int, count: Int, speed: Long): Notification {
        val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
        val tap = launch?.let { PendingIntent.getActivity(context, 0, it, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE) }
        val builder = NativeNotifications.builder(context, "downloads", NativeText.t(context, "Downloads"))
        if (Build.VERSION.SDK_INT >= 31) builder.setForegroundServiceBehavior(Notification.FOREGROUND_SERVICE_IMMEDIATE)
        val status = when {
            percent < 0 -> NativeText.t(context, "Preparing download")
            percent >= 100 -> NativeText.t(context, "Finalizing and adding to media library")
            else -> NativeText.percent(context, percent)
        }
        val speedLabel = when {
            speed >= 1048576 -> String.format(java.util.Locale.US, "%.1f MB/s", speed / 1048576.0)
            speed >= 1024 -> String.format(java.util.Locale.US, "%.0f KB/s", speed / 1024.0)
            else -> "$speed B/s"
        }
        val progressDetail = if (speed > 0 && percent in 0..99) "$status · \u2066$speedLabel\u2069" else status
        val detail = if (count > 1) "${NativeText.count(context, count)} · $progressDetail" else progressDetail
        val heading = title.ifBlank { NativeText.t(context, "Downloads") }.take(160)
        return builder
            .setContentTitle(if (count > 1) NativeText.count(context, count) else "omniget")
            .setContentText("$heading · $detail")
            .setStyle(Notification.DecoratedCustomViewStyle())
            .setCustomContentView(content(context, heading, detail, percent, false))
            .setCustomBigContentView(content(context, heading, detail, percent, true))
            .setCustomHeadsUpContentView(content(context, heading, detail, percent, false))
            .setSmallIcon(R.drawable.ic_omniget_download)
            .setContentIntent(tap)
            .setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_PROGRESS)
            .setShowWhen(false)
            .setOngoing(true)
            .build()
    }
}
