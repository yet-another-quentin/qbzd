<script lang="ts">
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import { Folder, Play, Music2, Search, X, LayoutGrid, List } from 'lucide-svelte';
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
    /**
     * Forward the same exclude-network-folders flag flat-mode search
     * uses, so the right-pane subfolder + direct-tracks listings stay
     * consistent with the tree rail and the recursive-play boundary.
     * Caller computes via `shouldExcludeNetworkFolders()`.
     */
    excludeNetworkFolders?: boolean;
  }

  let {
    folderPath,
    onSubfolderClick,
    onPlayTrack,
    onPlayAllRecursive,
    excludeNetworkFolders = false,
  }: Props = $props();

  let subfolders = $state<FolderEntry[]>([]);
  let directTracks = $state<LocalTrack[]>([]);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  // Local UI state: subfolder name filter + grid/list view toggle.
  // View mode is component-local (no backend persistence) — keeps the
  // change small and focused; can be promoted to a stored preference later.
  let albumSearchQuery = $state('');
  let viewMode = $state<'grid' | 'list'>('grid');

  // Total recursive count = sum of subfolder track_count_under + count of
  // direct-children tracks in this folder. Pure derivation, no $t() inside.
  const totalTracksUnder = $derived(
    subfolders.reduce((sum, sub) => sum + sub.track_count_under, 0) + directTracks.length
  );

  // Filtered subfolders for the search input. Pure derivation, no $t().
  const filteredSubfolders = $derived.by(() => {
    const q = albumSearchQuery.trim().toLowerCase();
    if (!q) return subfolders;
    return subfolders.filter((sub) => sub.segment.toLowerCase().includes(q));
  });

  const isSearchingAlbums = $derived(albumSearchQuery.trim().length > 0);

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
    // Reset transient UI state when navigating to a different folder so a
    // stale album-name filter doesn't carry across folders.
    albumSearchQuery = '';
    if (!path) {
      subfolders = [];
      directTracks = [];
      return;
    }
    loading = true;
    loadError = null;
    Promise.all([
      invoke<FolderTreeEntry[]>('v2_library_list_folder_children', {
        parentPath: path,
        excludeNetworkFolders,
      }),
      invoke<LocalTrack[]>('v2_library_list_folder_tracks', {
        folderPath: path,
        excludeNetworkFolders,
      }),
    ])
      .then(([children, tracks]) => {
        if (path !== folderPath) return;
        subfolders = children.filter((c): c is FolderEntry => c.kind === 'folder');
        directTracks = tracks;
      })
      .catch((e: unknown) => {
        if (path !== folderPath) return;
        loadError = String(e);
        // eslint-disable-next-line no-console
        console.error('[FolderDetail] load failed', e);
      })
      .finally(() => {
        if (path !== folderPath) return;
        loading = false;
      });
  });

  function thumbUrl(artwork: string | null): string | null {
    return artwork ? convertFileSrc(artwork) : null;
  }
</script>

