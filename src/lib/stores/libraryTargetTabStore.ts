import { writable } from 'svelte/store';
import type { LibraryTabType } from './libraryPreferencesStore';

/**
 * One-shot navigation target: when the user clicks a tab in the title bar
 * or sidebar Local Library dropdown, we set this to the desired tab and
 * trigger navigation to the Local Library view. The view consumes the
 * value (on mount or live via $effect) and clears it back to null.
 */
export const libraryTargetTab = writable<LibraryTabType | null>(null);

export function setLibraryTargetTab(tab: LibraryTabType | null): void {
  libraryTargetTab.set(tab);
}
