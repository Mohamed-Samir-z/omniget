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
  assertEquals("Download complete",complete.extras.getString(android.app.Notification.EXTRA_TITLE))
  assertEquals("Test clip",complete.extras.getCharSequence(android.app.Notification.EXTRA_BIG_TEXT).toString())
  assertEquals(2,complete.actions.size)
  assertEquals("Open",complete.actions[0].title.toString())
  assertEquals("Share",complete.actions[1].title.toString())
  assertTrue(prayer.extras.getCharSequence(android.app.Notification.EXTRA_BIG_TEXT).toString().contains("محمد سمير"))
  assertEquals("developer-prayer",prayer.channelId)
 }
}
