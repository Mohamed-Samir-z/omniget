import android.app.Notification
import android.content.Context
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.TextView
import com.openselena.libreget.DownloadNotification
import com.openselena.libreget.NativeText
import com.openselena.libreget.R
import java.io.File
import java.io.FileOutputStream
import java.util.Locale
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** Inflates the real aapt2-compiled app XML inside Android's decorated template. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk=[34], qualifiers="w412dp-h915dp-mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class DownloadNotificationTest {
 private fun context(language: String, dark: Boolean = false, font: Float = 1f): Context {
  val app=RuntimeEnvironment.getApplication()
  val config=Configuration(app.resources.configuration)
  config.setLocale(Locale(language)); config.setLayoutDirection(Locale(language))
  config.uiMode=(config.uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or if(dark) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
  config.fontScale=font
  return app.createConfigurationContext(config).also { NativeText.language(it,language) }
 }
 private fun layout(view: View) {
  view.measure(View.MeasureSpec.makeMeasureSpec(380,View.MeasureSpec.EXACTLY),View.MeasureSpec.makeMeasureSpec(1000,View.MeasureSpec.AT_MOST))
  view.layout(0,0,380,view.measuredHeight)
 }
 @Test fun compactExpandedAndHeadsUpAreCenteredInBothLanguages() {
  for(language in listOf("en","ar")) {
   val context=context(language)
   val title=if(language=="ar") "مقدمة في البرمجة Kotlin 2026" else "Programming lesson 2026"
   val n=DownloadNotification.build(context,title,42,1,1572864)
   assertEquals("downloads-alerts-v1",n.channelId)
   assertTrue(n.flags and Notification.FLAG_ONLY_ALERT_ONCE != 0)
   assertTrue(n.flags and Notification.FLAG_ONGOING_EVENT != 0)
   assertEquals(Notification.CATEGORY_PROGRESS,n.category)
   val builder=Notification.Builder.recoverBuilder(context,n)
   for((kind,remote) in listOf("compact" to builder.createContentView(),"expanded" to builder.createBigContentView(),"heads-up" to builder.createHeadsUpContentView())) {
    assertNotNull(remote)
    val host=FrameLayout(context)
    val root=remote.apply(context,host);layout(root)
    fun descendants(v: View): List<View> = listOf(v) + if(v is ViewGroup) (0 until v.childCount).flatMap {descendants(v.getChildAt(it))} else emptyList()
    assertEquals("Exactly one progress bar, no duplicate system bar",1,descendants(root).filterIsInstance<ProgressBar>().size)
    val body=root.findViewById<ViewGroup>(R.id.omniget_download_body)
    val heading=root.findViewById<TextView>(R.id.omniget_download_title)
    val detail=root.findViewById<TextView>(R.id.omniget_download_detail)
    val progress=root.findViewById<ProgressBar>(R.id.omniget_download_progress)
    assertEquals(body.paddingLeft,body.paddingRight)
    assertTrue(body.paddingLeft>=16)
    assertEquals(heading.left,body.width-heading.right)
    assertEquals(detail.left,body.width-detail.right)
    assertEquals(progress.left,body.width-progress.right)
    assertEquals(android.text.Layout.Alignment.ALIGN_CENTER,heading.layout.getParagraphAlignment(0))
    assertEquals(android.text.Layout.Alignment.ALIGN_CENTER,detail.layout.getParagraphAlignment(0))
    assertEquals(42,progress.progress)
    assertTrue(heading.layout.lineCount>0)
    assertTrue(progress.bottom<=body.height-body.paddingBottom)
    shot(root,"notification-$language-$kind.png",false)
   }
  }
 }
 @Test fun actualCompiledContentSupportsDarkAndLargeTextAndLongTitles() {
  for(language in listOf("en","ar")) for(dark in listOf(false,true)) {
   val context=context(language,dark,1.5f)
   val title=if(language=="ar") "درس طويل جدا في البرمجة مع أمثلة Kotlin 2026 وشرح مفصل للمبتدئين" else "A very long programming lesson title with Kotlin 2026 and detailed examples for beginners"
   val remote=DownloadNotification.content(context,title,NativeText.t(context,"Finalizing and adding to media library"),100,true)
   val root=remote.apply(context,FrameLayout(context));layout(root)
   val heading=root.findViewById<TextView>(R.id.omniget_download_title)
   val detail=root.findViewById<TextView>(R.id.omniget_download_detail)
   val progress=root.findViewById<ProgressBar>(R.id.omniget_download_progress)
   assertTrue(heading.bottom<=detail.top)
   assertTrue(detail.bottom<=progress.top)
   assertTrue(progress.bottom<=root.height-root.paddingBottom)
   assertTrue(heading.layout.lineCount in 1..2)
   shot(root,"notification-$language-${if(dark) "dark" else "light"}-large.png",dark)
  }
 }
 @Test fun multipleActiveDownloadsRemainVisibleInCustomStatus() {
  val context=context("en")
  val notification=DownloadNotification.build(context,"Video",35,2,0)
  val view=Notification.Builder.recoverBuilder(context,notification).createBigContentView().apply(context,FrameLayout(context))
  assertTrue(view.findViewById<TextView>(R.id.omniget_download_detail).text.toString().contains(NativeText.count(context,2)))
 }
 @Test fun preparingKeepsIndeterminateProgressAndCompletionUsesFinalizing() {
  val context=context("en")
  for(percent in listOf(-1,100)) {
   val n=DownloadNotification.build(context,"Video",percent,2,0)
   assertEquals(Notification.CATEGORY_PROGRESS,n.category)
   val view=DownloadNotification.content(context,"Video",if(percent<0) "Preparing download" else "Finalizing",percent,false).apply(context,FrameLayout(context))
   assertEquals(percent<0,view.findViewById<ProgressBar>(R.id.omniget_download_progress).isIndeterminate)
  }
 }
 private fun shot(view: View, name: String, dark: Boolean) {
  File("native-renders").mkdirs()
  val bitmap=Bitmap.createBitmap(view.width,view.height,Bitmap.Config.ARGB_8888)
  val canvas=Canvas(bitmap);canvas.drawColor(if(dark) Color.rgb(35,35,35) else Color.WHITE)
  view.jumpDrawablesToCurrentState();view.draw(canvas)
  FileOutputStream(File("native-renders",name)).use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
 }
}
