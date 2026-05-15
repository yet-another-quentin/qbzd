use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QueueVersion {
    pub major: u64,
    pub minor: u64,
}

impl QueueVersion {
    pub const fn new(major: u64, minor: u64) -> Self {
        Self { major, minor }
    }

    pub const fn next_minor(self) -> Self {
        Self {
            major: self.major,
            minor: self.minor.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub track_context_uuid: String,
    pub track_id: u64,
    pub queue_item_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QConnectQueueState {
    pub version: QueueVersion,
    pub queue_items: Vec<QueueItem>,
    pub shuffle_mode: bool,
    pub shuffle_order: Option<Vec<usize>>,
    pub autoplay_mode: bool,
    pub autoplay_loading: bool,
    pub autoplay_items: Vec<QueueItem>,
    pub updated_at_ms: u64,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueEvent {
    QueueStateReplaced {
        action_uuid: Option<String>,
        state: QConnectQueueState,
    },
    TracksAdded {
        action_uuid: Option<String>,
        version: QueueVersion,
        tracks: Vec<QueueItem>,
        shuffle_seed: Option<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    TracksLoaded {
        action_uuid: Option<String>,
        version: QueueVersion,
        tracks: Vec<QueueItem>,
        queue_position: Option<u64>,
        shuffle_mode: Option<bool>,
        shuffle_seed: Option<u64>,
        shuffle_pivot_queue_item_id: Option<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    TracksInserted {
        action_uuid: Option<String>,
        version: QueueVersion,
        tracks: Vec<QueueItem>,
        insert_after: Option<u64>,
        shuffle_seed: Option<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    TracksRemoved {
        action_uuid: Option<String>,
        version: QueueVersion,
        queue_item_ids: Vec<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    TracksReordered {
        action_uuid: Option<String>,
        version: QueueVersion,
        queue_item_ids: Vec<u64>,
        insert_after: Option<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    QueueCleared {
        action_uuid: Option<String>,
        version: QueueVersion,
    },
    ShuffleModeSet {
        action_uuid: Option<String>,
        version: QueueVersion,
        shuffle_mode: bool,
        shuffle_seed: Option<u64>,
        shuffle_pivot_queue_item_id: Option<u64>,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    AutoplayModeSet {
        action_uuid: Option<String>,
        version: QueueVersion,
        autoplay_mode: bool,
        autoplay_reset: bool,
        autoplay_loading: bool,
    },
    AutoplayTracksLoaded {
        action_uuid: Option<String>,
        version: QueueVersion,
        tracks: Vec<QueueItem>,
    },
    AutoplayTracksRemoved {
        action_uuid: Option<String>,
        version: QueueVersion,
        queue_item_ids: Vec<u64>,
    },
    QueueError {
        action_uuid: Option<String>,
        version: Option<QueueVersion>,
        code: String,
        message: String,
    },
}
