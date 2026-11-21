import { describe, expect, it } from "vitest";
import { diagnosticReport } from "./diagnostic-report";
describe("private diagnostic summaries", () => {
 it("never includes raw secrets, paths, titles, headers or links", () => {
  const secrets=["secret-cookie=ABCDE", "Bearer SECRET", "https://private.example/a?token=BAD", "/storage/emulated/0/private.mp4", "Mohamed private title", "an arbitrary password"];
  const report=diagnosticReport({version:"0.16.6",status:"error",platform:"Android",lines:[...secrets,"Operation not permitted (os error 1)","Normalizing clip timestamps"]});
  for(const secret of secrets) expect(report).not.toContain(secret);
  expect(report).toContain("filesystem permission error");expect(report).toContain("clip normalization");
 });
 it("does not trust injected metadata or unknown log text", () => {
  const report=diagnosticReport({version:"leak\nBearer secret",status:"token=PRIVATE",platform:"secret-device-id",lines:["do something with my password"]});
  expect(report).not.toContain("Bearer");expect(report).not.toContain("PRIVATE");expect(report).not.toContain("device-id");expect(report).toContain("unclassified");
 });
});
