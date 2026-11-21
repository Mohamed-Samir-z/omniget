import {describe,it,expect} from "vitest";
import {androidReadyNav,ANDROID_READY_ROUTES} from "./android-nav";
import type {NavItem} from "./nav-config";
describe("Android ready navigation",()=>{
 it("keeps exactly the four approved routes without mutating hidden work",()=>{
 const paths=["/","/downloads","/tools","/nami","/settings","/marketplace","/about"];
 const items=paths.map(href=>({href}) as NavItem);
 expect(androidReadyNav(items).map(i=>i.href)).toEqual([...ANDROID_READY_ROUTES]);
 expect(items.map(i=>i.href)).toEqual(paths);
 });
});
