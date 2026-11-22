import { describe, expect, it, vi } from "vitest";
import { createMediaDetailsLoader } from "./media-details";
describe("current-input full metadata sharing", () => {
  it("shares in-flight and successful exact-URL details between subtitles and formats", async () => {
    const result = { formats: ["all video and audio tracks"], subtitles: ["manual", "automatic"] };
    const fetcher = vi.fn(async () => result);
    const cache = createMediaDetailsLoader(fetcher);
    const formats = cache.load("video-a"), subtitles = cache.load("video-a");
    expect(formats).toBe(subtitles); expect(await formats).toBe(result);
    expect(await cache.load("video-a")).toBe(result); expect(fetcher).toHaveBeenCalledTimes(1);
    await cache.load("video-b"); expect(fetcher).toHaveBeenCalledTimes(2);
  });
  it("normalizes surrounding whitespace before sharing full details", async () => {
    const fetcher = vi.fn(async (url: string) => ({ url }));
    const cache = createMediaDetailsLoader(fetcher);
    const formats = cache.load("  video-a \n");
    const subtitles = cache.load("video-a");
    expect(formats).toBe(subtitles);
    expect(await subtitles).toEqual({ url: "video-a" });
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher).toHaveBeenCalledWith("video-a");
  });
  it("never treats a failed request as an empty subtitle list and permits retry", async () => {
    const fetcher = vi.fn().mockRejectedValueOnce(Error("offline")).mockResolvedValue({ tracks: [] });
    const cache = createMediaDetailsLoader(fetcher);
    await expect(cache.load("a")).rejects.toThrow("offline");
    await expect(cache.load("a")).resolves.toEqual({ tracks: [] });
    expect(fetcher).toHaveBeenCalledTimes(2);
  });
  it("input-session reset prevents old completion from becoming current cached metadata", async () => {
    let rejectOld: (e: Error) => void = () => {};
    const fetcher = vi.fn().mockImplementationOnce(() => new Promise((_, reject) => { rejectOld = reject; })).mockResolvedValue("current");
    const cache = createMediaDetailsLoader(fetcher);
    const old = cache.load("a"); await Promise.resolve();cache.reset();
    expect(await cache.load("a")).toBe("current");rejectOld(Error("late"));await expect(old).rejects.toThrow("late");
    expect(await cache.load("a")).toBe("current");expect(fetcher).toHaveBeenCalledTimes(2);
  });
});
