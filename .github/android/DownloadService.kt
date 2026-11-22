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
        val notification = DownloadNotification.build(this, p.title, p.percent, p.count, p.speed)
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
