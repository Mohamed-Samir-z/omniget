import org.junit.Test
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import org.robolectric.shadows.ShadowLooper
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.view.View
import android.view.ViewGroup
import android.widget.*
import com.openselena.libreget.*
@RunWith(RobolectricTestRunner::class)
@Config(manifest=Config.NONE, sdk=[34], qualifiers="w412dp-h915dp-mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class SplashTest {
 private fun all(v: View): List<View> = listOf(v) + if(v is ViewGroup) (0 until v.childCount).flatMap {all(v.getChildAt(it))} else emptyList()
 private fun activity() = Robolectric.buildActivity(MainActivity::class.java).setup().get()
 @Test fun brandedCoverDismissesAndDoesNotReturn() {
  val a=activity();val s=LaunchSplash(a);s.show()
  val v=a.findViewById<ViewGroup>(android.R.id.content)
  assertTrue(all(v).filterIsInstance<TextView>().any{it.text=="omniget"})
  // Resource-free harness: use the exact bundled icon at the production size.
  all(v).filterIsInstance<ImageView>().single().setImageBitmap(BitmapFactory.decodeFile("icon.png"))
  v.measure(View.MeasureSpec.makeMeasureSpec(412,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(915,View.MeasureSpec.EXACTLY));v.layout(0,0,412,915)
  val b=Bitmap.createBitmap(412,915,Bitmap.Config.ARGB_8888);v.draw(Canvas(b));java.io.FileOutputStream("splash-light.png").use{b.compress(Bitmap.CompressFormat.PNG,100,it)}
  s.dismiss();assertFalse(all(v).filterIsInstance<TextView>().any{it.text=="omniget"});s.show();assertFalse(all(v).filterIsInstance<TextView>().any{it.text=="omniget"})
 }
 @Test fun shareEntryNeverShowsCover() {
  val a=activity();a.intent=android.content.Intent(android.content.Intent.ACTION_SEND);LaunchSplash(a).show();assertFalse(all(a.window.decorView).filterIsInstance<TextView>().any{it.text=="omniget"})
 }
 @Test fun shareArrivalRemovesCover() {
  val a=activity();val s=LaunchSplash(a);s.show();s.hideForShare();assertFalse(all(a.window.decorView).filterIsInstance<TextView>().any{it.text=="omniget"})
 }
 @Test fun slowFrontendShowsRetryInsteadOfWhiteWebView() {
  val a=activity();val s=LaunchSplash(a);s.show();ShadowLooper.idleMainLooper(12,java.util.concurrent.TimeUnit.SECONDS);assertTrue(all(a.window.decorView).filterIsInstance<Button>().any{it.text=="Retry"});assertTrue(all(a.window.decorView).filterIsInstance<TextView>().any{it.text=="omniget"})
 }
}
