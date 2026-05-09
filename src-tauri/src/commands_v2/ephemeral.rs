//! Tauri commands for the ephemeral library.
//!
//! Three commands surface the in-memory ephemeral state to the frontend:
//!   * `v2_ephemeral_open_folder` — scan + extract metadata for every
//!     audio file under a path, replace any prior session, return the
//!     fresh track list.
//!   * `v2_ephemeral_clear` — drop the current session.
//!   * `v2_ephemeral_get_track` — fetch a single ephemeral track by its
//!     synthetic id (used by the player when it sees a negative id and
//!     needs to recover the metadata for queue display, etc.). The hot
//!     playback path resolves ids inline via `EphemeralLibraryState::get_track`,
//!     not through this command.

use std::path::Path;
use tauri::State;

use crate::ephemeral_library::{EphemeralFolderResult, EphemeralLibraryState};
use qbz_library::LocalTrack;

#[tauri::command]
pub async fn v2_ephemeral_open_folder(
    path: String,
    state: State<'_, EphemeralLibraryState>,
) -> Result<EphemeralFolderResult, String> {
    log::info!("Command: v2_ephemeral_open_folder {}", path);
    state
        .open_folder(Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn v2_ephemeral_clear(
    state: State<'_, EphemeralLibraryState>,
) -> Result<(), String> {
    log::info!("Command: v2_ephemeral_clear");
    state.clear();
    Ok(())
}

#[tauri::command]
pub async fn v2_ephemeral_get_track(
    track_id: i64,
    state: State<'_, EphemeralLibraryState>,
) -> Result<Option<LocalTrack>, String> {
    Ok(state.get_track(track_id))
}
