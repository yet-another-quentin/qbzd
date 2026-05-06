//! Maintenance operations on the offline cache: bulk delete and re-download.
//! Pure logic — no Tauri state. Callable from any future TUI or headless binary.

use std::path::Path;

use super::cmaf_store::BundleLayout;
use super::{CachedTrackInfo, OfflineCacheDb, OfflineCacheStatus};

#[derive(Debug, Clone)]
pub struct AlbumRemovalReport {
    pub album_id: String,
    pub removed_track_ids: Vec<u64>,
    pub freed_bytes: u64,
}

/// Removes all cached tracks of an album: SQLite rows + on-disk CMAF bundles.
/// Filesystem errors per-track are logged and not propagated; SQLite is the
/// source of truth and the bundle directories are best-effort cleanup.
pub fn remove_album_cached_tracks(
    db: &OfflineCacheDb,
    offline_root: &Path,
    album_id: &str,
) -> Result<AlbumRemovalReport, String> {
    let (ids, bytes) = db.delete_album_tracks(album_id)?;
    for &track_id in &ids {
        let layout = BundleLayout::new(offline_root, track_id);
        if layout.track_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&layout.track_dir) {
                log::warn!(
                    "Failed to remove CMAF dir for track {}: {} (continuing)",
                    track_id,
                    e
                );
            }
        }
    }
    Ok(AlbumRemovalReport {
        album_id: album_id.to_string(),
        removed_track_ids: ids,
        freed_bytes: bytes,
    })
}

/// Filters tracks targeted by re-download: skip in-flight Downloading,
/// optionally restrict to Failed only.
pub fn select_redownload_targets(
    tracks: &[CachedTrackInfo],
    failed_only: bool,
) -> Vec<&CachedTrackInfo> {
    tracks
        .iter()
        .filter(|track| match track.status {
            OfflineCacheStatus::Downloading => false,
            OfflineCacheStatus::Failed => true,
            _ => !failed_only,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offline_cache::OfflineCacheStatus;

    fn track_with_status(id: u64, status: OfflineCacheStatus) -> CachedTrackInfo {
        CachedTrackInfo {
            track_id: id,
            title: format!("t{}", id),
            artist: "A".into(),
            album: None,
            album_id: None,
            duration_secs: 0,
            file_size_bytes: 0,
            quality: "lossless".into(),
            bit_depth: None,
            sample_rate: None,
            status,
            progress_percent: 0,
            error_message: None,
            created_at: "".into(),
            last_accessed_at: "".into(),
        }
    }

    #[test]
    fn redownload_targets_full_skips_only_downloading() {
        let tracks = vec![
            track_with_status(1, OfflineCacheStatus::Ready),
            track_with_status(2, OfflineCacheStatus::Downloading),
            track_with_status(3, OfflineCacheStatus::Failed),
            track_with_status(4, OfflineCacheStatus::Queued),
        ];
        let picked = select_redownload_targets(&tracks, false);
        let ids: Vec<u64> = picked.iter().map(|track| track.track_id).collect();
        assert_eq!(ids, vec![1, 3, 4]);
    }

    #[test]
    fn redownload_targets_failed_only_returns_failed() {
        let tracks = vec![
            track_with_status(1, OfflineCacheStatus::Ready),
            track_with_status(2, OfflineCacheStatus::Failed),
            track_with_status(3, OfflineCacheStatus::Downloading),
        ];
        let picked = select_redownload_targets(&tracks, true);
        let ids: Vec<u64> = picked.iter().map(|track| track.track_id).collect();
        assert_eq!(ids, vec![2]);
    }
}
