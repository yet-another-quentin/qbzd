/**
 * Folder tree entry types
 *
 * Mirrors the Rust `FolderTreeEntry` enum from `crates/qbz-library/src/models.rs`,
 * deserialized via the `serde(tag = "kind", rename_all = "snake_case")` representation.
 *
 * Used by `LocalLibraryFolderTree.svelte` (recursive tree row component) and
 * `LocalLibraryFolderDetail.svelte` (right-pane folder content view) to render
 * filesystem-hierarchy navigation in the LocalLibrary Folders tab tree mode.
 */

export type FolderEntry = {
  kind: 'folder';
  path: string;
  segment: string;
  track_count_under: number;
  artwork: string | null;
};

export type TrackEntry = {
  kind: 'track';
  path: string;
  segment: string;
};

export type FolderTreeEntry = FolderEntry | TrackEntry;
