package @PACKAGE@

/**
 * Hands a downloaded APK to the system package installer.
 *
 * Rust's `install_android_update` command calls `install` over JNI with the
 * application context and the APK's path in the cache directory. The
 * FileProvider (declared by patch_project.py with res/xml/file_paths.xml)
 * grants the installer one-shot read access to that private file.
 */
object ApkInstaller {
  @JvmStatic
  fun install(context: android.content.Context, path: String) {
    val file = java.io.File(path)
    val uri = androidx.core.content.FileProvider.getUriForFile(
      context,
      context.packageName + ".fileprovider",
      file,
    )
    val intent = android.content.Intent(android.content.Intent.ACTION_VIEW)
    intent.setDataAndType(uri, "application/vnd.android.package-archive")
    intent.addFlags(
      android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION
        or android.content.Intent.FLAG_ACTIVITY_NEW_TASK,
    )
    context.startActivity(intent)
  }
}
