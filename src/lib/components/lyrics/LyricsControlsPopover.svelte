<script lang="ts">
  import { Check, Copy, RotateCcw } from 'lucide-svelte';
  import { t } from 'svelte-i18n';
  import { fade } from 'svelte/transition';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import Dropdown from '../Dropdown.svelte';
  import {
    lyricsDisplayStore,
    setLyricsAutoFollow,
    setLyricsFont,
    setLyricsFontSize,
    setLyricsDimming,
    setLyricsActiveColor,
    setLyricsUppercase,
    resetLyricsDisplay,
    type LyricsFont,
    type LyricsFontSize,
    type LyricsDimming
  } from '$lib/stores/lyricsDisplayStore';
  import { showToast } from '$lib/stores/toastStore';

  interface LyricsLine {
    text: string;
  }

  interface Props {
    open: boolean;
    anchorEl: HTMLElement | null;
    onClose: () => void;
    lines?: LyricsLine[];
    canCopy?: boolean;
  }

  let { open, anchorEl, onClose, lines = [], canCopy = false }: Props = $props();

  let copied = $state(false);
  let copyResetTimer: ReturnType<typeof setTimeout> | null = null;

  async function copyLyrics(): Promise<void> {
    if (!canCopy) return;
    const text = lines.map((l) => l.text).join('\n');
    try {
      await writeText(text);
      showToast($t('player.lyricsControls.copySuccess'), 'success');
      copied = true;
      if (copyResetTimer) clearTimeout(copyResetTimer);
      copyResetTimer = setTimeout(() => {
        copied = false;
        copyResetTimer = null;
      }, 1500);
    } catch (err) {
      console.error('[LyricsControlsPopover] Copy to clipboard failed:', err);
      showToast($t('player.lyricsControls.copyError'), 'error');
    }
  }

  let popoverEl: HTMLDivElement | null = $state(null);

  const prefs = $derived($lyricsDisplayStore);

  const fontOptions: { value: LyricsFont; labelKey: string }[] = [
    { value: 'system', labelKey: 'player.lyricsControls.fonts.system' },
    { value: 'line-seed-jp', labelKey: 'player.lyricsControls.fonts.lineSeedJp' },
    { value: 'montserrat', labelKey: 'player.lyricsControls.fonts.montserrat' },
    { value: 'noto-sans', labelKey: 'player.lyricsControls.fonts.notoSans' },
    { value: 'source-sans-3', labelKey: 'player.lyricsControls.fonts.sourceSans3' }
  ];

  const sizeOptions: { value: LyricsFontSize; labelKey: string }[] = [
    { value: 'small', labelKey: 'player.lyricsControls.sizes.small' },
    { value: 'medium', labelKey: 'player.lyricsControls.sizes.medium' },
    { value: 'large', labelKey: 'player.lyricsControls.sizes.large' },
    { value: 'xl', labelKey: 'player.lyricsControls.sizes.xl' }
  ];

  const dimmingOptions: { value: LyricsDimming; labelKey: string }[] = [
    { value: 'off', labelKey: 'player.lyricsControls.dimmingLevels.off' },
    { value: 'soft', labelKey: 'player.lyricsControls.dimmingLevels.soft' },
    { value: 'strong', labelKey: 'player.lyricsControls.dimmingLevels.strong' }
  ];

  function handleFontChange(label: string): void {
    const match = fontOptions.find((opt) => $t(opt.labelKey) === label);
    if (match) setLyricsFont(match.value);
  }

  /** Fallback hex shown in the color input when the user hasn't picked one yet. */
  const FALLBACK_COLOR = '#8b5cf6';

  function handleActiveColorInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    setLyricsActiveColor(target.value);
  }

  function handleClickOutside(event: MouseEvent) {
    if (!open) return;
    const target = event.target as Node;
    if (popoverEl && popoverEl.contains(target)) return;
    if (anchorEl && anchorEl.contains(target)) return;
    onClose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      event.stopPropagation();
      onClose();
    }
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeydown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeydown);
    };
  });
</script>

