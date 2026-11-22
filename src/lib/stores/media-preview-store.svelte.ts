export type MediaPreview = {
  url: string;
  title: string;
  author: string;
  thumbnail_url: string | null;
  duration_seconds: number | null;
};

let expectedUrl: string | null = null;
let currentPreview = $state<MediaPreview | null>(null);

export function getMediaPreview(): MediaPreview | null {
  return currentPreview;
}

export function setMediaPreview(preview: MediaPreview | null) {
  if (preview && preview.url !== expectedUrl) return;
  currentPreview = preview;
}

export function clearMediaPreview() {
  currentPreview = null;
  expectedUrl = null;
}

export function requestMediaPreview(url: string) {
  expectedUrl = url;
  currentPreview = null;
}
