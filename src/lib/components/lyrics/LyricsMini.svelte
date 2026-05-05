<script lang="ts">
  import { Pause, Play, SkipBack, SkipForward } from 'lucide-svelte';
  import LyricsLines from './LyricsLines.svelte';

  interface LyricsLine {
    text: string;
  }

  interface Props {
    lines: LyricsLine[];
    activeIndex?: number;
    activeProgress?: number;
    isSynced?: boolean;
    title?: string;
    artist?: string;
    artwork?: string;
    isPlaying?: boolean;
    onTogglePlay?: () => void;
    onSkipBack?: () => void;
    onSkipForward?: () => void;
  }

  let {
    lines,
    activeIndex = -1,
    activeProgress = 0,
    isSynced = false,
    title = '',
    artist = '',
    artwork,
    isPlaying = false,
    onTogglePlay,
    onSkipBack,
    onSkipForward
  }: Props = $props();

  const canToggle = $derived(!!onTogglePlay);
  const canSkipBack = $derived(!!onSkipBack);
  const canSkipForward = $derived(!!onSkipForward);
</script>

<div class="lyrics-mini">
  <div class="lyrics-mini-body">
    <LyricsLines
      {lines}
      {activeIndex}
      {activeProgress}
      {isSynced}
      compact={true}
      center={false}
    />
  </div>

  <div class="lyrics-mini-player">
    <div class="track-info">
      <div class="artwork">
        {#if artwork}
          <img src={artwork} alt={title || 'Artwork'} />
        {:else}
          <div class="artwork-placeholder"></div>
        {/if}
      </div>
      <div class="meta">
        <div class="track-title">{title || 'Unknown track'}</div>
        <div class="track-artist">{artist || 'Unknown artist'}</div>
      </div>
    </div>

    <div class="controls">
      <button class="control-btn" onclick={onSkipBack} disabled={!canSkipBack}>
        <SkipBack size={18} />
      </button>
      <button class="control-btn primary" onclick={onTogglePlay} disabled={!canToggle}>
        {#if isPlaying}
          <Pause size={18} />
        {:else}
          <Play size={18} />
        {/if}
      </button>
      <button class="control-btn" onclick={onSkipForward} disabled={!canSkipForward}>
        <SkipForward size={18} />
      </button>
    </div>
  </div>
</div>

<style>
  .lyrics-mini {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-radius: 12px;
    overflow: hidden;
  }

  .lyrics-mini-body {
    flex: 1;
    min-height: 0;
    padding: 18px 18px 8px 18px;
    overflow: hidden;
  }

  .lyrics-mini-body :global(.lyrics-lines) {
    padding: 0;
  }

  .lyrics-mini-player {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-primary);
    border-top: 1px solid var(--bg-tertiary);
  }

  .track-info {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .artwork {
    width: 44px;
    height: 44px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-tertiary);
    display: grid;
    place-items: center;
  }

  .artwork img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .artwork-placeholder {
    width: 60%;
    height: 60%;
    border-radius: 6px;
    background: var(--bg-secondary);
  }

  .meta {
    min-width: 0;
  }

  .track-title {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
  }

  .track-artist {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .control-btn {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 8px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 150ms ease, transform 150ms ease, opacity 150ms ease;
  }

  .control-btn.primary {
    background: var(--accent-primary);
    color: var(--btn-primary-text);
  }

  .control-btn:disabled {
    cursor: default;
    opacity: 0.4;
  }

  .control-btn:not(:disabled):hover {
    background: var(--bg-tertiary);
    transform: translateY(-1px);
  }

  .control-btn.primary:not(:disabled):hover {
    filter: brightness(1.1);
  }
</style>
