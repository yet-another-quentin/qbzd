<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from 'svelte-i18n';
  import { ArrowLeft } from 'lucide-svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { offlineCacheManagerStore } from '$lib/stores/offlineCacheManagerStore.svelte';

  type Props = {
    onBack: () => void;
    onGoToAlbum: (albumId: string) => void;
    onGoToFavorites: () => void;
  };
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { onBack, onGoToAlbum, onGoToFavorites }: Props = $props();

  const store = offlineCacheManagerStore;

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  type SortKey = 'alpha' | 'recent' | 'largest' | 'smallest';

  function sortKeyToLabel(key: SortKey): string {
    switch (key) {
      case 'alpha': return $t('offlineManager.sort.alpha');
      case 'recent': return $t('offlineManager.sort.recent');
      case 'largest': return $t('offlineManager.sort.largest');
      case 'smallest': return $t('offlineManager.sort.smallest');
    }
  }

  function labelToSortKey(label: string): SortKey {
    if (label === $t('offlineManager.sort.alpha')) return 'alpha';
    if (label === $t('offlineManager.sort.recent')) return 'recent';
    if (label === $t('offlineManager.sort.largest')) return 'largest';
    return 'smallest';
  }

  function getSortOptions(): string[] {
    return [
      $t('offlineManager.sort.alpha'),
      $t('offlineManager.sort.recent'),
      $t('offlineManager.sort.largest'),
      $t('offlineManager.sort.smallest'),
    ];
  }

  onMount(async () => {
    store.setSinglesLabel($t('offlineManager.singlesPseudoAlbum'));
    await store.loadAll();
    await store.subscribeToProgress();
  });

  onDestroy(() => {
    store.unsubscribe();
  });
</script>

<div class="offline-cache-manager">
  <header class="ocm-header">
    <button type="button" class="back-btn" onclick={onBack} aria-label={$t('actions.back')}>
      <ArrowLeft size={18} />
    </button>
    <h1>{$t('offlineManager.title')}</h1>
  </header>

  {#if store.loading}
    <div class="ocm-loading">{$t('actions.loading')}</div>
  {:else if store.artists.length === 0}
    <div class="ocm-empty">
      <h2>{$t('offlineManager.empty.title')}</h2>
      <p>{$t('offlineManager.empty.body')}</p>
    </div>
  {:else}
    <section class="ocm-stats">
      {#if store.stats}
        <span class="ocm-stat-totals">
          {$t('offlineManager.stats.totals', {
            values: {
              tracks: store.stats.totalTracks,
              albums: store.artists.reduce((s, a) => s + a.albumGroups.length, 0),
              artists: store.artists.length,
            },
          })}
        </span>
        <span class="ocm-stat-usage">
          {#if store.stats.limitBytes}
            {$t('offlineManager.stats.usage', {
              values: {
                used: formatBytes(store.stats.totalSizeBytes),
                limit: formatBytes(store.stats.limitBytes),
              },
            })}
            <span class="ocm-usage-bar">
              <span
                class="ocm-usage-fill"
                style:width="{Math.min(100, (store.stats.totalSizeBytes / store.stats.limitBytes) * 100)}%"
              ></span>
            </span>
          {:else}
            {$t('offlineManager.stats.unlimited', {
              values: { used: formatBytes(store.stats.totalSizeBytes) },
            })}
          {/if}
        </span>
      {/if}

      <div class="ocm-controls">
        <span class="ocm-sort-label">{$t('offlineManager.sort.label')}</span>
        <Dropdown
          value={sortKeyToLabel(store.sort)}
          options={getSortOptions()}
          onchange={(label) => store.setSort(labelToSortKey(label))}
        />
        <label class="ocm-toggle">
          <input
            type="checkbox"
            checked={store.showOnlyFailed}
            onchange={(e) => store.setShowOnlyFailed((e.target as HTMLInputElement).checked)}
          />
          <span>{$t('offlineManager.filter.showOnlyFailed')}</span>
        </label>
      </div>
    </section>

    <div class="ocm-body">
      <!-- left rail and right pane added in P6.2 / P6.3 -->
    </div>
  {/if}
</div>

<style>
  .offline-cache-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    color: var(--text-primary);
  }
  .ocm-header { display: flex; align-items: center; gap: 12px; padding: 16px 24px; }
  .ocm-header h1 { margin: 0; font-size: 1.5rem; font-weight: 600; }
  .back-btn { background: transparent; border: none; color: var(--text-muted); cursor: pointer; padding: 8px; border-radius: 6px; }
  .back-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .ocm-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    padding: 8px 24px 16px;
    align-items: center;
    font-size: 0.875rem;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-subtle);
  }
  .ocm-stat-totals, .ocm-stat-usage { white-space: nowrap; }
  .ocm-usage-bar { display: inline-block; vertical-align: middle; width: 120px; height: 4px; background: var(--bg-tertiary); border-radius: 2px; margin-left: 8px; overflow: hidden; }
  .ocm-usage-fill { display: block; height: 100%; background: var(--accent-primary); transition: width 0.2s ease; }
  .ocm-controls { margin-left: auto; display: flex; gap: 12px; align-items: center; }
  .ocm-sort-label { color: var(--text-muted); font-size: 0.85rem; }
  .ocm-toggle { display: flex; gap: 6px; align-items: center; cursor: pointer; }
  .ocm-loading { padding: 32px; text-align: center; color: var(--text-muted); }
  .ocm-empty { padding: 64px 24px; text-align: center; }
  .ocm-empty h2 { margin: 0 0 8px; font-size: 1.25rem; font-weight: 600; }
  .ocm-empty p { margin: 0; color: var(--text-muted); }
  .ocm-body { display: flex; flex: 1; min-height: 0; }
</style>
