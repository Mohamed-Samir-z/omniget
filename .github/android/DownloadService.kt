package @PACKAGE@

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.os.SystemClock

/** One foreground notification for the active download queue. */
class DownloadService : Service() {
    private var wake: PowerManager.WakeLock? = null
    companion object {
        private data class Progress(val title: String, val percent: Int, val count: Int, val speed: Long)
        @Volatile private var progress = Progress("", -1, 1, 0)
        @Volatile private var instance: DownloadService? = null
        private var lastUpdate = 0L
        @JvmStatic @Synchronized fun update(title: String, percent: Int, count: Int, speed: Long) {
            val next = Progress(title.take(160), percent.coerceIn(-1, 100), count.coerceAtLeast(1), speed.coerceAtLeast(0))
            val now = SystemClock.elapsedRealtime()
            if (next == progress && now - lastUpdate < 1000) return
            progress = next
            if (now - lastUpdate >= 500) {
                lastUpdate = now
                instance?.refresh(false)
            }
        }
    }
    override fun onBind(intent: Intent?): IBinder? = null
    private fun refresh(foreground: Boolean) {
        val p = progress
        val manager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= 26) {
            manager.createNotificationChannel(NotificationChannel("downloads", NativeText.t(this, "Downloads"), NotificationManager.IMPORTANCE_LOW))
        }
        val launch = packageManager.getLaunchIntentForPackage(packageName)
        val tap = launch?.let { PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE) }
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(this, "downloads") else Notification.Builder(this)
        val status = when {
            p.percent < 0 -> NativeText.t(this, "Preparing download")
            p.percent >= 100 -> NativeText.t(this, "Finalizing and adding to media library")
            else -> NativeText.percent(this, p.percent)
        }
        val speedLabel = when {
            p.speed >= 1048576 -> String.format(java.util.Locale.US, "%.1f MB/s", p.speed / 1048576.0)
            p.speed >= 1024 -> String.format(java.util.Locale.US, "%.0f KB/s", p.speed / 1024.0)
            else -> "${p.speed} B/s"
        }
        val detail = if (p.speed > 0 && p.percent in 0..99) "$status · $speedLabel" else status
        val notification = builder
            .setContentTitle(if (p.count > 1) NativeText.count(this, p.count) else "omniget")
            .setContentText("${p.title} · $detail")
            .setStyle(Notification.BigTextStyle().bigText("${p.title}\n$detail"))
            .setSmallIcon(R.drawable.ic_omniget_download)
            .setContentIntent(tap)
            .setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_PROGRESS)
            .setShowWhen(false)
            .setOngoing(true)
            .setProgress(100, p.percent.coerceAtLeast(0), p.percent < 0)
            .build()
        if (foreground) {
            if (Build.VERSION.SDK_INT >= 29) startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
            else startForeground(1, notification)
        } else manager.notify(1, notification)
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        instance = this
        refresh(true)
        if (wake == null) {
            val pm = getSystemService(POWER_SERVICE) as PowerManager
            wake = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "libreget:downloads").also { it.acquire() }
        }
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        instance = null
        progress = Progress("", -1, 1, 0)
        stopForeground(STOP_FOREGROUND_REMOVE)
        try { wake?.release() } catch (_: Throwable) {}
        super.onDestroy()
    }
}
