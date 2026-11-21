import { invoke } from "@tauri-apps/api/core";
import { storeChangelogForUpdate } from "$lib/stores/changelog-store.svelte";
import { getSettings } from "$lib/stores/settings-store.svelte";

// The updater and process plugins are not linked into the Android build, so
// they are imported lazily and never touched there. Android instead checks
// the android-latest GitHub prerelease through the commands in
// src-tauri/src/android_update.rs and installs the APK via the system
// package installer.
const isAndroid =
  typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);

export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
  error?: string;
  reason?: string;
}

function proxyUrl(): string | undefined {
  const proxy = getSettings()?.proxy;
  if (!proxy?.enabled || !proxy.host || !proxy.port) return undefined;
  const scheme =
    proxy.proxy_type === "socks5" || proxy.proxy_type === "https"
      ? proxy.proxy_type
      : "http";
  const auth = proxy.username
    ? `${encodeURIComponent(proxy.username)}:${encodeURIComponent(proxy.password ?? "")}@`
    : "";
  return `${scheme}://${auth}${proxy.host}:${proxy.port}`;
}

interface AndroidUpdateInfo {
  available: boolean;
  reason?: string | null;
  title?: string | null;
  publishedAt?: string | null;
  downloadUrl?: string | null;
  sizeBytes?: number | null;
  sha256?: string | null;
}

// The last Android check, kept so installUpdate can reuse its download URL
// and checksum without a second network request.
let androidUpdate: AndroidUpdateInfo | null = null;

async function checkAndroidUpdate(): Promise<UpdateInfo> {
  try {
    const info = await invoke<AndroidUpdateInfo>("check_android_update");
    androidUpdate = info.available ? info : null;
    return {
      available: info.available,
      version: info.title?.replace(/^Android build\s*/, "") || undefined,
      body: info.publishedAt ?? undefined,
      reason: info.reason ?? undefined,
    };
  } catch (e) {
    androidUpdate = null;
    return { available: false, error: String(e) };
  }
}

export async function checkForUpdate(): Promise<UpdateInfo> {
  if (isAndroid) return checkAndroidUpdate();
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check({ proxy: proxyUrl() });
    if (update) {
      return {
        available: true,
        version: update.version,
        body: update.body ?? undefined,
      };
    }
    return { available: false };
  } catch (e) {
    return { available: false, error: String(e) };
  }
}

export async function installUpdate(): Promise<void> {
  if (isAndroid) {
    const info = androidUpdate;
    if (!info?.downloadUrl) throw new Error("Check for updates again before installing");
    const path = await invoke<string>("download_android_update", {
      url: info.downloadUrl,
      sha256: info.sha256 ?? null,
    });
    await invoke("install_android_update", { path });
    return;
  }
  const { check } = await import("@tauri-apps/plugin-updater");
  const { relaunch } = await import("@tauri-apps/plugin-process");
  const update = await check({ proxy: proxyUrl() });
  if (update) {
    if (update.body && update.version) {
      storeChangelogForUpdate(update.body, update.version);
    }
    await update.downloadAndInstall();
    await relaunch();
  }
}
