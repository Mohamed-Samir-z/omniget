import { describe, expect, it, vi } from "vitest";
import fs from "node:fs";
import { invoke } from "@tauri-apps/api/core";
import { openDeveloperContact } from "./developer-contact";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
describe("developer contact browser handoff", () => {
  it("calls only the fixed native command without a URL, path or program", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await openDeveloperContact();
    expect(invoke).toHaveBeenLastCalledWith("open_developer_contact");
  });
  it("returns native errors to the card instead of reporting success", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("No browser available"));
    await expect(openDeveloperContact()).rejects.toThrow("No browser available");
  });
  it("registers the fixed command and does not widen capability permissions", () => {
    const source = fs.readFileSync("src-tauri/src/commands/downloads.rs", "utf8");
    const command = source.split("pub async fn open_developer_contact")[1].split("#[tauri::command]")[0];
    expect(command).toContain("(app: tauri::AppHandle)");
    expect(command).toContain("use tauri_plugin_shell::ShellExt");
    expect(command.replace(/\s+/g, "")).toContain('.open("https://mohamed-samir-profile.mo2868417.workers.dev/?v=m",None,)');
    const lib = fs.readFileSync("src-tauri/src/lib.rs", "utf8");
    expect(lib).toContain("commands::downloads::open_developer_contact,");
    expect(lib).toContain(".plugin(tauri_plugin_shell::init())");
    expect(fs.readFileSync("src/components/DeveloperCard.svelte", "utf8")).not.toContain("plugin-opener");
    const caps = JSON.parse(fs.readFileSync("src-tauri/capabilities/default.json", "utf8"));
    expect(caps.permissions).not.toContain("opener:default");
    expect(caps.permissions).not.toContain("shell:allow-execute");
    expect(caps.permissions).not.toContain("shell:allow-spawn");
  });
});
