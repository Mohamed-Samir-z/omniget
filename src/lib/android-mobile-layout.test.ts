import { describe, expect, it } from "vitest";
import fs from "node:fs";
const read = (path: string) => fs.readFileSync(path, "utf8");
describe("Android mobile layout repair", () => {
  it("removes the developer card only from Downloads", () => {
    expect(read("src/routes/downloads/+page.svelte")).not.toContain("DeveloperCard");
    expect(read("src/routes/+page.svelte")).toContain("DeveloperCard");
    expect(read("src/routes/settings/+page.svelte")).toContain("DeveloperCard");
  });
  it("keeps Android timing limitations available without an always-open paragraph", () => {
    const source=read("src/components/downloads/ScheduledDownloads.svelte");
    expect(source).toContain('<details class="schedule-details">');
    expect(source).toContain("downloads.schedule_system_note");
    expect(source).toContain("shown.length > 0");
    expect(source).not.toContain("downloads.schedule_empty");
    expect(source).toContain("run-now");
    expect(source).toContain("reschedule");
    const ar=JSON.parse(read("src/lib/i18n/ar.json"));
    expect(ar.downloads.schedule_refresh).toBe("تحديث");
    expect(ar.downloads.schedule_help).toBe("كيف تعمل الجدولة؟");
  });
  it("keeps raw technical lines inside the expandable log on Android", () => {
    const source=read("src/components/download/DownloadLog.svelte");
    expect(source).toContain("$derived(!isAndroid() && !expanded");
    expect(source).toContain('if expanded || status === "error"');
    expect(source).toContain('aria-expanded={expanded}');
  });
  it("allows filter wrapping and gives bottom links non-overlapping widths", () => {
    expect(read("src/routes/downloads/+page.svelte")).toContain('.filter-pills{display:flex;flex-wrap:wrap;');
    expect(read("src/lib/style/mobile.css")).toContain("flex: 0 0 76px");
  });
  it("preserves clipping and stops unsupported Android scheduling before invocation", () => {
    const source=read("src/components/home/DownloadTiming.svelte");
    expect(source).toContain('bind:value={clipStart}');
    expect(source).toContain('bind:value={clipEnd}');
    expect(source).toContain('@container(max-width:330px)');
    expect(source).toContain("timing.android_hint");
    expect(read("src/routes/+page.svelte")).toContain("if (isAndroid() && scheduleAt && (clipStart.trim() || clipEnd.trim() || scheduleStop))");
  });
  it("posts the prayer separately after normal completion without replacing actions", () => {
    const source=read(".github/android/MediaActions.kt");
    expect(source.indexOf('manager.notify(id,')).toBeLessThan(source.indexOf('manager.notify("developer-prayer", id,'));
    expect(source).toContain('NativeText.t(context, "Open"), open');
    expect(source).toContain('NativeText.t(context, "Share"), share');
    expect(source).not.toContain('title.take(160) +');
    expect(read("src/components/DeveloperCard.svelte")).not.toContain("دعوة");
  });
});
