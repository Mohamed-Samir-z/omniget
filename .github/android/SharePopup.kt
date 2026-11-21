package @PACKAGE@

import android.app.Dialog
import android.graphics.Color
import android.graphics.Typeface
import android.content.res.ColorStateList
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.*

/** Native share UI. The existing Tauri activity owns the queue, but its opaque
 * web content stays hidden until the user explicitly chooses the full app. */
class SharePopup(private val activity: MainActivity) {
    private var dialog: Dialog? = null
    private var generation = 0
    private val accent = Color.rgb(255, 165, 0)
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
    private fun rounded(color: Int) = GradientDrawable().apply {
        setColor(color)
        cornerRadius = dp(22).toFloat()
    }
    private fun label(text: String, size: Float = 15f) = TextView(activity).apply {
        this.text = text
        textSize = size
        setTextColor(Color.WHITE)
        setPadding(0, dp(7), 0, dp(7))
    }
    private fun button(text: String, primary: Boolean = false) = Button(activity).apply {
        this.text = text
        isAllCaps = false
        setTextColor(if (primary) Color.BLACK else Color.WHITE)
        background = rounded(if (primary) accent else Color.rgb(52, 52, 56))
        layoutParams = LinearLayout.LayoutParams(-1, dp(50)).apply { topMargin = dp(10) }
    }
    private fun hideApp() {
        activity.setShareContentVisible(false)
        activity.window.setBackgroundDrawableResource(android.R.color.transparent)
    }
    fun dispose() {
        generation++
        dialog?.setOnCancelListener(null)
        dialog?.setOnDismissListener(null)
        dialog?.dismiss()
        dialog = null
    }
    fun openFullApp() {
        generation++
        dialog?.setOnDismissListener(null)
        dialog?.dismiss()
        dialog = null
        activity.setShareContentVisible(true)
    }
    private fun leave() {
        generation++
        dialog?.setOnDismissListener(null)
        dialog?.dismiss()
        dialog = null
        activity.moveTaskToBack(true)
    }
    fun show(shared: String) {
        generation++
        val token = generation
        dialog?.setOnDismissListener(null)
        dialog?.dismiss()
        hideApp()
        val box = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(18), dp(24), dp(24))
            background = rounded(Color.rgb(34, 34, 38))
        }
        val title = label("Download shared link", 20f).apply { setTypeface(typeface, Typeface.BOLD) }
        box.addView(title)
        val url = shared.split(Regex("\\s+")).firstOrNull { it.startsWith("https://") || it.startsWith("http://") } ?: shared
        box.addView(label(url.take(180), 12f).apply { maxLines = 3; setTextColor(Color.LTGRAY) })
        box.addView(label("Type"))
        val types = RadioGroup(activity).apply { orientation = LinearLayout.HORIZONTAL }
        val video = RadioButton(activity).apply { id = View.generateViewId(); text = "Video"; setTextColor(Color.WHITE); buttonTintList = ColorStateList.valueOf(accent) }
        val mp3 = RadioButton(activity).apply { id = View.generateViewId(); text = "MP3"; setTextColor(Color.WHITE); buttonTintList = ColorStateList.valueOf(accent) }
        types.addView(video)
        types.addView(mp3)
        video.isChecked = true
        box.addView(types)
        val qualityLabel = label("Video quality")
        box.addView(qualityLabel)
        val qualities = arrayOf("Best available", "1080p", "720p", "480p", "360p")
        val quality = Spinner(activity).apply {
            adapter = object : ArrayAdapter<String>(activity, android.R.layout.simple_spinner_dropdown_item, qualities) {
                override fun getView(position: Int, convertView: View?, parent: ViewGroup): View =
                    (super.getView(position, convertView, parent) as TextView).apply { setTextColor(Color.BLACK) }
                override fun getDropDownView(position: Int, convertView: View?, parent: ViewGroup): View =
                    (super.getDropDownView(position, convertView, parent) as TextView).apply {
                        setTextColor(Color.BLACK); setBackgroundColor(Color.WHITE)
                    }
            }
            background = rounded(Color.rgb(225, 225, 230))
        }
        box.addView(quality, LinearLayout.LayoutParams(-1, dp(44)))
        types.setOnCheckedChangeListener { _, id ->
            val visible = if (id == video.id) View.VISIBLE else View.GONE
            quality.visibility = visible
            qualityLabel.visibility = visible
        }
        val error = label("", 13f).apply { setTextColor(Color.rgb(255, 150, 140)); visibility = View.GONE }
        box.addView(error)
        val download = button("Download", true)
        val more = button("More options in the app")
        val cancel = button("Not now")
        box.addView(download)
        box.addView(more)
        box.addView(cancel)
        val scroll = ScrollView(activity).apply { addView(box) }
        val modal = Dialog(activity).apply {
            requestWindowFeature(android.view.Window.FEATURE_NO_TITLE)
            setContentView(scroll)
            setCanceledOnTouchOutside(true)
            setOnCancelListener { leave() }
            window?.apply {
                setBackgroundDrawableResource(android.R.color.transparent)
                addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
                setDimAmount(0.48f)
            }
        }
        dialog = modal
        modal.show()
        modal.window?.apply {
            setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
            setGravity(Gravity.BOTTOM)
        }
        fun runAction(action: String) {
            if (token != generation) return
            download.isEnabled = false
            more.isEnabled = false
            cancel.isEnabled = false
            modal.setCancelable(false)
            error.visibility = View.GONE
            download.text = "Preparing..."
            // Capture UI state on the main thread, never from the worker.
            val mode = if (mp3.isChecked) "mp3" else "video"
            val selectedQuality = if (quality.selectedItemPosition == 0) "best" else qualities[quality.selectedItemPosition]
            Thread {
                var result = "The app is still starting. Try again or open omniget first."
                try {
                    var ready = false
                    for (attempt in 0 until 100) {
                        try { ready = activity.nativeShareReady() } catch (_: UnsatisfiedLinkError) { }
                        if (ready) break
                        Thread.sleep(200)
                    }
                    if (ready) result = activity.nativeShareAction(shared, mode, selectedQuality, action)
                } catch (_: Throwable) { result = "Could not prepare this download. Open omniget for more options." }
                Handler(Looper.getMainLooper()).post {
                    if (token != generation || activity.isFinishing || activity.isDestroyed) return@post
                    if (result.isNotEmpty()) {
                        error.text = result
                        error.visibility = View.VISIBLE
                        download.text = "Download"
                        download.isEnabled = true
                        more.isEnabled = true
                        cancel.isEnabled = true
                        modal.setCancelable(true)
                    } else if (action == "prefill") {
                        openFullApp()
                    } else {
                        // Enqueued is accurate even when Wi-Fi policy delays transfer.
                        box.removeAllViews()
                        box.addView(label("Download added", 21f))
                        box.addView(label("You can keep using the app underneath. Progress appears in the download notification.", 14f))
                        box.addView(button("Open omniget").apply { setOnClickListener { openFullApp() } })
                        box.addView(button("Not now", true).apply { setOnClickListener { leave() } })
                        modal.setCancelable(true)
                    }
                }
            }.start()
        }
        download.setOnClickListener { runAction("download") }
        more.setOnClickListener { runAction("prefill") }
        cancel.setOnClickListener { leave() }
    }
}