<div class="folder-detail">
  <header class="folder-detail-header">
    <button
      type="button"
      class="action-btn-circle primary"
      onclick={() => onPlayAllRecursive(folderPath)}
      disabled={totalTracksUnder === 0}
      aria-label={$t('library.foldersTree.playAllRecursive')}
      title={$t('library.foldersTree.playAllRecursive')}
    >
      <Play size={20} fill="currentColor" color="currentColor" />
    </button>
    <div class="header-text">
      <h2 title={folderName}>{folderName}</h2>
      <span class="track-count">
        {$t('library.foldersTree.treeFolderTracks', { values: { count: totalTracksUnder } })}
      </span>
    </div>

    <div class="folder-detail-controls">
      <div
        class="folder-detail-search"
        role="search"
        aria-label={$t('library.foldersTree.searchAlbumsLabel')}
      >
        <Search size={14} />
        <input
          type="search"
          class="folder-detail-search-input"
          placeholder={$t('library.foldersTree.searchAlbumsPlaceholder')}
          bind:value={albumSearchQuery}
          aria-label={$t('library.foldersTree.searchAlbumsLabel')}
        />
        {#if isSearchingAlbums}
          <button
            type="button"
            class="folder-detail-search-clear"
            onclick={() => (albumSearchQuery = '')}
            title={$t('actions.close')}
            aria-label={$t('actions.close')}
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <div
        class="folder-view-mode-toggle"
        role="radiogroup"
        aria-label={$t('library.foldersTree.viewModeLabel')}
      >
        <button
          type="button"
          class="view-mode-btn"
          class:active={viewMode === 'grid'}
          onclick={() => (viewMode = 'grid')}
          title={$t('library.foldersTree.gridView')}
          aria-label={$t('library.foldersTree.gridView')}
          aria-pressed={viewMode === 'grid'}
        >
          <LayoutGrid size={14} />
        </button>
        <button
          type="button"
          class="view-mode-btn"
          class:active={viewMode === 'list'}
          onclick={() => (viewMode = 'list')}
          title={$t('library.foldersTree.listView')}
          aria-label={$t('library.foldersTree.listView')}
          aria-pressed={viewMode === 'list'}
        >
          <List size={14} />
        </button>
      </div>
    </div>
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
        {#if filteredSubfolders.length === 0}
          <div class="state-empty">{$t('library.foldersTree.searchNoResults')}</div>
        {:else if viewMode === 'grid'}
          <div class="subfolders-grid">
            {#each filteredSubfolders as sub (sub.path)}
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
        {:else}
          <div class="subfolders-list">
            {#each filteredSubfolders as sub (sub.path)}
              <button
                type="button"
                class="subfolder-list-row"
                onclick={() => onSubfolderClick(sub.path)}
                title={sub.segment}
              >
                <Folder size={14} class="row-icon" />
                <span class="row-name">{sub.segment}</span>
                <span class="row-count">
                  {$t('library.foldersTree.treeFolderTracks', {
                    values: { count: sub.track_count_under },
                  })}
                </span>
              </button>
            {/each}
          </div>
        {/if}
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
    gap: 12px;
    padding: 16px 16px 16px;
    border-bottom: 1px solid var(--bg-tertiary);
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

  .folder-detail-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }

  /* Album/subfolder name filter — visual language matches the compact
     folder-album track search (LocalLibraryFolderAlbumView). */
  .folder-detail-search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--bg-tertiary);
    border-radius: 6px;
    color: var(--text-muted);
    min-width: 180px;
    transition: border-color 150ms ease;
  }

  .folder-detail-search:focus-within {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .folder-detail-search-input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 12px;
  }

  .folder-detail-search-input::placeholder {
    color: var(--text-muted);
  }

  .folder-detail-search-input::-webkit-search-cancel-button {
    display: none;
  }

  .folder-detail-search-clear {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 50%;
    transition: background 150ms ease, color 150ms ease;
  }

  .folder-detail-search-clear:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  /* Grid/list view-mode toggle. Flat icon buttons matching the tree
     toolbar style — no pills, no segmented chrome. */
  .folder-view-mode-toggle {
    display: inline-flex;
    gap: 2px;
  }

  .view-mode-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    border-radius: 3px;
    cursor: pointer;
    transition: color 120ms ease, background 120ms ease;
  }
  .view-mode-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary, rgba(255, 255, 255, 0.05));
  }
  .view-mode-btn.active {
    color: var(--accent-primary);
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

  /* Compact list view of subfolders — single-line rows, hover highlight. */
  .subfolders-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .subfolder-list-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    transition: background 100ms ease;
  }
  .subfolder-list-row:hover {
    background: var(--bg-hover);
  }

  :global(.folder-detail .row-icon) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .subfolder-list-row .row-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subfolder-list-row .row-count {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 11px;
    flex-shrink: 0;
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
