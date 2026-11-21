export type DiagnosticInput = { version: string; build?: string; status: string; lines: string[]; platform: string };
/** Allowlisted summary, not a partially redacted raw log. Unknown text never leaves. */
export function diagnosticReport(input: DiagnosticInput): string {
  const text = input.lines.join("\n");
  const stages: [RegExp, string][] = [
    [/normaliz|timestamp/i, "clip normalization"], [/merg|ffmpeg|postprocess/i, "media processing"],
    [/\[download\]|download:/i, "transfer"], [/extract|fetch.*info/i, "source lookup"],
    [/runtime|engine|initializ/i, "engine startup"],
  ];
  const signals: [RegExp, string][] = [
    [/HTTP[^\n]{0,30}\b403\b|Forbidden/i, "HTTP 403: source denied access"],
    [/HTTP[^\n]{0,30}\b404\b|not found/i, "HTTP 404: source unavailable"],
    [/HTTP[^\n]{0,30}\b429\b|too many requests/i, "HTTP 429: rate limited"],
    [/Operation not permitted|permission denied|EACCES|EPERM/i, "filesystem permission error"],
    [/no space|ENOSPC/i, "storage full"], [/timed? ?out|timeout/i, "network or engine timeout"],
    [/sign.?in|cookies|authentication/i, "source authentication may be required"],
    [/duration|shortfall|truncat/i, "media duration/validation signal"],
    [/checksum|corrupt/i, "integrity check signal"], [/wifi|Wi-Fi/i, "Wi-Fi policy signal"],
  ];
  const safeVersion = /^[\w.+-]{1,60}$/.test(input.version) ? input.version : "unknown";
  const status = ["queued", "downloading", "paused", "error", "complete"].includes(input.status) ? input.status : "unknown";
  const platform = ["Android", "desktop"].includes(input.platform) ? input.platform : "unknown";
  return ["omniget diagnostic summary", `App version: ${safeVersion}`, `Build: ${/^[a-f0-9]{7,40}$/i.test(input.build || "") ? input.build : "unknown"}`, `Platform: ${platform}`, `Job state: ${status}`,
    `Observed stages: ${stages.filter(([p]) => p.test(text)).map(([,v]) => v).join(", ") || "unknown"}`,
    `Signals: ${signals.filter(([p]) => p.test(text)).map(([,v]) => v).join("; ") || "unclassified, no raw text included"}`,
    `Log lines examined: ${input.lines.length}`,
    "Privacy: URLs, titles, paths, account details, cookies, tokens, request headers and raw log text are not included.",
    "This summary is heuristic. Review it before sharing; nothing is sent automatically.",
  ].join("\n");
}