{#if open}
  <div
    class="popover"
    bind:this={popoverEl}
    role="dialog"
    aria-label={$t('player.lyricsControls.title')}
    transition:fade={{ duration: 150 }}
  >
    <div class="row">
      <span class="label">{$t('player.lyricsControls.autoFollow')}</span>
      <button
        type="button"
        class="switch"
        class:on={prefs.autoFollow}
        role="switch"
        aria-checked={prefs.autoFollow}
        aria-label={$t('player.lyricsControls.autoFollow')}
        onclick={() => setLyricsAutoFollow(!prefs.autoFollow)}
      >
        <span class="switch-thumb"></span>
      </button>
    </div>

    <div class="row">
      <span class="label">{$t('player.lyricsControls.font')}</span>
      <div class="dropdown-wrap">
        <Dropdown
          value={$t(fontOptions.find((opt) => opt.value === prefs.font)?.labelKey ?? fontOptions[0].labelKey)}
          options={fontOptions.map((opt) => $t(opt.labelKey))}
          onchange={handleFontChange}
          expandLeft
          compact
        />
      </div>
    </div>

    <div class="row">
      <span class="label">{$t('player.lyricsControls.size')}</span>
      <div class="segmented" role="group" aria-label={$t('player.lyricsControls.size')}>
        {#each sizeOptions as opt (opt.value)}
          <button
            type="button"
            class="seg"
            class:active={prefs.fontSize === opt.value}
            aria-pressed={prefs.fontSize === opt.value}
            onclick={() => setLyricsFontSize(opt.value)}
          >
            {$t(opt.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    <div class="row">
      <span class="label">{$t('player.lyricsControls.activeColor')}</span>
      <div class="color-wrap">
        {#if prefs.activeColor}
          <button
            type="button"
            class="color-clear"
            aria-label={$t('player.lyricsControls.useThemeColor')}
            title={$t('player.lyricsControls.useThemeColor')}
            onclick={() => setLyricsActiveColor('')}
          >
            {$t('player.lyricsControls.useThemeColor')}
          </button>
        {/if}
        <input
          type="color"
          class="color-input"
          aria-label={$t('player.lyricsControls.activeColor')}
          value={prefs.activeColor || FALLBACK_COLOR}
          oninput={handleActiveColorInput}
        />
      </div>
    </div>

    <div class="row">
      <span class="label">{$t('player.lyricsControls.uppercase')}</span>
      <button
        type="button"
        class="switch"
        class:on={prefs.uppercase}
        role="switch"
        aria-checked={prefs.uppercase}
        aria-label={$t('player.lyricsControls.uppercase')}
        onclick={() => setLyricsUppercase(!prefs.uppercase)}
      >
        <span class="switch-thumb"></span>
      </button>
    </div>

    <div class="row">
      <span class="label">{$t('player.lyricsControls.dimming')}</span>
      <div class="segmented" role="group" aria-label={$t('player.lyricsControls.dimming')}>
        {#each dimmingOptions as opt (opt.value)}
          <button
            type="button"
            class="seg"
            class:active={prefs.dimming === opt.value}
            aria-pressed={prefs.dimming === opt.value}
            onclick={() => setLyricsDimming(opt.value)}
          >
            {$t(opt.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    <div class="footer">
      <button
        type="button"
        class="footer-btn"
        class:copied
        disabled={!canCopy}
        aria-label={$t('player.lyricsControls.copyLyrics')}
        onclick={copyLyrics}
      >
        {#if copied}
          <Check size={14} />
        {:else}
          <Copy size={14} />
        {/if}
        <span>{$t('player.lyricsControls.copyLyrics')}</span>
      </button>
      <button type="button" class="footer-btn" onclick={resetLyricsDisplay}>
        <RotateCcw size={14} />
        <span>{$t('player.lyricsControls.reset')}</span>
      </button>
    </div>
  </div>
{/if}

<style>
  .popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 8px;
    z-index: 100;
    width: 268px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg-primary);
    border: 1px solid var(--bg-tertiary);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 28px;
  }

  .label {
    font-size: 12px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .switch {
    width: 36px;
    height: 20px;
    border-radius: 999px;
    background: var(--bg-tertiary);
    border: 1px solid var(--bg-tertiary);
    padding: 0;
    position: relative;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .switch.on {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
  }

  .switch-thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-primary);
    transition: transform 150ms ease;
  }

  .switch.on .switch-thumb {
    transform: translateX(16px);
  }

  /* Shrink the app Dropdown to fit the popover scale. */
  .dropdown-wrap :global(.dropdown .trigger) {
    height: 28px;
    width: 170px;
    padding: 0 10px;
    font-size: 12px;
    border-radius: 6px;
  }

  .color-wrap {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .color-input {
    width: 36px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--bg-tertiary);
    border-radius: 6px;
    background: var(--bg-secondary);
    cursor: pointer;
    overflow: hidden;
  }

  /* Remove the default inner padding/border of the color input to show a clean swatch. */
  .color-input::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .color-input::-webkit-color-swatch {
    border: none;
    border-radius: 4px;
  }
  .color-input::-moz-color-swatch {
    border: none;
    border-radius: 4px;
  }

  .color-clear {
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--bg-tertiary);
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 11px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color 150ms ease, background 150ms ease, border-color 150ms ease;
  }

  .color-clear:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .segmented {
    display: inline-flex;
    border: 1px solid var(--bg-tertiary);
    border-radius: 6px;
    overflow: hidden;
  }

  .seg {
    background: transparent;
    color: var(--text-muted);
    border: none;
    padding: 4px 10px;
    font-size: 12px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background 150ms ease, color 150ms ease;
  }

  .seg:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .seg.active {
    background: var(--accent-primary);
    color: var(--btn-primary-text);
  }

  .seg + .seg {
    border-left: 1px solid var(--bg-tertiary);
  }

  .footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 8px;
    border-top: 1px solid var(--bg-tertiary);
    gap: 8px;
  }

  .footer-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--text-muted);
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color 150ms ease, background 150ms ease, border-color 150ms ease;
  }

  .footer-btn:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--bg-secondary);
    border-color: var(--bg-tertiary);
  }

  .footer-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .footer-btn.copied {
    color: var(--accent-primary);
  }
</style>
