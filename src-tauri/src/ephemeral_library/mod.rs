//! In-memory ephemeral library for ad-hoc folder playback.
//!
//! The user can point QBZ at a folder that lives outside their library
//! (a downloaded album they haven't decided to keep, an external drive,
//! etc.), browse it, and play tracks from it without anything landing
//! in `local_tracks`. The ephemeral session lives only in memory: a
//! `HashMap<i64, LocalTrack>` keyed by *synthetic ids in the high
//! range* (>= `EPHEMERAL_ID_FLOOR` = 2^48). Synthetic ids in this range
//! are how the rest of the playback pipeline distinguishes ephemeral
//! tracks from DB-resolvable ones — local_tracks autoincrement IDs are
//! orders of magnitude smaller, so any track_id arriving at
//! `v2_library_play_track` at or above the floor is unambiguously
//! ephemeral and gets routed here instead of the DB.
//!
//! The high-positive design (instead of the obvious "use negatives")
//! exists because the queue/playback-context commands serialize ids as
//! `u64` end-to-end (V2QueueTrack, v2_set_playback_context) and reject
//! negative numbers at the serde boundary. Positive ids above the DB
//! range and below 2^53 (JS Number safe limit) are valid u64 *and*
//! survive the JSON round-trip without precision loss.
//!
//! Only one folder is held at a time; opening a new folder replaces the
//! previous session. The state vanishes on app exit by virtue of being
//! in-memory — nothing persists, no migration, no cleanup logic needed.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use qbz_library::{LibraryError, LibraryScanner, LocalTrack, MetadataExtractor};
use serde::Serialize;

/// Floor for synthetic ephemeral track ids. Any id at or above this
/// value is an ephemeral track; below it is a DB row id. Set high
/// enough to be impossible to collide with autoincrement DB ids in any
/// realistic library size, low enough to fit in JS Number's safe
/// integer range (2^53 - 1) so the JSON round-trip stays lossless.
pub const EPHEMERAL_ID_FLOOR: i64 = 1 << 48;

#[derive(Debug, Serialize, Clone)]
pub struct EphemeralFolderResult {
    pub folder_path: String,
    pub tracks: Vec<LocalTrack>,
    pub skipped_files: usize,
}

#[derive(Debug)]
pub enum EphemeralError {
    Lock,
    Library(String),
    Io(String),
}

impl std::fmt::Display for EphemeralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lock => write!(f, "ephemeral library state lock poisoned"),
            Self::Library(msg) => write!(f, "{}", msg),
            Self::Io(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<LibraryError> for EphemeralError {
    fn from(e: LibraryError) -> Self {
        EphemeralError::Library(e.to_string())
    }
}

struct EphemeralLibraryInner {
    tracks: HashMap<i64, LocalTrack>,
    next_id: i64,
    current_folder_path: Option<String>,
}

impl EphemeralLibraryInner {
    fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            next_id: EPHEMERAL_ID_FLOOR,
            current_folder_path: None,
        }
    }

    fn reset(&mut self) {
        self.tracks.clear();
        self.next_id = EPHEMERAL_ID_FLOOR;
        self.current_folder_path = None;
    }
}

pub struct EphemeralLibraryState {
    inner: Mutex<EphemeralLibraryInner>,
}

impl EphemeralLibraryState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(EphemeralLibraryInner::new()),
        }
    }

    /// Scan a folder, extract metadata for every supported audio file
    /// found, assign synthetic negative ids and stash the result. The
    /// previous ephemeral session, if any, is dropped.
    pub fn open_folder(&self, path: &Path) -> Result<EphemeralFolderResult, EphemeralError> {
        if !path.exists() {
            return Err(EphemeralError::Io(format!(
                "Folder does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(EphemeralError::Io(format!(
                "Not a directory: {}",
                path.display()
            )));
        }

        let scanner = LibraryScanner::new();
        let scan = scanner.scan_directory(path)?;

        let mut tracks_out: Vec<LocalTrack> = Vec::with_capacity(scan.audio_files.len());
        let mut skipped_files: usize = 0;

        let mut inner = self.inner.lock().map_err(|_| EphemeralError::Lock)?;
        inner.reset();

        for audio_file in &scan.audio_files {
            match MetadataExtractor::extract(audio_file) {
                Ok(mut track) => {
                    track.id = inner.next_id;
                    inner.next_id += 1;
                    track.source = Some("ephemeral".to_string());
                    inner.tracks.insert(track.id, track.clone());
                    tracks_out.push(track);
                }
                Err(e) => {
                    log::warn!(
                        "[ephemeral] failed to extract metadata from {}: {}",
                        audio_file.display(),
                        e
                    );
                    skipped_files += 1;
                }
            }
        }

        let folder_path = path.display().to_string();
        inner.current_folder_path = Some(folder_path.clone());

        log::info!(
            "[ephemeral] opened {} ({} tracks, {} skipped)",
            folder_path,
            tracks_out.len(),
            skipped_files
        );

        Ok(EphemeralFolderResult {
            folder_path,
            tracks: tracks_out,
            skipped_files,
        })
    }

    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.reset();
        }
    }

    /// Resolve a synthetic negative id to the cached `LocalTrack`. Returns
    /// `None` if the id is unknown (stale queue entry from a previous
    /// session, race against `clear`, etc.).
    pub fn get_track(&self, id: i64) -> Option<LocalTrack> {
        let inner = self.inner.lock().ok()?;
        inner.tracks.get(&id).cloned()
    }
}

impl Default for EphemeralLibraryState {
    fn default() -> Self {
        Self::new()
    }
}
