//! Android download policy. Fail closed when Wi-Fi cannot be verified.

pub fn downloads_allowed(app: &tauri::AppHandle) -> bool {
    #[cfg(target_os = "android")]
    {
        !crate::storage::config::load_settings(app)
            .download
            .wifi_only
            || wifi_connected()
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        true
    }
}

#[cfg(target_os = "android")]
fn wifi_connected() -> bool {
    use jni::objects::{JObject, JValue};
    fn check() -> jni::errors::Result<bool> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let context = unsafe { JObject::from_raw(ctx.context().cast()) };
        let result = (|| {
            let name = env.new_string("connectivity")?;
            let manager = env
                .call_method(
                    &context,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[JValue::Object(&name)],
                )?
                .l()?;
            let network = env
                .call_method(&manager, "getActiveNetwork", "()Landroid/net/Network;", &[])?
                .l()?;
            if network.is_null() {
                return Ok(false);
            }
            let caps = env
                .call_method(
                    &manager,
                    "getNetworkCapabilities",
                    "(Landroid/net/Network;)Landroid/net/NetworkCapabilities;",
                    &[JValue::Object(&network)],
                )?
                .l()?;
            if caps.is_null() {
                return Ok(false);
            }
            // TRANSPORT_WIFI = 1. Unlike NOT_METERED, this never accepts
            // unmetered cellular data. Android VPNs normally retain their
            // underlying transports; an unverifiable VPN fails closed.
            env.call_method(&caps, "hasTransport", "(I)Z", &[JValue::Int(1)])?
                .z()
        })();
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result
    }
    check().unwrap_or(false)
}
