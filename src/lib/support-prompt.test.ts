import { describe, expect, it } from "vitest";
import { SUPPORT_COOLDOWN, SUPPORT_QUIET_TIME, acknowledgeSupport, canOfferSupport, deferSupport, readSupportState, recordVerifiedSupportDownload } from "./support-prompt";
function storage() { const data = new Map<string, string>(); return { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => { data.set(k, v); } }; }
describe("voluntary support prompt", () => {
  it("needs three distinct verified completions, then a quiet delay", () => {
    const s=storage();expect(canOfferSupport(100000,s)).toBe(false);
    recordVerifiedSupportDownload(1,1000,s);recordVerifiedSupportDownload(1,1000,s);recordVerifiedSupportDownload(2,1000,s);
    expect(readSupportState(s).count).toBe(2);expect(canOfferSupport(100000,s)).toBe(false);
    recordVerifiedSupportDownload(3,1000,s);expect(canOfferSupport(1000+SUPPORT_QUIET_TIME-1,s)).toBe(false);expect(canOfferSupport(1000+SUPPORT_QUIET_TIME,s)).toBe(true);
  });
  it("later/showing or visiting GitHub never claims a star and waits four days", () => {
    const s=storage();for(let i=0;i<3;i++)recordVerifiedSupportDownload(i,0,s);
    deferSupport(100000,s);expect(readSupportState(s).supported).toBe(false);
    expect(canOfferSupport(100000+SUPPORT_COOLDOWN-1,s)).toBe(false);expect(canOfferSupport(100000+SUPPORT_COOLDOWN,s)).toBe(true);
  });
  it("already-supported suppresses permanently as a local self-report", () => {
    const s=storage();for(let i=0;i<3;i++)recordVerifiedSupportDownload(i,0,s);acknowledgeSupport(s);
    recordVerifiedSupportDownload(10,1,s);expect(canOfferSupport(9999999999999,s)).toBe(false);expect(readSupportState(s).supported).toBe(true);
  });
  it("keeps this session usable when storage writes are blocked", () => {
    const s={getItem:()=>null,setItem:()=>{throw new Error("blocked")}};
    for(let i=0;i<3;i++)recordVerifiedSupportDownload(i,0,s);
    expect(canOfferSupport(100000,s)).toBe(true);acknowledgeSupport(s);expect(canOfferSupport(100000,s)).toBe(false);
  });
  it("invalid state is not treated as proof of successful downloads", () => {
    const s=storage();s.setItem("omniget.support.v1","not json");expect(readSupportState(s).count).toBe(0);
  });
});
