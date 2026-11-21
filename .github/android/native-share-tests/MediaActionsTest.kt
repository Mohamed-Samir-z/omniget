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
}
