/** One exact-URL full metadata request per Home input session, shared by
 * subtitles and audio/format controls. Preview data never enters this cache. */
export function createMediaDetailsLoader<T>(fetchDetails: (url: string) => Promise<T>) {
  let current: { url: string; promise: Promise<T> } | null = null;
  return {
    load(url: string): Promise<T> {
      if (current?.url === url) return current.promise;
      const request = { url, promise: Promise.resolve().then(() => fetchDetails(url)) };
      current = request;
      request.promise.catch(() => { if (current === request) current = null; });
      return request.promise;
    },
    reset() { current = null; },
  };
}
