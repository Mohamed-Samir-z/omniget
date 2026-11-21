package @PACKAGE@

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.pm.ServiceInfo
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import androidx.work.*
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit

/** WorkManager owns persistence and reboot recovery. No Activity, WebView or Rust queue is required. */
object ScheduledDownloads {
    @JvmStatic fun dispatch(context: Context, action: String, payload: String): String {
        return try {
            val value = when (action) {
                "schedule" -> schedule(context, payload)
                "list" -> list(context)
                "run-now" -> replace(context, payload, true)
                "reschedule" -> replace(context, payload, false)
                "locale" -> { NativeText.language(context, payload); "" }
                "cancel" -> { cancel(context, payload); "" }
                "forget" -> { forget(context, payload); "" }
                else -> error("Unsupported schedule action")
            }
            JSONObject().put("ok", true).put("value", value).toString()
        } catch (e: Exception) { JSONObject().put("ok", false).put("error", e.cause?.message ?: e.message ?: "Schedule operation failed").toString() }
    }
    const val TAG = "omniget.scheduled.download"
    private fun prefs(context: Context) = context.getSharedPreferences(TAG, Context.MODE_PRIVATE)

    @JvmStatic @Synchronized fun schedule(context: Context, json: String): String = enqueue(context, json, false)
    private fun enqueue(context: Context, json: String, immediate: Boolean): String {
        val spec = JSONObject(json)
        val uri = android.net.Uri.parse(spec.getString("url"))
        require(uri.scheme in listOf("http", "https") && !uri.host.isNullOrBlank()) { NativeText.t(context, "A valid HTTP or HTTPS link is required") }
        val delay = spec.getLong("at") - System.currentTimeMillis()
        require(immediate || delay > 0) { NativeText.t(context, "Choose a future download time") }
        require(json.toByteArray().size < 8000) { NativeText.t(context, "Scheduled download options are too large") }
        val request = OneTimeWorkRequest.Builder(ScheduledDownloadWorker::class.java)
            .setInputData(Data.Builder().putString("spec", json).build())
            .setInitialDelay(delay.coerceAtLeast(0), TimeUnit.MILLISECONDS)
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 15, TimeUnit.MINUTES)
            .addTag(TAG).build()
        val id = request.id.toString()
        // Save the description first. A failed enqueue removes it; a process death
        // in between leaves a visible interrupted record instead of a false success.
        check(prefs(context).edit().putString(id, json).commit()) { NativeText.t(context, "Could not save the schedule") }
        try { WorkManager.getInstance(context).enqueue(request).result.get(30, TimeUnit.SECONDS) }
        catch (e: Exception) { prefs(context).edit().remove(id).commit(); throw e }
        return id
    }
    @JvmStatic @Synchronized fun replace(context: Context, payload: String, immediate: Boolean): String {
        val change = JSONObject(payload)
        val id = change.getString("id")
        val uuid = UUID.fromString(id)
        val original = prefs(context).getString(id, null) ?: error(NativeText.t(context, "Schedule not found"))
        val state = WorkManager.getInstance(context).getWorkInfoById(uuid).get(20, TimeUnit.SECONDS)?.state
        require(state != WorkInfo.State.RUNNING && state != WorkInfo.State.SUCCEEDED) { NativeText.t(context, "This download has already started or finished. Refresh the list.") }
        val spec = JSONObject(original)
        val at = if (immediate) System.currentTimeMillis() else change.getLong("at")
        require(immediate || at > System.currentTimeMillis()) { NativeText.t(context, "Choose a future download time") }
        spec.put("at", at).remove("finishedAt")
        spec.remove("lastState"); spec.remove("lastPath"); spec.remove("lastError")
        // Cancel and confirm the old work first, so two workers can never be queued by this edit.
        WorkManager.getInstance(context).cancelWorkById(uuid).result.get(20, TimeUnit.SECONDS)
        record(context, id, "CANCELLED")
        return try {
            val next = enqueue(context, spec.toString(), immediate)
            prefs(context).edit().remove(id).commit()
            next
        } catch (e: Exception) {
            // Preserve a recoverable visible record if replacing could not enqueue.
            record(context, id, "INTERRUPTED", error = e.message ?: "Could not update schedule")
            throw e
        }
    }
    @Synchronized fun record(context: Context, id: String, state: String, path: String = "", error: String = "") {
        val original = prefs(context).getString(id, null) ?: return
        val spec = JSONObject(original).put("lastState", state).put("lastPath", path).put("lastError", error)
        if (state in listOf("SUCCEEDED", "FAILED", "CANCELLED")) spec.put("finishedAt", System.currentTimeMillis())
        prefs(context).edit().putString(id, spec.toString()).commit()
    }
    @JvmStatic fun list(context: Context): String {
        val states = WorkManager.getInstance(context).getWorkInfosByTag(TAG).get(20, TimeUnit.SECONDS).associateBy { it.id.toString() }
        val records = JSONArray()
        prefs(context).all.forEach { (id, value) ->
            try {
                val spec = JSONObject(value as String)
                val work = states[id]
                records.put(JSONObject().put("id", id).put("url", spec.getString("url")).put("at", spec.getLong("at"))
                    .put("finishedAt", spec.optLong("finishedAt", spec.getLong("at")))
                    .put("state", if (spec.optString("lastState") == "INTERRUPTED") "INTERRUPTED" else work?.state?.name ?: spec.optString("lastState", "INTERRUPTED"))
                    .put("percent", work?.progress?.getInt("percent", -1) ?: -1)
                    .put("path", work?.outputData?.getString("path") ?: spec.optString("lastPath"))
                    .put("error", work?.outputData?.getString("error") ?: spec.optString("lastError")))
            } catch (e: Exception) { android.util.Log.w("libreget", "Invalid schedule record", e) }
        }
        return records.toString()
    }
    @JvmStatic fun cancel(context: Context, id: String) {
        WorkManager.getInstance(context).cancelWorkById(UUID.fromString(id)).result.get(20, TimeUnit.SECONDS)
        record(context, id, "CANCELLED")
    }
    @JvmStatic fun forget(context: Context, id: String) {
        val state = WorkManager.getInstance(context).getWorkInfoById(UUID.fromString(id)).get(20, TimeUnit.SECONDS)?.state
        require(state == null || state.isFinished) { NativeText.t(context, "Cancel this scheduled download before removing it") }
        check(prefs(context).edit().remove(id).commit()) { NativeText.t(context, "Could not remove the schedule") }
    }
}

