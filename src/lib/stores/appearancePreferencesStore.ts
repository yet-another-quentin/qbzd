/**
 * Appearance Preferences Store
 *
 * Per-user toggles for visual treatments that have a clear opt-in/opt-out
 * value (gradients, blurs, motion). Persisted in localStorage.
 */

export interface AppearancePreferences {
  /** Render the artwork-derived gradient on album detail headers. */
  albumHeaderGradient: boolean;
}

const STORAGE_KEY = 'qbz-appearance-preferences';

const DEFAULT_SETTINGS: AppearancePreferences = {
  albumHeaderGradient: true,
};

let settings: AppearancePreferences = loadSettings();
const listeners = new Set<() => void>();

function notifyListeners(): void {
  for (const listener of listeners) listener();
}

function loadSettings(): AppearancePreferences {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved) as Partial<AppearancePreferences>;
      return { ...DEFAULT_SETTINGS, ...parsed };
    }
  } catch (err) {
    console.error('Failed to load appearance preferences:', err);
  }
  return { ...DEFAULT_SETTINGS };
}

function saveSettings(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (err) {
    console.error('Failed to save appearance preferences:', err);
  }
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getSettings(): AppearancePreferences {
  return settings;
}

export function isAlbumHeaderGradientEnabled(): boolean {
  return settings.albumHeaderGradient;
}

export function setAlbumHeaderGradient(enabled: boolean): void {
  if (settings.albumHeaderGradient === enabled) return;
  settings = { ...settings, albumHeaderGradient: enabled };
  saveSettings();
  notifyListeners();
}
