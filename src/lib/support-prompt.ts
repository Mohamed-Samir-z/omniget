export const SUPPORT_KEY = "omniget.support.v1";
export const SUPPORT_COOLDOWN = 4 * 24 * 60 * 60 * 1000;
export const SUPPORT_QUIET_TIME = 15000;
export type SupportState = { count: number; ids: string[]; nextPromptAt: number; lastCompletedAt: number; supported: boolean };
type Store = Pick<Storage, "getItem" | "setItem">;
const empty = (): SupportState => ({ count: 0, ids: [], nextPromptAt: 0, lastCompletedAt: 0, supported: false });
let fallback = empty();
const failedStates = new WeakMap<Store, SupportState>();
function browserStorage(): Store | undefined { try { return typeof localStorage === "undefined" ? undefined : localStorage; } catch { return undefined; } }
export function readSupportState(storage = browserStorage()): SupportState {
  if (!storage) return { ...fallback, ids: [...fallback.ids] };
  const session = failedStates.get(storage);
  if (session) return { ...session, ids: [...session.ids] };
  try {
    const x = JSON.parse(storage.getItem(SUPPORT_KEY) || "null");
    if (!x || typeof x !== "object") return empty();
    return { count: Number.isFinite(x.count) ? Math.max(0, Math.min(3, x.count)) : 0, ids: Array.isArray(x.ids) ? x.ids.filter((v: unknown) => typeof v === "string").slice(-200) : [], nextPromptAt: Number.isFinite(x.nextPromptAt) ? Math.max(0, x.nextPromptAt) : 0, lastCompletedAt: Number.isFinite(x.lastCompletedAt) ? Math.max(0, x.lastCompletedAt) : 0, supported: x.supported === true };
  } catch { const state=empty(); failedStates.set(storage,state); return state; }
}
function save(state: SupportState, storage?: Store) { fallback = state; try { storage?.setItem(SUPPORT_KEY, JSON.stringify(state)); } catch { if(storage) failedStates.set(storage,state); } return state; }
export function recordVerifiedSupportDownload(id: number, now = Date.now(), storage = browserStorage()): SupportState {
  const state = readSupportState(storage);
  const key = String(id);
  if (state.ids.includes(key)) return state;
  state.count = Math.min(3, state.count + 1); state.ids = [...state.ids, key].slice(-200); state.lastCompletedAt = now;
  return save(state, storage);
}
export function canOfferSupport(now = Date.now(), storage = browserStorage()): boolean {
  const state = readSupportState(storage);
  return !state.supported && state.count >= 3 && now >= state.nextPromptAt && now - state.lastCompletedAt >= SUPPORT_QUIET_TIME;
}
/** Showing, dismissing or opening GitHub is not proof of support. */
export function deferSupport(now = Date.now(), storage = browserStorage()): SupportState {
  const state = readSupportState(storage); state.nextPromptAt = now + SUPPORT_COOLDOWN; return save(state, storage);
}
/** This is the user's voluntary self-report, not a verified GitHub star. */
export function acknowledgeSupport(storage = browserStorage()): SupportState {
  const state = readSupportState(storage); state.supported = true; return save(state, storage);
}
