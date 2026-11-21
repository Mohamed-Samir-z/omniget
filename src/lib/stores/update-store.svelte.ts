import { checkForUpdate, installUpdate, type UpdateInfo } from "$lib/updater";
import { showToast } from "./toast-store.svelte";

let updateInfo: UpdateInfo = $state({ available: false });
let updateDismissed: boolean = $state(false);
let isUpdating: boolean = $state(false);

export function getUpdateInfo(): UpdateInfo {
  return updateInfo;
}

export function isUpdateDismissed(): boolean {
  return updateDismissed;
}

export function dismissUpdate(): void {
  updateDismissed = true;
}

export function resetUpdateDismissed(): void {
  updateDismissed = false;
}

export function getIsUpdating(): boolean {
  return isUpdating;
}

export function setUpdateInfoForTest(info: UpdateInfo): void {
  updateInfo = info;
}

export async function refreshUpdateInfo(): Promise<void> {
  updateInfo = await checkForUpdate();
}

export async function performUpdate(): Promise<void> {
  if (isUpdating) return;
  isUpdating = true;
  try {
    await installUpdate();
  } catch (error) {
    showToast("error", String(error));
    throw error;
  } finally {
    isUpdating = false;
  }
}
