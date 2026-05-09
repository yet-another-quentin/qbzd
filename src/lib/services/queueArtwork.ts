import { convertFileSrc } from '@tauri-apps/api/core';
import { getUserItem } from '$lib/utils/userStorage';

// Backend CoreQueueTrack stores artwork as either:
//   - a full http(s) URL (Qobuz CDN)
//   - a "file://" URL (local library tracks, pre-formatted by the local track mapper)
//   - a raw filesystem path (older local rows or when the mapper is bypassed)
//   - a raw Plex path like "/library/metadata/<ratingKey>/thumb/<version>"
//     (from PlexCachedTrack, no baseUrl / token)
//
// The UI <img src> can only handle the first two. When a Plex queue track
// reaches NowPlayingBar with a raw Plex path, the browser resolves it against
// the dev port and 404s. Run every queue-track artwork through this helper so
// the Collection / Mixtape / session-restore paths all match what
// LocalLibraryView produces for its own queue.
export function resolveQueueTrackArtwork(artworkUrl?: string | null): string {
  if (!artworkUrl) return '';
  if (/^https?:\/\//i.test(artworkUrl)) return artworkUrl;
  if (artworkUrl.startsWith('file://')) return artworkUrl;
  // Tauri's asset protocol — already resolved upstream (e.g. by
  // LocalLibraryView's getArtworkUrl when populating the ephemeral
  // queue). Without this branch the fallthrough below double-converts
  // and produces a 403 path (asset://localhost/asset%3A%2F%2F...).
  if (artworkUrl.startsWith('asset:')) return artworkUrl;
  if (artworkUrl.startsWith('/library/')) {
    const baseUrl = getUserItem('qbz-plex-poc-base-url') || '';
    const token = getUserItem('qbz-plex-poc-token') || '';
    if (!baseUrl || !token) return '';
    const base = baseUrl.replace(/\/+$/, '');
    const separator = artworkUrl.includes('?') ? '&' : '?';
    return `${base}${artworkUrl}${separator}X-Plex-Token=${encodeURIComponent(token)}`;
  }
  // Treat as a local filesystem path.
  return convertFileSrc(artworkUrl);
}
