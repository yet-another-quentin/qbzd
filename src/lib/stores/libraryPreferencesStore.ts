import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { LibraryPreferences } from '$lib/types';

export type LibraryTabType = 'albums' | 'artists' | 'tracks' | 'folders';

const KNOWN_TABS: readonly LibraryTabType[] = ['albums', 'artists', 'tracks', 'folders'];

const DEFAULT_TAB_ORDER: LibraryTabType[] = ['tracks', 'folders', 'albums', 'artists'];

export interface LibraryPreferencesState {
  tab_order: LibraryTabType[];
  hidden_tabs: LibraryTabType[];
}

const DEFAULT_STATE: LibraryPreferencesState = {
  tab_order: [...DEFAULT_TAB_ORDER],
  hidden_tabs: [],
};

export const libraryPreferencesStore = writable<LibraryPreferencesState>({
  tab_order: [...DEFAULT_STATE.tab_order],
  hidden_tabs: [...DEFAULT_STATE.hidden_tabs],
});

function isKnownTab(tab: string): tab is LibraryTabType {
  return (KNOWN_TABS as readonly string[]).includes(tab);
}

/**
 * Sanitize raw preferences from the backend: keep only known tabs, backfill
 * any missing ones at the end of `tab_order` so users with older preferences
 * still surface tabs added in future releases. Mirrors the logic in
 * `LocalLibraryView.svelte`'s `sanitizeLibraryPreferences`.
 */
export function sanitizeLibraryPreferences(prefs: LibraryPreferences | null | undefined): LibraryPreferencesState {
  const rawOrder = prefs?.tab_order ?? [];
  const validOrder: LibraryTabType[] = rawOrder.filter(isKnownTab);
  for (const tab of DEFAULT_TAB_ORDER) {
    if (!validOrder.includes(tab)) validOrder.push(tab);
  }
  const rawHidden = prefs?.hidden_tabs ?? [];
  const validHidden: LibraryTabType[] = rawHidden.filter(isKnownTab);
  return { tab_order: validOrder, hidden_tabs: validHidden };
}

/**
 * Push externally-loaded preferences into the store. Use this from
 * `LocalLibraryView` so the store is kept in sync with the view's local
 * state without the view having to fully migrate to subscribing.
 */
export function setLibraryPreferences(prefs: LibraryPreferences | LibraryPreferencesState): void {
  libraryPreferencesStore.set(sanitizeLibraryPreferences(prefs as LibraryPreferences));
}

/**
 * Fetch preferences directly from the backend and update the store. Used by
 * navigation components (TitleBarNav, Sidebar) that want their own initial
 * load without depending on `LocalLibraryView` having mounted yet.
 */
export async function loadLibraryPreferences(): Promise<void> {
  try {
    const prefs = await invoke<LibraryPreferences>('v2_get_library_preferences');
    libraryPreferencesStore.set(sanitizeLibraryPreferences(prefs));
  } catch (err) {
    console.warn('[libraryPreferencesStore] load failed, keeping defaults:', err);
  }
}
