export type ExternalUrlEvent = {
  id: number;
  url: string;
  source: string;
  action: string;
};

let nextExternalId = 0;
let pendingPrefill: ExternalUrlEvent | null = $state(null);
let pendingShare: ExternalUrlEvent | null = $state(null);

export function queueExternalPrefill(event: Omit<ExternalUrlEvent, "id">) {
  pendingPrefill = {
    ...event,
    id: nextExternalId++,
  };
}

export function getPendingExternalPrefill(): ExternalUrlEvent | null {
  return pendingPrefill;
}

export function clearPendingExternalPrefill(id: number) {
  if (pendingPrefill?.id === id) {
    pendingPrefill = null;
  }
}

export function queueExternalShare(event: Omit<ExternalUrlEvent, "id">) {
  pendingShare = {
    ...event,
    id: nextExternalId++,
  };
}

export function getPendingExternalShare(): ExternalUrlEvent | null {
  return pendingShare;
}

export function clearPendingExternalShare(id: number) {
  if (pendingShare?.id === id) {
    pendingShare = null;
  }
}
