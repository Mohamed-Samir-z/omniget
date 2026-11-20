## Saved working build (commit 35192fa)

Tested by the owner on a Realme C61 (Android 16). This release is permanent; it is not overwritten by new CI builds (`android-latest` is).

### Works
- **Facebook**: short and long videos (Web/generic yt-dlp path).
- **Instagram**: many posts and reels, saved to `Download/OmniGet`, indexed for the gallery. Some posts fail with "Login required".
- yt-dlp 2025.11.12, Python, QuickJS and ffmpeg 7.1.1 run inside the app.
- Stable signing key: later builds install over this one without uninstalling.
- Foreground download service only while downloading.

### Known not working in this build
- **YouTube**: fails with `Impersonate target "chrome" is not available` (curl_cffi is not bundled).
- **TikTok**: blocked / same impersonation problem.
- Other yt-dlp sites that rely on impersonation.
- Quality picker, clip, audio-track features are enabled but untested.

### Arabic / عربي
نسخة محفوظة وشغالة (commit 35192fa): فيسبوك (طويل وقصير) شغال، إنستا شغال في أغلب الفيديوهات (بعضها يطلب تسجيل دخول). يوتيوب وتيك توك لسه مش شغالين في النسخة دي (سبب يوتيوب: خيار impersonate).
