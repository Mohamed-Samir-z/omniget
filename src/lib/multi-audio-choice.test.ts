import { describe, expect, it } from "vitest";
import { audioChoice } from "./multi-audio-choice";
describe("two-attempt audio choice",()=>{
 it("keeps named available, unavailable and uncertain languages separate",()=>{
  const choice={requested:["ar","en","fr"],available:["ar"],unavailable:["en"],unknown:["fr"]};
  expect(audioChoice("MULTI_AUDIO_CHOICE:"+JSON.stringify(choice))).toEqual(choice);
 });
 it("rejects unrelated strings and spoofed language payloads",()=>{
  for(const value of [null,"network error","MULTI_AUDIO_CHOICE:{}","MULTI_AUDIO_CHOICE:"+JSON.stringify({requested:["ar"],available:["en"],unavailable:[],unknown:[]}),"MULTI_AUDIO_CHOICE:"+JSON.stringify({requested:["<script>"],available:[],unavailable:[],unknown:[]})])expect(audioChoice(value)).toBeNull();
 });
});
