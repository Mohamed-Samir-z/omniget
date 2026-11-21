package com.openselena.libreget
class MainActivity : android.app.Activity() {
 var fullAppSession = false
 fun hasFullAppSession(): Boolean = fullAppSession
 var contentVisible = true
 var left = false
 @Volatile var ready = true
 @Volatile var calls = 0
 @Volatile var last = listOf<String>()
 @Volatile var result = ""
 fun setShareContentVisible(visible: Boolean) { contentVisible = visible }
 fun nativeShareReady(): Boolean = ready
 fun nativeShareAction(text: String, mode: String, quality: String, action: String): String {
  last=listOf(text,mode,quality,action); calls++; return result
 }
 override fun moveTaskToBack(nonRoot: Boolean): Boolean { left=true; return true }
}

object MediaRuntime { @Volatile var ready = true; @Volatile var error: String? = null }
