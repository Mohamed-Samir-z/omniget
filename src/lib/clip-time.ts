export function timeSeconds(value: string): number | null {
  const parts = value.trim().split(":");
  if (!value.trim() || parts.length > 3) return null;
  let total = 0;
  for (const [i, part] of parts.entries()) {
    if (!/^\d+(\.\d{1,3})?$/.test(part) || (i < parts.length - 1 && part.includes("."))) return null;
    const n = Number(part);
    if (!Number.isFinite(n) || (i > 0 && n >= 60)) return null;
    total = total * 60 + n;
  }
  return total <= 31536000 ? total : null;
}

export function validTimeRange(start: string, end: string): boolean {
  const a = start.trim() ? timeSeconds(start) : 0;
  const b = end.trim() ? timeSeconds(end) : null;
  return a !== null && (!end.trim() || (b !== null && b > a));
}

export function splitTime(value: string): string[] {
  if (!value) return ["", "", ""];
  const parts = value.split(":");
  return [...Array(Math.max(0, 3 - parts.length)).fill(""), ...parts].slice(-3);
}

export function joinTime(parts: string[]): string {
  return parts.every((v) => !v.trim()) ? "" : parts.map((v) => v || "00").join(":");
}
