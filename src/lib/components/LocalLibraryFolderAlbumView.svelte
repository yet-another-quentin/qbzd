<script lang="ts">
  // Compact album-detail variant rendered inside the LocalLibrary Folders
  // tab tree-mode right pane. Mirrors the data layout of the full-page
  // local-library album-detail (header + track list) but strips backdrop,
  // sidebar sections, "more by this artist" carousel, and any chrome that
  // would compete with the tree rail on the left. The full-page view
  // (line ~4573 in LocalLibraryView.svelte) remains unchanged for flat
  // mode and direct navigation to a library album.
  //
  // Helpers (formatDuration, getQualityBadge, etc.) are received as props
  // following the same idiom as VirtualizedTrackList — keeps the component
  // free of tight coupling to LocalLibraryView's local helpers without
  // duplicating them.

  import { Disc3, Play, Shuffle, CircleAlert } from 'lucide-svelte';
  import { t } from '$lib/i18n';
  import TrackRow from './TrackRow.svelte';
  import { formatTrackTitle } from '$lib/utils/trackTitle';

  interface LocalTrack {
    id: number;
    file_path: string;
    title: string;
    artist: string;
    album: string;
    album_artist?: string;
    album_group_key?: string;
    album_group_title?: string;
    track_number?: number;
    disc_number?: number;
    year?: number;
    genre?: string;
    catalog_number?: string;
    duration_secs: number;
    format: string;
    bit_depth?: number;
    sample_rate: number;
    channels: number;
    file_size_bytes: number;
    cue_file_path?: string;
    cue_start_secs?: number;
    cue_end_secs?: number;
    artwork_path?: string;
    last_modified: number;
    indexed_at: number;
    source?: string;
  }

  interface LocalAlbum {
    id: string;
    title: string;
    artist: string;
    all_artists?: string;
    year?: number;
    genre?: string;
    catalog_number?: string;
    artwork_path?: string;
    track_count: number;
    total_duration_secs: number;
    format: string;
    bit_depth?: number;
    sample_rate: number;
    directory_path: string;
    source_folders?: string | null;
    source?: string;
    likely_single_file_album?: boolean;
  }

  interface AlbumSection {
    disc: number;
    label: string;
    tracks: LocalTrack[];
    useIndexNumbering: boolean;
  }

  interface Props {
    album: LocalAlbum;
    tracks: LocalTrack[];
    activeTrackId?: number | null;
    isPlaybackActive?: boolean;
    onPlayAll: () => void;
    onShuffleAll: () => void;
    onTrackPlay: (track: LocalTrack) => void;
    onTrackPlayNext?: (track: LocalTrack) => void;
    onTrackPlayLater?: (track: LocalTrack) => void;
    onTrackAddToPlaylist?: (trackId: number) => void;
    onTrackAddPlexToPlaylist?: (filePath: string) => void;
    onArtistClick?: (name: string) => void;
    formatDuration: (seconds: number) => string;
    formatTotalDuration: (seconds: number) => string;
    formatBitDepth: (bits?: number) => string;
    formatSampleRate: (hz: number) => string;
    getQualityBadge: (track: LocalTrack) => string;
    isHiRes: (track: LocalTrack) => boolean;
    getFullArtworkUrl: (path?: string) => string;
    buildAlbumSections: (tracks: LocalTrack[]) => AlbumSection[];
    normalizeArtistName: (name: string) => string;
  }

  let {
    album,
    tracks,
    activeTrackId = null,
    isPlaybackActive = false,
    onPlayAll,
    onShuffleAll,
    onTrackPlay,
    onTrackPlayNext,
    onTrackPlayLater,
    onTrackAddToPlaylist,
    onTrackAddPlexToPlaylist,
    onArtistClick,
    formatDuration,
    formatTotalDuration,
    formatBitDepth,
    formatSampleRate,
    getQualityBadge,
    isHiRes,
    getFullArtworkUrl,
    buildAlbumSections,
    normalizeArtistName,
  }: Props = $props();

  const albumSections = $derived(buildAlbumSections(tracks));
  const showDiscHeaders = $derived(albumSections.length > 1);
  const isVariousArtists = $derived(
    normalizeArtistName(album.artist) === 'various artists'
  );
