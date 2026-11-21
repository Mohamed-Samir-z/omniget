import type { NavItem } from './nav-config';
// Temporarily hide unfinished Android navigation entries. Routes, features and
// the full desktop navigation stay intact for future development.
export const ANDROID_READY_ROUTES = ['/', '/downloads', '/settings', '/about'] as const;
export function androidReadyNav(items: NavItem[]): NavItem[] {
  return items.filter(item => (ANDROID_READY_ROUTES as readonly string[]).includes(item.href));
}
