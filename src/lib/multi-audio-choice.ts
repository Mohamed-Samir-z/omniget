export type AudioChoice = {requested: string[]; available: string[]; unavailable: string[]; unknown: string[]};
export function audioChoice(error: string | null | undefined): AudioChoice | null {
 if (!error?.startsWith("MULTI_AUDIO_CHOICE:")) return null;
 try {
  const value=JSON.parse(error.slice("MULTI_AUDIO_CHOICE:".length));
  for(const key of ["requested","available","unavailable","unknown"]) {
   if(!Array.isArray(value[key]) || value[key].length>16 || value[key].some((v: unknown)=>typeof v!=="string" || !/^[a-zA-Z0-9_-]{1,16}$/.test(v))) return null;
  }
  if(value.requested.length===0 || ["available","unavailable","unknown"].some(k=>value[k].some((v:string)=>!value.requested.includes(v)))) return null;
  return value;
 } catch { return null; }
}