</script>

<div class="folder-album-view">
  <!-- Compact header: artwork on the left, metadata + actions on the right.
       No artwork backdrop, no back button (the tree on the left is the
       navigation), no edit / search / select chrome — those belong to the
       full-page album-detail view. -->
  <div class="folder-album-header">
    <div class="folder-album-artwork">
      {#if album.artwork_path}
        <img src={getFullArtworkUrl(album.artwork_path)} alt={album.title} />
      {:else}
        <div class="folder-album-artwork-placeholder">
          <Disc3 size={48} />
        </div>
      {/if}
    </div>
    <div class="folder-album-info">
      <h2 class="folder-album-title" title={album.title}>{album.title}</h2>
      {#if !isVariousArtists && onArtistClick}
        <button
          type="button"
          class="folder-album-artist artist-link"
          onclick={() => onArtistClick?.(album.artist)}
        >
          {album.artist}
        </button>
      {:else}
        <p class="folder-album-artist">{album.artist}</p>
      {/if}
      <p class="folder-album-meta">
        {#if album.year}{album.year} &bull; {/if}
        {album.track_count} {$t('library.tracks').toLowerCase()} &bull;
        {formatTotalDuration(album.total_duration_secs)}
      </p>
      {#if tracks.length > 0}
        {@const firstTrack = tracks[0]}
        <div class="folder-album-specs">
          <span class="folder-album-spec-badge" class:hires={isHiRes(firstTrack)}>
            {firstTrack.format.toUpperCase()}
          </span>
          <span class="folder-album-spec-item">{formatBitDepth(firstTrack.bit_depth)}</span>
          <span class="folder-album-spec-item">{formatSampleRate(firstTrack.sample_rate)}</span>
        </div>
      {/if}
      {#if album.likely_single_file_album}
        <div class="folder-album-single-file-notice">
          <CircleAlert size={12} />
          <span>{$t('library.singleFileAlbumNotice')}</span>
        </div>
      {/if}
      <div class="folder-album-actions">
        <button
          type="button"
          class="folder-album-action-btn primary"
          onclick={onPlayAll}
          disabled={tracks.length === 0}
          title={$t('actions.playAll')}
          aria-label={$t('actions.playAll')}
        >
          <Play size={16} fill="currentColor" color="currentColor" />
          <span>{$t('actions.playAll')}</span>
        </button>
        <button
          type="button"
          class="folder-album-action-btn"
          onclick={onShuffleAll}
          disabled={tracks.length === 0}
          title={$t('actions.shuffle')}
          aria-label={$t('actions.shuffle')}
        >
          <Shuffle size={14} />
        </button>
      </div>
    </div>
  </div>

  <!-- Track list. Reuses TrackRow with the same callback wiring as the
       full-page album-detail view so the queue/play/context-menu actions
       behave identically. Multi-select is intentionally omitted in the
       compact view: tree mode already exposes recursive multi-select via
       the FolderTree rail, which is the canonical bulk-select surface
       inside the Folders tab. -->
  <div class="folder-album-tracks">
    <div class="folder-album-tracks-header">
      <div class="col-number">#</div>
      <div class="col-title">{$t('tracklist.title')}</div>
      <div class="col-duration">{$t('tracklist.duration')}</div>
      <div class="col-quality">{$t('tracklist.quality')}</div>
      <div class="col-spacer"></div>
      <div class="col-spacer"></div>
      <div class="col-spacer"></div>
    </div>
    {#each albumSections as section (section.disc)}
      {#if showDiscHeaders}
        <div class="folder-album-disc-header">{section.label}</div>
      {/if}
      {#each section.tracks as track, index (track.id)}
        <TrackRow
          number={section.useIndexNumbering ? index + 1 : (track.track_number || index + 1)}
          title={formatTrackTitle(track)}
          artist={track.artist !== album.artist ? track.artist : undefined}
          duration={formatDuration(track.duration_secs)}
          quality={getQualityBadge(track)}
          isPlaying={isPlaybackActive && activeTrackId === track.id}
          isActiveTrack={activeTrackId === track.id}
          isLocal={true}
          localSource={track.source === 'plex' ? 'plex' : 'local'}
          hideDownload={true}
          hideFavorite={true}
          onArtistClick={track.artist && track.artist !== album.artist && onArtistClick
            ? () => onArtistClick?.(track.artist)
            : undefined}
          onPlay={() => onTrackPlay(track)}
          menuActions={{
            onPlayNow: () => onTrackPlay(track),
            onPlayNext: onTrackPlayNext ? () => onTrackPlayNext!(track) : undefined,
            onPlayLater: onTrackPlayLater ? () => onTrackPlayLater!(track) : undefined,
            onAddToPlaylist: track.source === 'plex'
              ? (onTrackAddPlexToPlaylist ? () => onTrackAddPlexToPlaylist!(track.file_path) : undefined)
              : (onTrackAddToPlaylist ? () => onTrackAddToPlaylist!(track.id) : undefined)
          }}
        />
      {/each}
    {/each}
    {#if tracks.length === 0}
      <div class="folder-album-empty">
        {$t('library.foldersTree.treeLoading')}
      </div>
    {/if}
  </div>
</div>

<style>
  .folder-album-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 4px 4px 24px 4px;
  }

  /* Compact header: smaller artwork (160px vs 200px in full view), no
     artwork backdrop, action row sits inline with metadata. */
  .folder-album-header {
    display: flex;
    gap: 16px;
    padding: 8px 4px 16px 4px;
    border-bottom: 1px solid var(--bg-tertiary);
    margin-bottom: 12px;
  }

  .folder-album-artwork {
    width: 160px;
    height: 160px;
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .folder-album-artwork img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .folder-album-artwork-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    color: var(--text-muted);
  }

  .folder-album-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 4px;
  }

  .folder-album-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
  }

  .folder-album-artist {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .folder-album-artist.artist-link {
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: var(--accent-primary);
    font-family: inherit;
  }

  .folder-album-artist.artist-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .folder-album-meta {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
  }

  .folder-album-specs {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
    flex-wrap: wrap;
  }

  .folder-album-spec-badge {
    padding: 2px 8px;
    background: var(--bg-tertiary);
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .folder-album-spec-badge.hires {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    color: white;
  }

  .folder-album-spec-item {
    font-size: 11px;
    color: var(--text-secondary);
    padding: 2px 6px;
    background: var(--bg-secondary);
    border-radius: 4px;
  }

  .folder-album-single-file-notice {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border-radius: 6px;
    padding: 4px 8px;
    max-width: fit-content;
    margin-top: 4px;
  }

  .folder-album-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .folder-album-action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--bg-tertiary);
    border-radius: 999px;
    cursor: pointer;
    font-family: inherit;
    font-size: 12px;
    transition: opacity 100ms ease, background 100ms ease;
  }

  .folder-album-action-btn.primary {
    background: var(--accent-primary);
    color: var(--btn-primary-text, white);
    border-color: transparent;
  }

  .folder-album-action-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .folder-album-action-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* Track list — reuse TrackRow's grid columns. The header here mirrors
     the full-page album-detail track-list-header so the TrackRow row
     widths line up. */
  .folder-album-tracks {
    display: flex;
    flex-direction: column;
  }

  .folder-album-tracks-header {
    display: grid;
    grid-template-columns: 50px 1fr 80px 100px 32px 32px 32px;
    gap: 8px;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid var(--bg-tertiary);
    margin-bottom: 4px;
  }

  .folder-album-tracks-header .col-number {
    text-align: center;
  }

  .folder-album-disc-header {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 12px 12px 4px 12px;
  }

  .folder-album-empty {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
