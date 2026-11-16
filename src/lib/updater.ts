import { storeChangelogForUpdate } from "$lib/stores/changelog-store.svelte";
import { getSettings } from "$lib/stores/settings-store.svelte";

// The updater and process plugins are not linked into the Android build, so
// they are imported lazily and never touched there. Android updates come from
// installing a newer APK.
const isAndroid =
  typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);

export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
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

export async function checkForUpdate(): Promise<UpdateInfo> {
  if (isAndroid) return { available: false };
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
  } catch {
    return { available: false };
  }
}

export async function installUpdate(): Promise<void> {
  if (isAndroid) return;
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
