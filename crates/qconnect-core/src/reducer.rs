use std::collections::{HashMap, HashSet};

use crate::queue::{QConnectQueueState, QueueEvent, QueueItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducerOutcome {
    pub queue_changed: bool,
    pub version_changed: bool,
    pub event_name: &'static str,
}

impl ReducerOutcome {
    const fn unchanged(event_name: &'static str) -> Self {
        Self {
            queue_changed: false,
            version_changed: false,
            event_name,
        }
    }

    const fn changed(event_name: &'static str, version_changed: bool) -> Self {
        Self {
            queue_changed: true,
            version_changed,
            event_name,
        }
    }
}

pub fn apply_event(
    state: &mut QConnectQueueState,
    event: &QueueEvent,
    now_ms: u64,
) -> ReducerOutcome {
    match event {
        QueueEvent::QueueStateReplaced { state: next, .. } => {
            *state = next.clone();
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_QUEUE_STATE", true)
        }
        QueueEvent::TracksAdded {
            version,
            tracks,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            let start_idx = state.queue_items.len();
            state.queue_items.extend(tracks.iter().cloned());

            if state.shuffle_mode {
                if let Some(order) = state.shuffle_order.as_mut() {
                    for idx in start_idx..state.queue_items.len() {
                        order.push(idx);
                    }
                }
            }

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_QUEUE_TRACKS_ADDED", version_changed)
        }
        QueueEvent::TracksLoaded {
            version,
            tracks,
            shuffle_mode,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            state.queue_items = tracks.clone();

            if let Some(enabled) = shuffle_mode {
                state.shuffle_mode = *enabled;
            }

            state.shuffle_order = None;

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_QUEUE_TRACKS_LOADED", version_changed)
        }
        QueueEvent::TracksInserted {
            version,
            tracks,
            insert_after,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            let insert_index = insertion_index(&state.queue_items, *insert_after);
            let insert_count = tracks.len();
            let mut next_queue =
                Vec::with_capacity(state.queue_items.len().saturating_add(insert_count));
            next_queue.extend_from_slice(&state.queue_items[..insert_index]);
            next_queue.extend(tracks.iter().cloned());
            next_queue.extend_from_slice(&state.queue_items[insert_index..]);
            state.queue_items = next_queue;

            if state.shuffle_mode {
                if let Some(order) = state.shuffle_order.as_mut() {
                    for idx in order.iter_mut() {
                        if *idx >= insert_index {
                            *idx += insert_count;
                        }
                    }
                    for idx in insert_index..insert_index + insert_count {
                        order.push(idx);
                    }
                }
            }

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_QUEUE_TRACKS_INSERTED", version_changed)
        }
        QueueEvent::TracksRemoved {
            version,
            queue_item_ids,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            let before = state.queue_items.clone();
            let to_remove: HashSet<u64> = queue_item_ids.iter().copied().collect();
            let old_len = before.len();

            let mut old_to_new = vec![None; old_len];
            let mut next_queue = Vec::with_capacity(old_len);
            for (old_idx, item) in before.iter().cloned().enumerate() {
                if !to_remove.contains(&item.queue_item_id) {
                    old_to_new[old_idx] = Some(next_queue.len());
                    next_queue.push(item);
                }
            }

            let removed = old_len != next_queue.len();
            state.queue_items = next_queue;

            if let Some(order) = state.shuffle_order.as_mut() {
                let mapped: Vec<usize> = order
                    .iter()
                    .filter_map(|old_idx| old_to_new.get(*old_idx).and_then(|value| *value))
                    .collect();
                *order = mapped;
            }

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;

            if removed {
                ReducerOutcome::changed("SRVR_CTRL_QUEUE_TRACKS_REMOVED", version_changed)
            } else {
                ReducerOutcome::unchanged("SRVR_CTRL_QUEUE_TRACKS_REMOVED")
            }
        }
        QueueEvent::TracksReordered {
            version,
            queue_item_ids,
            insert_after,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            let before = state.queue_items.clone();
            let to_move: HashSet<u64> = queue_item_ids.iter().copied().collect();
            let mut by_id: HashMap<u64, QueueItem> = before
                .iter()
                .cloned()
                .map(|item| (item.queue_item_id, item))
                .collect();

            let moving: Vec<QueueItem> = queue_item_ids
                .iter()
                .filter_map(|id| by_id.remove(id))
                .collect();
            let mut remaining: Vec<QueueItem> = before
                .iter()
                .filter(|item| !to_move.contains(&item.queue_item_id))
                .cloned()
                .collect();
            let insert_index = insertion_index(&remaining, *insert_after);

            let reordered = !moving.is_empty();
            if reordered {
                remaining.splice(insert_index..insert_index, moving);
                state.queue_items = remaining;

                if let Some(order) = state.shuffle_order.as_mut() {
                    let id_by_old_index: HashMap<usize, u64> = before
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| (idx, item.queue_item_id))
                        .collect();
                    let new_index_by_id: HashMap<u64, usize> = state
                        .queue_items
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| (item.queue_item_id, idx))
                        .collect();

                    let mapped: Vec<usize> = order
                        .iter()
                        .filter_map(|old_idx| id_by_old_index.get(old_idx))
                        .filter_map(|id| new_index_by_id.get(id))
                        .copied()
                        .collect();
                    *order = mapped;
                }
            }

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;

            if reordered {
                ReducerOutcome::changed("SRVR_CTRL_QUEUE_TRACKS_REORDERED", version_changed)
            } else {
                ReducerOutcome::unchanged("SRVR_CTRL_QUEUE_TRACKS_REORDERED")
            }
        }
        QueueEvent::QueueCleared { version, .. } => {
            state.queue_items.clear();
            state.autoplay_items.clear();
            state.shuffle_mode = false;
            state.shuffle_order = None;
            state.autoplay_loading = false;
            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_QUEUE_CLEARED", version_changed)
        }
        QueueEvent::ShuffleModeSet {
            version,
            shuffle_mode,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            state.shuffle_mode = *shuffle_mode;
            if !*shuffle_mode {
                state.shuffle_order = None;
            }

            if *autoplay_reset {
                state.autoplay_items.clear();
            }
            state.autoplay_loading = *autoplay_loading;

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_SHUFFLE_MODE_SET", version_changed)
        }
        QueueEvent::AutoplayModeSet {
            version,
            autoplay_mode,
            autoplay_reset,
            autoplay_loading,
            ..
        } => {
            state.autoplay_mode = *autoplay_mode;
            state.autoplay_loading = *autoplay_loading;
            if *autoplay_reset {
                state.autoplay_items.clear();
            }

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_AUTOPLAY_MODE_SET", version_changed)
        }
        QueueEvent::AutoplayTracksLoaded {
            version, tracks, ..
        } => {
            state.autoplay_items = tracks.clone();
            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;
            ReducerOutcome::changed("SRVR_CTRL_AUTOPLAY_TRACKS_LOADED", version_changed)
        }
        QueueEvent::AutoplayTracksRemoved {
            version,
            queue_item_ids,
            ..
        } => {
            let to_remove: HashSet<u64> = queue_item_ids.iter().copied().collect();
            let before = state.autoplay_items.len();
            state
                .autoplay_items
                .retain(|track| !to_remove.contains(&track.queue_item_id));
            let changed = before != state.autoplay_items.len();

            let version_changed = state.version != *version;
            state.version = *version;
            state.updated_at_ms = now_ms;

            if changed {
                ReducerOutcome::changed("SRVR_CTRL_AUTOPLAY_TRACKS_REMOVED", version_changed)
            } else {
                ReducerOutcome::unchanged("SRVR_CTRL_AUTOPLAY_TRACKS_REMOVED")
            }
        }
        QueueEvent::QueueError { version, .. } => {
            if let Some(version) = version {
                let version_changed = state.version != *version;
                state.version = *version;
                state.updated_at_ms = now_ms;
                return ReducerOutcome {
                    queue_changed: false,
                    version_changed,
                    event_name: "SRVR_CTRL_QUEUE_ERROR_MESSAGE",
                };
            }
            ReducerOutcome::unchanged("SRVR_CTRL_QUEUE_ERROR_MESSAGE")
        }
    }
}

