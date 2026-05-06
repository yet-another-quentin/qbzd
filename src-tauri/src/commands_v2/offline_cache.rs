//! V2 cache-management commands. Thin Tauri dispatchers over offline_cache::*.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::core_bridge::CoreBridgeState;
use crate::offline_cache::{
    downloader::spawn_track_cache_download, maintenance, OfflineCacheState, OfflineCacheStatus,
};
use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedAlbumReport {
    pub album_id: String,
    pub removed_track_ids: Vec<u64>,
    pub freed_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedownloadStartedReport {
    pub album_id: String,
    pub queued_track_ids: Vec<u64>,
}

#[tauri::command]
pub async fn v2_remove_cached_album(
    album_id: String,
    cache_state: State<'_, OfflineCacheState>,
) -> Result<RemovedAlbumReport, String> {
    log::info!("Command: v2_remove_cached_album {}", album_id);
    let offline_root_string = cache_state.get_cache_path();
    let offline_root = PathBuf::from(&offline_root_string);
    let guard = cache_state.db.lock().await;
    let db = guard.as_ref().ok_or("No active session - please log in")?;
    let report = maintenance::remove_album_cached_tracks(db, &offline_root, &album_id)?;
    Ok(RemovedAlbumReport {
        album_id: report.album_id,
        removed_track_ids: report.removed_track_ids,
        freed_bytes: report.freed_bytes,
    })
}

#[tauri::command]
pub async fn v2_redownload_cached_album(
    album_id: String,
    failed_only: bool,
    cache_state: State<'_, OfflineCacheState>,
    state: State<'_, AppState>,
    bridge: State<'_, CoreBridgeState>,
    app_handle: tauri::AppHandle,
) -> Result<RedownloadStartedReport, String> {
    log::info!(
        "Command: v2_redownload_cached_album {} (failed_only={})",
        album_id,
        failed_only
    );
    let offline_root_string = cache_state.get_cache_path();

    let queued_ids: Vec<u64> = {
        let guard = cache_state.db.lock().await;
        let db = guard.as_ref().ok_or("No active session - please log in")?;
        let tracks = db.get_album_tracks(&album_id)?;
        let targets = maintenance::select_redownload_targets(&tracks, failed_only);
        let ids: Vec<u64> = targets.iter().map(|track| track.track_id).collect();
        for &id in &ids {
            db.reset_track_for_redownload(id)?;
        }
        ids
    };

    for track_id in &queued_ids {
        let file_path = cache_state.track_file_path(*track_id, "flac");
        spawn_track_cache_download(
            *track_id,
            file_path,
            state.client.clone(),
            bridge.0.clone(),
            cache_state.fetcher.clone(),
            cache_state.db.clone(),
            offline_root_string.clone(),
            cache_state.library_db.clone(),
            app_handle.clone(),
            cache_state.cache_semaphore.clone(),
        );
    }

    Ok(RedownloadStartedReport {
        album_id,
        queued_track_ids: queued_ids,
    })
}

#[tauri::command]
pub async fn v2_redownload_cached_track(
    track_id: u64,
    cache_state: State<'_, OfflineCacheState>,
    state: State<'_, AppState>,
    bridge: State<'_, CoreBridgeState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log::info!("Command: v2_redownload_cached_track {}", track_id);
    let offline_root_string = cache_state.get_cache_path();

    {
        let guard = cache_state.db.lock().await;
        let db = guard.as_ref().ok_or("No active session - please log in")?;
        let track = db
            .get_track(track_id)?
            .ok_or_else(|| format!("Track {} not found in cache", track_id))?;
        // Skip in-flight downloads.
        if matches!(track.status, OfflineCacheStatus::Downloading) {
            return Ok(());
        }
        db.reset_track_for_redownload(track_id)?;
    }

    let file_path = cache_state.track_file_path(track_id, "flac");
    spawn_track_cache_download(
        track_id,
        file_path,
        state.client.clone(),
        bridge.0.clone(),
        cache_state.fetcher.clone(),
        cache_state.db.clone(),
        offline_root_string,
        cache_state.library_db.clone(),
        app_handle,
        cache_state.cache_semaphore.clone(),
    );

    Ok(())
}
