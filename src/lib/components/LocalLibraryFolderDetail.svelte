<script lang="ts">
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import { Folder, Play, Music2 } from 'lucide-svelte';
  import type { FolderTreeEntry, FolderEntry } from '$lib/types/folderTree';

  // LocalTrack matches the Rust model in qbz-library — kept inline here to
  // mirror LocalLibraryView.svelte's idiom (it also defines the type inline).
  // If a shared types module appears later, swap to an import.
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

  interface Props {
    folderPath: string;
    onSubfolderClick: (path: string) => void;
    onPlayTrack: (track: LocalTrack) => void;
    onPlayAllRecursive: (folderPath: string) => void;
  }

  let { folderPath, onSubfolderClick, onPlayTrack, onPlayAllRecursive }: Props = $props();

  let subfolders = $state<FolderEntry[]>([]);
  let directTracks = $state<LocalTrack[]>([]);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  // Total recursive count = sum of subfolder track_count_under + count of
  // direct-children tracks in this folder. Pure derivation, no $t() inside.
  const totalTracksUnder = $derived(
    subfolders.reduce((sum, sub) => sum + sub.track_count_under, 0) + directTracks.length
  );

  const folderName = $derived.by(() => {
    if (!folderPath) return '';
    const parts = folderPath.split('/');
    const last = parts[parts.length - 1];
    return last && last.length > 0 ? last : folderPath;
  });

  // Refetch whenever folderPath changes. Wrapping in $effect keeps the
  // store-free deps and re-runs naturally when the prop is reassigned.
  $effect(() => {
    const path = folderPath;
    if (!path) {
      subfolders = [];
      directTracks = [];
      return;
    }
    loading = true;
    loadError = null;
    Promise.all([
      invoke<FolderTreeEntry[]>('v2_library_list_folder_children', { parentPath: path }),
      invoke<LocalTrack[]>('v2_library_list_folder_tracks', { folderPath: path }),
    ])
      .then(([children, tracks]) => {
        subfolders = children.filter((c): c is FolderEntry => c.kind === 'folder');
        directTracks = tracks;
      })
      .catch((e: unknown) => {
        loadError = String(e);
        // eslint-disable-next-line no-console
        console.error('[FolderDetail] load failed', e);
      })
      .finally(() => {
        loading = false;
      });
  });

  function thumbUrl(artwork: string | null): string | null {
    return artwork ? convertFileSrc(artwork) : null;
  }
</script>

<div class="folder-detail">
  <header class="folder-detail-header">
    <Folder size={28} />
    <div class="header-text">
      <h2 title={folderName}>{folderName}</h2>
      <span class="track-count">
        {$t('library.foldersTree.treeFolderTracks', { values: { count: totalTracksUnder } })}
      </span>
    </div>
    <button
      class="play-all-btn"
      onclick={() => onPlayAllRecursive(folderPath)}
      disabled={totalTracksUnder === 0}
      aria-label={$t('library.foldersTree.playAllRecursive')}
      title={$t('library.foldersTree.playAllRecursive')}
    >
      <Play size={16} fill="currentColor" />
      <span>{$t('library.foldersTree.playAllRecursive')}</span>
    </button>
  </header>

  {#if loading}
    <div class="state-empty">{$t('library.foldersTree.treeLoading')}</div>
  {:else if loadError}
    <div class="state-error" title={loadError}>{loadError}</div>
  {:else if subfolders.length === 0 && directTracks.length === 0}
    <div class="state-empty">{$t('library.foldersTree.treeEmpty')}</div>
  {:else}
    {#if subfolders.length > 0}
      <section class="subfolders-section">
        <h3>{$t('library.folders')}</h3>
        <div class="subfolders-grid">
          {#each subfolders as sub (sub.path)}
            {@const thumb = thumbUrl(sub.artwork)}
            <button
              type="button"
              class="subfolder-card"
              onclick={() => onSubfolderClick(sub.path)}
              title={sub.segment}
            >
              {#if thumb}
                <img class="card-thumb" src={thumb} alt="" loading="lazy" decoding="async" />
              {:else}
                <div class="card-thumb-placeholder">
                  <Folder size={32} />
                </div>
              {/if}
              <span class="card-name">{sub.segment}</span>
              <span class="card-meta">
                {$t('library.foldersTree.treeFolderTracks', {
                  values: { count: sub.track_count_under },
                })}
              </span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    {#if directTracks.length > 0}
      <section class="tracks-section">
        <h3>{$t('library.tracks')}</h3>
        <div class="tracks-list">
          {#each directTracks as track (track.id)}
            <button
              type="button"
              class="track-row"
              onclick={() => onPlayTrack(track)}
              title={track.title}
            >
              <Music2 size={14} class="track-icon" />
              <span class="track-name">{track.title}</span>
              <span class="track-artist" title={track.artist}>{track.artist}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .folder-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .folder-detail-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 16px 16px;
    border-bottom: 1px solid var(--bg-tertiary);
    flex-shrink: 0;
  }

  .folder-detail-header > :global(svg) {
    color: var(--accent-primary, var(--text-secondary));
    flex-shrink: 0;
  }

  .header-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .header-text h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-count {
    font-size: 13px;
    color: var(--text-muted);
  }

  .play-all-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    background: var(--accent-primary);
    color: var(--btn-primary-text, white);
    border: none;
    border-radius: 999px;
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    flex-shrink: 0;
    transition: opacity 100ms ease;
  }
  .play-all-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
  .play-all-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  section {
    padding: 16px;
  }
  section h3 {
    margin: 0 0 12px 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .subfolders-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  .subfolder-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--bg-tertiary);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-family: inherit;
    transition: background 100ms ease, border-color 100ms ease;
  }
  .subfolder-card:hover {
    background: var(--bg-hover);
    border-color: var(--accent-soft, var(--bg-tertiary));
  }

  .card-thumb {
    width: 100%;
    aspect-ratio: 1;
    object-fit: cover;
    border-radius: 4px;
    background: var(--bg-tertiary);
  }
  .card-thumb-placeholder {
    width: 100%;
    aspect-ratio: 1;
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .card-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-meta {
    font-size: 11px;
    color: var(--text-muted);
  }

  .tracks-list {
    display: flex;
    flex-direction: column;
  }

  .track-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    border-radius: 4px;
    transition: background 100ms ease;
  }
  .track-row:hover {
    background: var(--bg-hover);
  }

  .track-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-artist {
    color: var(--text-muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 200px;
  }

  :global(.folder-detail .track-icon) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .state-empty,
  .state-error {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
  .state-error {
    color: var(--error, #f55);
  }
</style>
