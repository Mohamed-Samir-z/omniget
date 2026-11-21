import android.content.Context
import androidx.work.*
import androidx.work.testing.WorkManagerTestInitHelper
import androidx.work.testing.SynchronousExecutor
import org.junit.Test
import org.junit.Before
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import com.openselena.libreget.ScheduledDownloads
import org.json.JSONObject
import org.json.JSONArray
import java.util.UUID

@RunWith(RobolectricTestRunner::class)
@Config(manifest=Config.NONE, sdk=[34])
class ScheduleTest {
 lateinit var context: Context
 @Before fun setup() {
  context=RuntimeEnvironment.getApplication()
  context.getSharedPreferences(ScheduledDownloads.TAG,0).edit().clear().commit()
  WorkManagerTestInitHelper.initializeTestWorkManager(context, Configuration.Builder().setExecutor(SynchronousExecutor()).build())
 }
 fun spec()=JSONObject().put("url","https://example.com/test.mp4").put("output",context.cacheDir.path).put("at",System.currentTimeMillis()+3600000).toString()
 @Test fun enqueuePersistsWithoutAnActivityAndCanBeCancelled() {
  val id=ScheduledDownloads.schedule(context,spec())
  assertEquals(WorkInfo.State.ENQUEUED,WorkManager.getInstance(context).getWorkInfoById(UUID.fromString(id)).get()!!.state)
  val row=JSONArray(ScheduledDownloads.list(context)).getJSONObject(0)
  assertEquals(id,row.getString("id"));assertEquals("ENQUEUED",row.getString("state"))
  try { ScheduledDownloads.forget(context,id);fail("Cannot remove active work") } catch (_:IllegalArgumentException) {}
  ScheduledDownloads.cancel(context,id)
  assertEquals("CANCELLED",JSONArray(ScheduledDownloads.list(context)).getJSONObject(0).getString("state"))
  ScheduledDownloads.forget(context,id)
  assertEquals(0,JSONArray(ScheduledDownloads.list(context)).length())
 }
 @Test fun malformedUrlAndPastTimeCannotCreateSchedules() {
  for (json in listOf(JSONObject(spec()).put("url","file:///private").toString(),JSONObject(spec()).put("at",1).toString())) {
   val result=JSONObject(ScheduledDownloads.dispatch(context,"schedule",json))
   assertFalse(result.getBoolean("ok"))
  }
  assertEquals(0,JSONArray(ScheduledDownloads.list(context)).length())
 }
}
