import { describe, expect, it } from "vitest";
import { swipeDestination } from "./section-swipe";
const gesture = { dx: -120, dy: 5, elapsed: 250, rtl: false };
describe("four-section Android swipes", () => {
  it("moves left to next in English, right to next in Arabic", () => {
    expect(swipeDestination("/", gesture)).toBe("/downloads");
    expect(swipeDestination("/", { ...gesture, dx: 120, rtl: true })).toBe("/downloads");
    expect(swipeDestination("/settings", gesture)).toBe("/about");
    expect(swipeDestination("/about", { ...gesture, dx: 120 })).toBe("/settings");
    expect(swipeDestination("/downloads", { ...gesture, dx: -120, rtl: true })).toBe("/");
  });
  it("has no wrap and never adds hidden sections", () => {
    expect(swipeDestination("/about", gesture)).toBeNull();
    expect(swipeDestination("/", { ...gesture, dx: 120 })).toBeNull();
    expect(swipeDestination("/tools", gesture)).toBeNull();
    expect(swipeDestination("/marketplace", gesture)).toBeNull();
    expect(swipeDestination("/settings/downloads", gesture)).toBe("/about");
  });
  it("ignores short, vertical, diagonal and long gestures", () => {
    for (const patch of [{ dx: 40 }, { dy: 180 }, { dy: 100 }, { elapsed: 1100 }]) {
      expect(swipeDestination("/", { ...gesture, ...patch })).toBeNull();
    }
  });
});
