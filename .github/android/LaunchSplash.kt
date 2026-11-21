package @PACKAGE@

/** Branded cover for the WebView's first paint. Never covers share-only entry. */
class LaunchSplash(private val activity: MainActivity) {
    private var cover: android.widget.FrameLayout? = null
    private var ready = false
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    private val failSafe = Runnable { dismiss() }

    fun show() {
        if (ready || cover != null || activity.intent?.action == android.content.Intent.ACTION_SEND) return
        val background = 0xfffaf9f6.toInt()
        val foreground = 0xff242824.toInt()
        val density = activity.resources.displayMetrics.density
        fun dp(n: Int) = (n * density).toInt()
        val frame = android.widget.FrameLayout(activity).apply {
            setBackgroundColor(background)
            isClickable = true
            importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_YES
            contentDescription = "omniget is starting"
        }
        val column = android.widget.LinearLayout(activity).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            gravity = android.view.Gravity.CENTER
        }
        val icon = android.widget.ImageView(activity).apply {
            setImageResource(R.drawable.omniget_launch)
            scaleType = android.widget.ImageView.ScaleType.FIT_CENTER
            contentDescription = null
        }
        column.addView(icon, android.widget.LinearLayout.LayoutParams(dp(288), dp(288)))
        val label = android.widget.TextView(activity).apply {
            text = "omniget"
            textSize = 26f
            setTextColor(foreground)
            gravity = android.view.Gravity.CENTER
            setTypeface(android.graphics.Typeface.DEFAULT, android.graphics.Typeface.BOLD)
        }
        column.addView(label, android.widget.LinearLayout.LayoutParams(-2, -2).apply { topMargin = dp(20) })
        frame.addView(column, android.widget.FrameLayout.LayoutParams(-2, -2, android.view.Gravity.CENTER))
        cover = frame
        activity.addContentView(frame, android.view.ViewGroup.LayoutParams(-1, -1))
        // A frontend error must not trap the user behind a permanent splash.
        handler.postDelayed(failSafe, 12000)
    }

    fun dismiss() {
        ready = true
        handler.removeCallbacks(failSafe)
        cover?.let { (it.parent as? android.view.ViewGroup)?.removeView(it) }
        cover = null
    }

    fun hideForShare() {
        handler.removeCallbacks(failSafe)
        cover?.let { (it.parent as? android.view.ViewGroup)?.removeView(it) }
        cover = null
    }

    fun dispose() = dismiss()
}
