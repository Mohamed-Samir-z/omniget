import org.junit.Before
import org.junit.Test
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import org.robolectric.shadows.ShadowDialog
import org.robolectric.shadows.ShadowLooper
import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import android.view.ViewGroup
import android.widget.*
import java.io.FileOutputStream
import com.openselena.libreget.MainActivity
import com.openselena.libreget.SharePopup
@RunWith(RobolectricTestRunner::class)
@Config(manifest=Config.NONE, sdk=[34], qualifiers="w412dp-h915dp-mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class PopupRenderTest {
 @Before fun resetState() {
  com.openselena.libreget.MediaRuntime.ready=true
  com.openselena.libreget.MediaRuntime.error=null
  org.robolectric.RuntimeEnvironment.getApplication().getSharedPreferences("omniget.share.choices",0).edit().clear().commit()
 }
 @Test fun runtimeMustBeReadyEvenWhenAppHandleIsReady() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get()
  com.openselena.libreget.MediaRuntime.ready=false
  SharePopup(a).show("https://example.com/cold-engine"); click("Download"); Thread.sleep(250)
  assertEquals(0,a.calls)
  com.openselena.libreget.MediaRuntime.ready=true; settle(a); assertEquals(1,a.calls)
 }
 @Test fun acceptedChoicesSurviveNewPopupAndReset() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get()
  SharePopup(a).show("https://example.com/one"); click("MP3"); click("Download"); settle(a)
  SharePopup(a).show("https://example.com/two")
  assertTrue(all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<RadioButton>().first{it.text=="MP3"}.isChecked)
  click("Reset choices")
  assertTrue(all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<RadioButton>().first{it.text=="Video"}.isChecked)
 }

 private fun all(v: View): List<View> = listOf(v) + if(v is ViewGroup) (0 until v.childCount).flatMap {all(v.getChildAt(it))} else emptyList()
 private fun click(text: String) { all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<TextView>().first {it.text.toString()==text}.performClick() }
 private fun settle(a: MainActivity) {
  for(i in 0..100) { if(a.calls>0) break; Thread.sleep(20) }
  Thread.sleep(50); ShadowLooper.idleMainLooper()
 }
 private fun shot(path:String) {
  java.io.File("native-renders").mkdirs()
  ShadowLooper.idleMainLooper(1000,java.util.concurrent.TimeUnit.MILLISECONDS)
  val v=ShadowDialog.getLatestDialog().window!!.decorView
  v.measure(View.MeasureSpec.makeMeasureSpec(412,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(915,View.MeasureSpec.AT_MOST))
  v.layout(0,0,412,v.measuredHeight); all(v).forEach { it.jumpDrawablesToCurrentState(); if(it is RadioButton) it.buttonDrawable?.jumpToCurrentState() }
  val b=Bitmap.createBitmap(412,v.measuredHeight,Bitmap.Config.ARGB_8888); v.draw(Canvas(b))
  FileOutputStream(path).use { b.compress(Bitmap.CompressFormat.PNG,100,it) }
 }
 @Test fun renderAndDownloadMp3() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get(); val p=SharePopup(a)
  p.show("https://www.youtube.com/watch?v=sample")
  assertFalse(a.contentVisible)
  assertTrue(all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<RadioButton>().first {it.text=="Video"}.isChecked)
  shot("native-renders/native-popup.png")
  click("MP3"); shot("native-renders/native-mp3.png"); click("Download"); settle(a)
  assertEquals(listOf("https://www.youtube.com/watch?v=sample","mp3","best","download"),a.last)
  assertFalse(a.contentVisible)
  shot("native-renders/native-confirm.png"); click("Not now"); assertTrue(a.left)
 }
 @Test fun moreOptionsPrefillsAndRestoresApp() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get(); val p=SharePopup(a)
  p.show("https://example.com/first"); p.show("https://example.com/second"); click("More options in the app"); settle(a)
  assertEquals("https://example.com/second",a.last[0]); assertEquals("prefill",a.last[3]); assertTrue(a.contentVisible)
 }
 @Test fun queueErrorStaysVisibleAndRetryable() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get(); a.result="Choose a folder first"
  SharePopup(a).show("https://example.com/video"); click("Download"); settle(a)
  assertFalse(a.contentVisible)
  assertTrue(all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<TextView>().any {it.text=="Choose a folder first"})
  assertTrue(all(ShadowDialog.getLatestDialog().window!!.decorView).filterIsInstance<Button>().first {it.text=="Download"}.isEnabled)
 }
 @Test fun coldStartWaitsForReadyBeforeCallingQueue() {
  val a=Robolectric.buildActivity(MainActivity::class.java).setup().get(); a.ready=false
  SharePopup(a).show("https://example.com/cold"); click("Download"); Thread.sleep(250)
  assertEquals(0,a.calls); a.ready=true; settle(a); assertEquals(1,a.calls)
 }
}
