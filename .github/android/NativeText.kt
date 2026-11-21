package @PACKAGE@

import android.content.Context
import android.view.View

object NativeText {
    fun arabic(context: Context): Boolean = context.getSharedPreferences("omniget.ui", 0).getString("language", null)?.let { it.startsWith("ar") } ?: (java.util.Locale.getDefault().language == "ar")
    fun language(context: Context, value: String) { context.getSharedPreferences("omniget.ui", 0).edit().putString("language", value).apply() }
    fun direction(context: Context): Int = if (arabic(context)) View.LAYOUT_DIRECTION_RTL else View.LAYOUT_DIRECTION_LTR
    fun t(context: Context, text: String): String = if (arabic(context)) arabic[text] ?: text else text
    fun percent(context: Context, value: Int): String = if (arabic(context)) "تم تنزيل $value٪" else "$value% downloaded"
    fun count(context: Context, value: Int): String = if (arabic(context)) "omniget · $value تنزيلات" else "omniget · $value downloads"
    private val arabic = mapOf(
        "Finalizing and adding to media library" to "جارٍ إنهاء التنزيل وإضافته إلى مكتبة الوسائط",
        "Download shared link" to "تنزيل الرابط المشترك",
        "Type" to "النوع",
        "Video" to "فيديو",
        "Video quality" to "جودة الفيديو",
        "Best available" to "أفضل جودة متاحة",
        "Reset choices" to "إعادة ضبط الاختيارات",
        "Download" to "تنزيل",
        "More options in the app" to "خيارات إضافية داخل التطبيق",
        "Not now" to "ليس الآن",
        "Preparing..." to "جارٍ التجهيز...",
        "The app is still starting. Try again or open omniget first." to "التطبيق لا يزال قيد التشغيل. أعد المحاولة أو افتح omniget أولًا.",
        "Could not prepare this download. Open omniget for more options." to "تعذر تجهيز التنزيل. افتح omniget لعرض الخيارات.",
        "Download added" to "أُضيف التنزيل",
        "You can keep using the app underneath. Progress appears in the download notification." to "يمكنك متابعة استخدام التطبيق الآخر. يظهر تقدم التنزيل في الإشعار.",
        "Open omniget" to "فتح omniget",
        "The downloaded file is missing or empty" to "الملف المنزّل مفقود أو فارغ",
        "Share downloaded file" to "مشاركة الملف المنزّل",
        "Completed downloads" to "التنزيلات المكتملة",
        "Download complete" to "اكتمل التنزيل",
        "Developer appreciation" to "دعوة للمطور",
        "Open" to "فتح",
        "Share" to "مشاركة",
        "Scheduled downloads" to "التنزيلات المجدولة",
        "omniget · Scheduled download" to "omniget · تنزيل مجدول",
        "Preparing download" to "جارٍ تجهيز التنزيل",
        "Cancel" to "إلغاء",
        "Retry" to "إعادة المحاولة",
        "The app is taking longer to start. You can retry without removing your downloads." to "يستغرق فتح التطبيق وقتًا أطول من المعتاد. يمكنك إعادة المحاولة دون حذف تنزيلاتك.",
        "omniget is starting" to "جارٍ فتح omniget",
        "Could not prepare the download engine. Open omniget and check the download log." to "تعذر تجهيز محرك التنزيل. افتح omniget وراجع سجل التنزيل.",
        "A valid HTTP or HTTPS link is required" to "أدخل رابط HTTP أو HTTPS صالحًا",
        "Choose a future download time" to "اختر موعدًا في المستقبل",
        "Scheduled download options are too large" to "خيارات التنزيل المجدول أكبر من المسموح",
        "Could not save the schedule" to "تعذر حفظ الموعد",
        "Could not remove the schedule" to "تعذر حذف الموعد",
        "Cancel this scheduled download before removing it" to "ألغِ التنزيل المجدول قبل حذفه",
        "The download folder is unavailable" to "مجلد التنزيل غير متاح",
        "Cannot create a download workspace" to "تعذر إنشاء مساحة عمل للتنزيل",
        "The download engine did not become ready. Retry the schedule." to "محرك التنزيل لم يجهز بعد. أعد جدولة التنزيل.",
        "Scheduled download cancelled" to "أُلغي التنزيل المجدول",
        "Invalid subtitle language" to "لغة الترجمة غير صالحة",
        "No completed file was reported" to "لم يُعثر على ملف مكتمل",
        "The completed file is missing or invalid" to "الملف المكتمل مفقود أو غير صالح",
        "Could not move the completed download" to "تعذر نقل التنزيل المكتمل",
        "Could not save subtitle sidecar" to "تعذر حفظ ملف الترجمة",
        "Scheduled download failed" to "فشل التنزيل المجدول",
        "Schedule not found" to "الموعد غير موجود",
        "This download has already started or finished. Refresh the list." to "بدأ هذا التنزيل أو اكتمل بالفعل. حدّث القائمة.",
        "Downloads" to "التنزيلات"
    )
}
