export type DownloadSelection = {
  audio_requested: string[];
  subtitles_enabled: boolean | null;
  subtitle_requested: string | null;
  automatic_subtitles: boolean;
  subtitles_obtained: string[];
};

type RecentItem = { id: number; completedAtMs?: number | null; startedAtMs?: number | null; startedAt?: number };
/** Receipt/update time is deliberately excluded: a queue snapshot refreshes all
 * cards together. Terminal time comes from the backend and survives restarts. */
export function newestDownloadFirst(a: RecentItem, b: RecentItem): number {
  const time = (item: RecentItem) => item.completedAtMs ?? item.startedAtMs ?? item.startedAt ?? 0;
  return time(b) - time(a) || b.id - a.id;
}

export function subtitleWarning(code?: string | null): boolean {
  return code === "subtitles_missing";
}
