let open = $state(false);
export const isGuideOpen = () => open;
export function openGuide() { open = true; }
export function closeGuide() { open = false; }