class ScheduledDownloadWorker(context: Context, params: WorkerParameters) : Worker(context, params) {
    private val processId = "schedule-$id"
    override fun onStopped() {
        YoutubeDL.getInstance().destroyProcessById(processId)
        super.onStopped()
    }
    private fun onWifi(): Boolean {
        val manager = applicationContext.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = manager.activeNetwork ?: return false
        return manager.getNetworkCapabilities(network)?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
    }
    private fun foreground(percent: Int): ForegroundInfo {
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(NotificationChannel("scheduled", NativeText.t(applicationContext, "Scheduled downloads"), NotificationManager.IMPORTANCE_LOW))
        val tap = applicationContext.packageManager.getLaunchIntentForPackage(applicationContext.packageName)?.let {
            PendingIntent.getActivity(applicationContext, 0, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(applicationContext, "scheduled") else Notification.Builder(applicationContext)
        val notification = builder.setSmallIcon(R.drawable.ic_omniget_download).setContentTitle(NativeText.t(applicationContext, "omniget · Scheduled download"))
            .setContentText(if (percent < 0) NativeText.t(applicationContext, "Preparing download") else NativeText.percent(applicationContext, percent))
            .setProgress(100, percent.coerceAtLeast(0), percent < 0).setOngoing(true).setOnlyAlertOnce(true).setContentIntent(tap)
            .addAction(Notification.Action.Builder(null, NativeText.t(applicationContext, "Cancel"), WorkManager.getInstance(applicationContext).createCancelPendingIntent(id)).build()).build()
        val notificationId = (id.hashCode() and 0x1fffffff) + 200
        return if (Build.VERSION.SDK_INT >= 29) ForegroundInfo(notificationId, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
            else ForegroundInfo(notificationId, notification)
    }
    override fun doWork(): Result {
        var stage: File? = null
        var saved: File? = null
        var networkCallback: ConnectivityManager.NetworkCallback? = null
        val wifiLost = java.util.concurrent.atomic.AtomicBoolean(false)
        val connectivity = applicationContext.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        return try {
            val spec = JSONObject(inputData.getString("spec") ?: error("Missing schedule options"))
            if (spec.optBoolean("wifiOnly") && !onWifi()) return Result.retry()
            setForegroundAsync(foreground(-1)).get(20, TimeUnit.SECONDS)
            MediaRuntime.start(applicationContext)
            val deadline = android.os.SystemClock.elapsedRealtime() + 120000
            while (!MediaRuntime.ready && MediaRuntime.error == null && !isStopped && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(200)
            check(!isStopped) { NativeText.t(applicationContext, "Scheduled download cancelled") }
            MediaRuntime.error?.let { error(it) }
            check(MediaRuntime.ready) { NativeText.t(applicationContext, "The download engine did not become ready. Retry the schedule.") }
            val output = File(spec.getString("output")).canonicalFile
            check(output.isDirectory || output.mkdirs()) { NativeText.t(applicationContext, "The download folder is unavailable") }
            stage = File(output, ".omniget-schedule-$id").also { check(it.isDirectory || it.mkdirs()) { NativeText.t(applicationContext, "Cannot create a download workspace") } }
            val manifest = File(stage, "finished-path.txt")
            val args = mutableListOf("--windows-filenames", "--no-playlist", "--no-overwrites", "--no-mtime", "--print-to-file", "after_move:filepath", manifest.path,
                "-o", File(stage, "%(title).160B [%(id)s].%(ext)s").path)
            val mode = spec.optString("mode")
            if (mode == "mp3" || mode == "audio") args.addAll(listOf("-x", "--audio-format", if (mode == "mp3") "mp3" else "m4a"))
            else {
                val height = spec.optString("quality").removeSuffix("p").toIntOrNull()
                val filter = height?.let { "[height<=$it]" } ?: ""
                val format = spec.optString("formatId").ifEmpty { if (mode == "mute") "bestvideo$filter" else "bestvideo$filter+bestaudio/best$filter" }
                args.addAll(listOf("-f", format, "--merge-output-format", "mp4"))
            }
            spec.optString("referer").takeIf { it.isNotBlank() }?.let { args.addAll(listOf("--referer", it)) }
            val subtitles = spec.optString("subtitleTrack")
            if (subtitles.isNotEmpty()) {
                val (kind, lang) = subtitles.split(':', limit = 2)
                require(lang.matches(Regex("[A-Za-z0-9_-]{1,40}"))) { NativeText.t(applicationContext, "Invalid subtitle language") }
                args.addAll(listOf("--write-subs", if (kind == "auto") "--write-auto-subs" else "--no-write-auto-subs", "--sub-langs", lang, "--convert-subs", "srt"))
            } else args.addAll(listOf("--no-write-subs", "--no-write-auto-subs"))
            if (spec.optBoolean("wifiOnly")) {
                networkCallback = object : ConnectivityManager.NetworkCallback() {
                    private fun checkWifi() {
                        if (!onWifi()) { wifiLost.set(true); YoutubeDL.getInstance().destroyProcessById(processId) }
                    }
                    override fun onCapabilitiesChanged(network: android.net.Network, caps: NetworkCapabilities) { checkWifi() }
                    override fun onLost(network: android.net.Network) { checkWifi() }
                }.also { connectivity.registerDefaultNetworkCallback(it) }
                if (!onWifi()) return Result.retry()
            }
            var lastProgress = -2
            check(!isStopped) { NativeText.t(applicationContext, "Scheduled download cancelled") }
            YoutubeDL.getInstance().execute(YoutubeDLRequest(spec.getString("url")).addCommands(args), processId, false) { value, _, _ ->
                val percent = value.toInt().coerceIn(0, 100)
                if (percent != lastProgress) {
                    lastProgress = percent
                    setProgressAsync(Data.Builder().putInt("percent", percent).build())
                    if (percent == 100 || percent % 2 == 0) setForegroundAsync(foreground(percent))
                }
            }
            check(!isStopped) { NativeText.t(applicationContext, "Scheduled download cancelled") }
            val source = File(manifest.readLines().lastOrNull { it.isNotBlank() } ?: error(NativeText.t(applicationContext, "No completed file was reported"))).canonicalFile
            require(source.parentFile == stage.canonicalFile && source.isFile && source.length() > 0) { NativeText.t(applicationContext, "The completed file is missing or invalid") }
            var target = File(output, source.name)
            var suffix = 1
            while (target.exists()) { target = File(output, "${source.nameWithoutExtension} (${suffix++}).${source.extension}") }
            // Reserve the final name atomically so concurrent workers cannot overwrite it.
            while (!target.createNewFile()) { target = File(output, "${source.nameWithoutExtension} (${suffix++}).${source.extension}") }
            if (!source.renameTo(target)) { target.delete(); error(NativeText.t(applicationContext, "Could not move the completed download")) }
            saved = target
            // Preserve source-provided subtitle sidecars with the final media name.
            stage.listFiles()?.filter { it.isFile && it.name.startsWith(source.nameWithoutExtension + ".") && it.extension in listOf("srt", "vtt", "ass") }?.forEach {
                val tail = it.name.removePrefix(source.nameWithoutExtension)
                check(it.renameTo(File(output, target.nameWithoutExtension + tail))) { NativeText.t(applicationContext, "Could not save subtitle sidecar") }
            }
            MediaLibrary.register(applicationContext, target.path)
            try { MediaActions.complete(applicationContext, target.path, target.nameWithoutExtension) } catch (e: Exception) { android.util.Log.w("libreget", "Completion notification unavailable", e) }
            stage.deleteRecursively()
            ScheduledDownloads.record(applicationContext, id.toString(), "SUCCEEDED", target.path)
            Result.success(Data.Builder().putString("path", target.path).build())
        } catch (e: Exception) {
            android.util.Log.e("libreget", NativeText.t(applicationContext, "Scheduled download failed"), e)
            val message = (e.message ?: NativeText.t(applicationContext, "Scheduled download failed")).take(1500)
            if (wifiLost.get() && !isStopped) Result.retry()
            else {
                ScheduledDownloads.record(applicationContext, id.toString(), if (isStopped) "CANCELLED" else "FAILED", saved?.path ?: "", message)
                Result.failure(Data.Builder().putString("error", message).putString("path", saved?.path ?: "").build())
            }
        } finally {
            networkCallback?.let { try { connectivity.unregisterNetworkCallback(it) } catch (_: Exception) {} }
            if (isStopped) { YoutubeDL.getInstance().destroyProcessById(processId); stage?.deleteRecursively() }
        }
    }
}
