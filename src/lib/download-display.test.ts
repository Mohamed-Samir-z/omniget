import { describe, expect, it } from "vitest";
import { newestDownloadFirst } from "./download-display";

describe("download presentation recency", () => {
  it("uses actual completion time, not enqueue ID or snapshot receipt", () => {
    const rows = [
      { id: 3, completedAtMs: 100, startedAtMs: 5, lastUpdateAt: 900 },
      { id: 1, completedAtMs: 300, startedAtMs: 1, lastUpdateAt: 900 },
      { id: 2, completedAtMs: 200, startedAtMs: 3, lastUpdateAt: 900 },
    ];
    expect(rows.sort(newestDownloadFirst).map(row => row.id)).toEqual([1, 2, 3]);
    for (const row of rows) row.lastUpdateAt = 2000;
    expect(rows.sort(newestDownloadFirst).map(row => row.id)).toEqual([1, 2, 3]);
  });
  it("uses an ID tie-break for second-resolution old history", () => {
    expect([{ id: 1, completedAtMs: 1000 }, { id: 2, completedAtMs: 1000 }].sort(newestDownloadFirst).map(row => row.id)).toEqual([2, 1]);
  });
  it("preserves zero timestamps and falls back only when unknown", () => {
    const rows = [{ id: 1, completedAtMs: 0, startedAt: 500 }, { id: 2, startedAtMs: 10 }, { id: 3, startedAt: 20 }];
    expect(rows.sort(newestDownloadFirst).map(row => row.id)).toEqual([3, 2, 1]);
  });
});
