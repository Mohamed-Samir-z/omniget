import { ANDROID_READY_ROUTES } from "./android-nav";

type Gesture = { dx: number; dy: number; elapsed: number; rtl: boolean };
export function swipeDestination(path: string, { dx, dy, elapsed, rtl }: Gesture): string | null {
  if (Math.abs(dx) < 72 || Math.abs(dx) < Math.abs(dy) * 1.6 || elapsed > 900 || elapsed < 0) return null;
  const index = ANDROID_READY_ROUTES.findIndex(route => route === "/" ? path === "/" : path === route || path.startsWith(route + "/"));
  if (index < 0) return null;
  const next = index + ((rtl ? dx > 0 : dx < 0) ? 1 : -1);
  return ANDROID_READY_ROUTES[next] ?? null;
}

type Options = { enabled: boolean; getPath: () => string; rtl: () => boolean; navigate: (href: string) => unknown };
export function sectionSwipe(node: HTMLElement, initial: Options) {
  let options = initial;
  let start: { x: number; y: number; at: number } | null = null;
  const blocked = (target: EventTarget | null) => {
    if (!(target instanceof Element)) return true;
    if (target.closest('button,a,input,select,textarea,summary,video,audio,canvas,[role="slider"],[role="button"],[contenteditable="true"],[data-no-section-swipe]')) return true;
    for (let element: Element | null = target; element && element !== node; element = element.parentElement) {
      const style = getComputedStyle(element);
      if (/(auto|scroll)/.test(style.overflowX) && element.scrollWidth > element.clientWidth + 2) return true;
    }
    return false;
  };
  const reset = () => { start = null; };
  const begin = (event: TouchEvent) => {
    reset();
    if (!options.enabled || event.touches.length !== 1 || blocked(event.target)) return;
    const touch = event.touches[0];
    // Leave Android's edge/back gesture to the system.
    if (touch.clientX < 24 || touch.clientX > window.innerWidth - 24) return;
    start = { x: touch.clientX, y: touch.clientY, at: performance.now() };
  };
  const move = (event: TouchEvent) => {
    if (event.touches.length !== 1) { reset(); return; }
    if (!start) return;
    const dx = Math.abs(event.touches[0].clientX - start.x);
    const dy = Math.abs(event.touches[0].clientY - start.y);
    if (dy > 12 && dy > dx) { reset(); return; }
    // Claim only a clearly horizontal gesture before Chrome cancels it for
    // viewport panning. Vertical scroll, pinch zoom and excluded controls stay native.
    if (dx > 10 && dx > dy * 1.6 && event.cancelable) event.preventDefault();
  };
  const end = (event: TouchEvent) => {
    const origin = start;
    reset();
    if (!origin || event.changedTouches.length !== 1 || event.touches.length !== 0) return;
    const touch = event.changedTouches[0];
    const href = swipeDestination(options.getPath(), { dx: touch.clientX - origin.x, dy: touch.clientY - origin.y, elapsed: performance.now() - origin.at, rtl: options.rtl() });
    if (href) void options.navigate(href);
  };
  node.addEventListener("touchstart", begin, { passive: true });
  node.addEventListener("touchmove", move, { passive: false });
  node.addEventListener("touchend", end, { passive: true });
  node.addEventListener("touchcancel", reset, { passive: true });
  return {
    update(next: Options) { options = next; reset(); },
    destroy() {
      node.removeEventListener("touchstart", begin);
      node.removeEventListener("touchmove", move);
      node.removeEventListener("touchend", end);
      node.removeEventListener("touchcancel", reset);
    },
  };
}
