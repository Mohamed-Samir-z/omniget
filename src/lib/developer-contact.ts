import { invoke } from "@tauri-apps/api/core";

// The native command owns the fixed contact URL and uses the mobile browser
// intent. Do not replace with plugin-opener, which is not linked into this app.
export function openDeveloperContact(): Promise<void> {
  return invoke<void>("open_developer_contact");
}

export type DeveloperProfileLink = "whatsapp" | "portfolio" | "github" | "project";

export function openDeveloperProfileLink(target: DeveloperProfileLink): Promise<void> {
  return invoke<void>("open_developer_profile_link", { target });
}
