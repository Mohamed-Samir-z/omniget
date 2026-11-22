package @PACKAGE@

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.os.Build

/** Alert for each new event, not each update. System/user settings remain final. */
object NativeNotifications {
    private const val ALERT_SUFFIX = "-alerts-v1"

    internal fun mayMigrate(sdk: Int, importance: Int, importanceChosen: Boolean, soundChosen: Boolean): Boolean =
        sdk >= 30 && importance != NotificationManager.IMPORTANCE_NONE && !importanceChosen && !soundChosen

    @JvmStatic fun channel(context: Context, legacyId: String, label: String): String {
        if (Build.VERSION.SDK_INT < 26) return legacyId
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val id = legacyId + ALERT_SUFFIX
        // Never recreate/delete a channel to undo a user choice, including on the new channel.
        if (manager.getNotificationChannel(id) != null) return id
        val old = manager.getNotificationChannel(legacyId)
        if (old != null && !mayMigrate(Build.VERSION.SDK_INT, old.importance,
                if (Build.VERSION.SDK_INT >= 29) old.hasUserSetImportance() else true,
                if (Build.VERSION.SDK_INT >= 30) old.hasUserSetSound() else true)) {
            android.util.Log.i("libreget", "Notification channel $legacyId keeps existing settings; change its sound/pop-up in Android settings if desired")
            return legacyId
        }
        val alert = NotificationChannel(id, label, NotificationManager.IMPORTANCE_HIGH)
        alert.setSound(RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION),
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
        if (old != null) {
            // Keep unrelated channel choices when migrating only app-default alert behavior.
            alert.enableLights(old.shouldShowLights())
            alert.lightColor = old.lightColor
            old.vibrationPattern?.let { alert.vibrationPattern = it }
            alert.enableVibration(old.shouldVibrate())
            alert.lockscreenVisibility = old.lockscreenVisibility
            alert.setShowBadge(old.canShowBadge())
            alert.group = old.group
        }
        manager.createNotificationChannel(alert)
        return id
    }

    internal fun builder(context: Context, legacyId: String, label: String): Notification.Builder {
        return if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, channel(context, legacyId, label))
        else Notification.Builder(context).setPriority(Notification.PRIORITY_HIGH).setDefaults(Notification.DEFAULT_SOUND)
    }
}
