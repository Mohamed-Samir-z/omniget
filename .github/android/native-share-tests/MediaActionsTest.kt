import android.content.Intent
import org.junit.Test
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import com.openselena.libreget.MediaActions
import java.io.File

@RunWith(RobolectricTestRunner::class)
@Config(manifest=Config.NONE, sdk=[34])
class MediaActionsTest {
 @Test fun missingFilesCannotBeOpenedOrShared() {
  val context=RuntimeEnvironment.getApplication()
  for (share in listOf(false,true)) {
   try { MediaActions.intent(context,File(context.cacheDir,"absent.mp4").path,share); fail("Missing file must be rejected") }
   catch (expected: IllegalArgumentException) { assertTrue(expected.message!!.contains("missing")) }
  }
 }
 @Test fun emptyFilesCannotBeShared() {
  val context=RuntimeEnvironment.getApplication()
  val file=File(context.cacheDir,"empty.mp4");file.writeBytes(byteArrayOf())
  try { MediaActions.intent(context,file.path,true);fail("Empty file must be rejected") }
  catch (expected: IllegalArgumentException) { assertTrue(expected.message!!.contains("empty")) }
 }
 @Test fun prayerIsSeparateAndKeepsCompletionActions() {
  val context=RuntimeEnvironment.getApplication()
  val manager=context.getSystemService(android.content.Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
  manager.cancelAll()
  manager.createNotificationChannel(android.app.NotificationChannel("completed","Completed",android.app.NotificationManager.IMPORTANCE_DEFAULT))
  val pending=android.app.PendingIntent.getActivity(context,7,Intent(Intent.ACTION_VIEW,android.net.Uri.parse("https://example.com")),android.app.PendingIntent.FLAG_IMMUTABLE)
  MediaActions.postCompletion(context,314,"Test clip",pending,pending)
  val shadow=org.robolectric.Shadows.shadowOf(manager)
  val complete=shadow.getNotification(314)
  val prayer=shadow.getNotification("developer-prayer",314)
  assertNotNull(complete);assertNotNull(prayer)
  assertEquals(2,shadow.allNotifications.size)
  assertEquals("completed-alerts-v1",complete.channelId)
  assertTrue((complete.flags and android.app.Notification.FLAG_ONLY_ALERT_ONCE) != 0)
  assertEquals("Download complete",complete.extras.getString(android.app.Notification.EXTRA_TITLE))
  assertEquals("Test clip",complete.extras.getCharSequence(android.app.Notification.EXTRA_BIG_TEXT).toString())
  assertEquals(2,complete.actions.size)
  assertEquals("Open",complete.actions[0].title.toString())
  assertEquals("Share",complete.actions[1].title.toString())
  assertTrue(prayer.extras.getCharSequence(android.app.Notification.EXTRA_BIG_TEXT).toString().contains("محمد سمير"))
  assertEquals("developer-prayer-alerts-v1",prayer.channelId)
 }
}

@RunWith(RobolectricTestRunner::class)
@Config(manifest=Config.NONE, sdk=[34])
class NotificationPolicyTest {
 @Test fun everyNativeCategoryRequestsBannerAndSound() {
  val context=RuntimeEnvironment.getApplication()
  val manager=context.getSystemService(android.content.Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
  for (id in listOf("downloads","scheduled","completed","developer-prayer","default")) {
   val channelId=com.openselena.libreget.NativeNotifications.channel(context,id,"Test $id")
   val channel=manager.getNotificationChannel(channelId)
   assertEquals("$id-alerts-v1",channelId)
   assertEquals(android.app.NotificationManager.IMPORTANCE_HIGH,channel.importance)
   assertNotNull(channel.sound)
  }
 }
 @Test fun defaultLowChannelMigratesWithoutDeletingOldChannel() {
  val context=RuntimeEnvironment.getApplication()
  val manager=context.getSystemService(android.content.Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
  manager.createNotificationChannel(android.app.NotificationChannel("old-low","Old",android.app.NotificationManager.IMPORTANCE_LOW))
  val id=com.openselena.libreget.NativeNotifications.channel(context,"old-low","Alert")
  assertEquals("old-low-alerts-v1",id)
  assertNotNull(manager.getNotificationChannel("old-low"))
  assertEquals(android.app.NotificationManager.IMPORTANCE_HIGH,manager.getNotificationChannel(id).importance)
 }
 @Test fun blockedLegacyAndExistingAlertChannelsAreNotBypassed() {
  val context=RuntimeEnvironment.getApplication()
  val manager=context.getSystemService(android.content.Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
  manager.createNotificationChannel(android.app.NotificationChannel("blocked","Blocked",android.app.NotificationManager.IMPORTANCE_NONE))
  assertEquals("blocked",com.openselena.libreget.NativeNotifications.channel(context,"blocked","Blocked"))
  assertNull(manager.getNotificationChannel("blocked-alerts-v1"))
  manager.createNotificationChannel(android.app.NotificationChannel("muted-alerts-v1","Muted",android.app.NotificationManager.IMPORTANCE_LOW).apply { setSound(null,null) })
  assertEquals("muted-alerts-v1",com.openselena.libreget.NativeNotifications.channel(context,"muted","Muted"))
  assertEquals(android.app.NotificationManager.IMPORTANCE_LOW,manager.getNotificationChannel("muted-alerts-v1").importance)
  assertNull(manager.getNotificationChannel("muted-alerts-v1").sound)
 }
 @Test fun explicitChoicesAndUnknownOlderChoicesNeverAutoMigrate() {
  val high=android.app.NotificationManager.IMPORTANCE_HIGH
  assertTrue(com.openselena.libreget.NativeNotifications.mayMigrate(30,high,false,false))
  assertFalse(com.openselena.libreget.NativeNotifications.mayMigrate(29,high,false,false))
  assertFalse(com.openselena.libreget.NativeNotifications.mayMigrate(34,high,true,false))
  assertFalse(com.openselena.libreget.NativeNotifications.mayMigrate(34,high,false,true))
  assertFalse(com.openselena.libreget.NativeNotifications.mayMigrate(34,0,false,false))
 }
}
