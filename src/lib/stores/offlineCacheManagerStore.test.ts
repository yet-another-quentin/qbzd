import { describe, it, expect } from 'vitest';
import { buildRollup, type CachedTrackInfo } from './offlineCacheManagerStore.svelte';

function track(
  id: number,
  artist: string,
  albumId: string | null,
  album: string | null,
  status: CachedTrackInfo['status'] = 'ready',
  fileSizeBytes = 1000,
  quality = 'lossless',
): CachedTrackInfo {
  return {
    trackId: id,
    title: `Track ${id}`,
    artist,
    album,
    albumId,
    durationSecs: 200,
    fileSizeBytes,
    quality,
    bitDepth: 16,
    sampleRate: 44100,
    status,
    progressPercent: status === 'ready' ? 100 : 0,
    errorMessage: null,
    createdAt: new Date(2026, 0, id).toISOString(),
    lastAccessedAt: new Date(2026, 0, id).toISOString(),
  };
}

describe('buildRollup', () => {
  it('groups tracks by artist then album', () => {
    const tracks = [
      track(1, 'Alice In Chains', 'alb-dirt', 'Dirt'),
      track(2, 'Alice In Chains', 'alb-dirt', 'Dirt'),
      track(3, 'Alice In Chains', 'alb-jof', 'Jar Of Flies'),
      track(4, 'Beatles', 'alb-revolver', 'Revolver'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles');
    expect(result).toHaveLength(2);
    expect(result[0].artistName).toBe('Alice In Chains');
    expect(result[0].albumGroups).toHaveLength(2);
    expect(result[1].artistName).toBe('Beatles');
  });

  it('places null albumId tracks in Singles pseudo-album', () => {
    const tracks = [
      track(1, 'Bjork', null, null),
      track(2, 'Bjork', 'alb-debut', 'Debut'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles & loose');
    const bjork = result.find(a => a.artistName === 'Bjork')!;
    expect(bjork.albumGroups).toHaveLength(2);
    const singles = bjork.albumGroups.find(g => g.albumId === null)!;
    expect(singles.title).toBe('Singles & loose');
    expect(singles.cachedTracks).toHaveLength(1);
  });

  it('worstStatus is failed if any track failed', () => {
    const tracks = [
      track(1, 'A', 'alb1', 'Alb', 'ready'),
      track(2, 'A', 'alb1', 'Alb', 'failed'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles');
    expect(result[0].albumGroups[0].worstStatus).toBe('failed');
    expect(result[0].albumGroups[0].failedCount).toBe(1);
  });

  it('dominantQuality returns Mixed when no quality has majority', () => {
    const tracks = [
      track(1, 'A', 'alb1', 'Alb', 'ready', 1000, 'lossless'),
      track(2, 'A', 'alb1', 'Alb', 'ready', 1000, 'hi-res'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles');
    expect(result[0].albumGroups[0].dominantQuality).toBe('Mixed');
  });

  it('canonicalizes artist by case and trims whitespace', () => {
    const tracks = [
      track(1, 'Bjork', 'alb1', 'Alb'),
      track(2, ' BJORK ', 'alb2', 'Alb2'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles');
    expect(result).toHaveLength(1);
    expect(result[0].albumGroups).toHaveLength(2);
  });

  it('isFullyCached reflects passed flag map', () => {
    const tracks = [track(1, 'A', 'alb-x', 'X')];
    const flags = new Map([['alb-x', true]]);
    const result = buildRollup(tracks, flags, 'Singles');
    expect(result[0].albumGroups[0].isFullyCached).toBe(true);
  });

  it('sorts artists alphabetically', () => {
    const tracks = [
      track(1, 'Zebra', 'alb1', 'Z'),
      track(2, 'Alice', 'alb2', 'A'),
      track(3, 'Mountain', 'alb3', 'M'),
    ];
    const result = buildRollup(tracks, new Map(), 'Singles');
    expect(result.map(a => a.artistName)).toEqual(['Alice', 'Mountain', 'Zebra']);
  });
});