fn insertion_index(items: &[QueueItem], insert_after: Option<u64>) -> usize {
    insert_after
        .and_then(|queue_item_id| {
            items
                .iter()
                .position(|item| item.queue_item_id == queue_item_id)
                .map(|idx| idx + 1)
        })
        .unwrap_or(items.len())
}

pub fn build_shuffle_order(count: usize, seed: u64, pivot_index: Option<usize>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    if count <= 1 {
        return order;
    }

    let mut rng = if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    };

    for idx in (1..count).rev() {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let swap_idx = (rng % ((idx + 1) as u64)) as usize;
        order.swap(idx, swap_idx);
    }

    if let Some(pivot) = pivot_index.filter(|value| *value < count) {
        if let Some(pos) = order.iter().position(|value| *value == pivot) {
            order.rotate_left(pos);
        }
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{QueueEvent, QueueVersion};

    fn item(id: u64) -> QueueItem {
        QueueItem {
            track_context_uuid: "ctx".to_string(),
            track_id: id,
            queue_item_id: id,
        }
    }

    #[test]
    fn tracks_loaded_replaces_queue_without_deriving_shuffle_order() {
        let mut state = QConnectQueueState::default();
        let event = QueueEvent::TracksLoaded {
            action_uuid: None,
            version: QueueVersion::new(1, 1),
            tracks: vec![item(10), item(20), item(30)],
            queue_position: Some(1),
            shuffle_mode: Some(true),
            shuffle_seed: Some(42),
            shuffle_pivot_queue_item_id: Some(20),
            autoplay_reset: true,
            autoplay_loading: true,
        };

        let outcome = apply_event(&mut state, &event, 1234);
        assert!(outcome.queue_changed);
        assert_eq!(state.queue_items.len(), 3);
        assert!(state.shuffle_mode);
        assert_eq!(state.shuffle_order, None);
        assert!(state.autoplay_loading);
    }

    #[test]
    fn shuffle_mode_set_keeps_existing_order_until_authoritative_queue_arrives() {
        let mut state = QConnectQueueState {
            queue_items: vec![item(1), item(2), item(3)],
            shuffle_mode: false,
            shuffle_order: Some(vec![0, 2, 1]),
            ..Default::default()
        };

        let event = QueueEvent::ShuffleModeSet {
            action_uuid: None,
            version: QueueVersion::new(1, 2),
            shuffle_mode: true,
            shuffle_seed: Some(42),
            shuffle_pivot_queue_item_id: Some(1),
            autoplay_reset: false,
            autoplay_loading: false,
        };

        let outcome = apply_event(&mut state, &event, 2345);
        assert!(outcome.queue_changed);
        assert!(state.shuffle_mode);
        assert_eq!(state.shuffle_order, Some(vec![0, 2, 1]));
    }

    #[test]
    fn tracks_removed_updates_queue_and_shuffle() {
        let mut state = QConnectQueueState {
            queue_items: vec![item(1), item(2), item(3)],
            shuffle_mode: true,
            shuffle_order: Some(vec![2, 1, 0]),
            ..Default::default()
        };

        let event = QueueEvent::TracksRemoved {
            action_uuid: None,
            version: QueueVersion::new(2, 0),
            queue_item_ids: vec![2],
            autoplay_reset: false,
            autoplay_loading: false,
        };

        let outcome = apply_event(&mut state, &event, 22);
        assert!(outcome.queue_changed);
        assert_eq!(state.queue_items.len(), 2);
        assert_eq!(state.queue_items[0].queue_item_id, 1);
        assert_eq!(state.queue_items[1].queue_item_id, 3);
        assert_eq!(state.shuffle_order, Some(vec![1, 0]));
    }

    #[test]
    fn tracks_reordered_moves_subset() {
        let mut state = QConnectQueueState {
            queue_items: vec![item(1), item(2), item(3), item(4)],
            shuffle_mode: true,
            shuffle_order: Some(vec![0, 1, 2, 3]),
            ..Default::default()
        };

        let event = QueueEvent::TracksReordered {
            action_uuid: None,
            version: QueueVersion::new(3, 0),
            queue_item_ids: vec![4],
            insert_after: Some(1),
            autoplay_reset: false,
            autoplay_loading: false,
        };

        let outcome = apply_event(&mut state, &event, 33);
        assert!(outcome.queue_changed);
        assert_eq!(
            state
                .queue_items
                .iter()
                .map(|entry| entry.queue_item_id)
                .collect::<Vec<_>>(),
            vec![1, 4, 2, 3]
        );
    }
}
