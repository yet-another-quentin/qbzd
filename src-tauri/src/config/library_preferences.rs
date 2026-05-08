use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldersViewMode {
    Flat,
    Tree,
}

impl Default for FoldersViewMode {
    fn default() -> Self {
        FoldersViewMode::Flat
    }
}

impl FoldersViewMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            FoldersViewMode::Flat => "flat",
            FoldersViewMode::Tree => "tree",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tree" => FoldersViewMode::Tree,
            _ => FoldersViewMode::Flat,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryPreferences {
    pub tab_order: Vec<String>,
    pub hidden_tabs: Vec<String>,
    #[serde(default)]
    pub folders_view_mode: FoldersViewMode,
}

impl Default for LibraryPreferences {
    fn default() -> Self {
        Self {
            // Default order matches the current LocalLibraryView tab list,
            // with the renamed 'folders' tab in place of the old 'albums'
            // tab. Phase 3 adds 'albums' (the new metadata-grouped view)
            // to this default.
            tab_order: vec![
                "tracks".to_string(),
                "folders".to_string(),
                "albums".to_string(),
                "artists".to_string(),
            ],
            hidden_tabs: vec![],
            folders_view_mode: FoldersViewMode::Flat,
        }
    }
}

pub struct LibraryPreferencesStore {
    conn: Connection,
}

impl LibraryPreferencesStore {
    fn open_at(dir: &Path, db_name: &str) -> Result<Self, String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let db_path = dir.join(db_name);
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open library preferences database: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| format!("Failed to set WAL mode on library preferences: {}", e))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS library_preferences (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                tab_order TEXT NOT NULL,
                hidden_tabs TEXT NOT NULL DEFAULT '[]'
            )",
            [],
        )
        .map_err(|e| format!("Failed to create library preferences table: {}", e))?;

        // Idempotent migrations for columns added after the initial schema.
        // These ALTERs fail when the column already exists; that failure is
        // expected and ignored.
        let _ = conn.execute(
            "ALTER TABLE library_preferences ADD COLUMN folders_view_mode TEXT DEFAULT 'flat'",
            [],
        );

        Ok(Self { conn })
    }

    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("qbz");
        Self::open_at(&data_dir, "library_preferences.db")
    }

    pub fn new_at(base_dir: &Path) -> Result<Self, String> {
        Self::open_at(base_dir, "library_preferences.db")
    }

    pub fn get_preferences(&self) -> Result<LibraryPreferences, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT tab_order, hidden_tabs, folders_view_mode \
                 FROM library_preferences WHERE id = 1",
            )
            .map_err(|e| format!("Failed to prepare select: {}", e))?;

        let result = stmt.query_row([], |row| {
            let tab_order_str: String = row.get(0)?;
            let hidden_tabs_str: String = row.get(1)?;
            let folders_view_mode_str: Option<String> = row.get(2)?;

            let tab_order: Vec<String> = serde_json::from_str(&tab_order_str)
                .unwrap_or_else(|_| LibraryPreferences::default().tab_order);
            let hidden_tabs: Vec<String> =
                serde_json::from_str(&hidden_tabs_str).unwrap_or_default();
            let folders_view_mode = folders_view_mode_str
                .map(|s| FoldersViewMode::from_str(&s))
                .unwrap_or_default();

            Ok(LibraryPreferences {
                tab_order,
                hidden_tabs,
                folders_view_mode,
            })
        });

        match result {
            Ok(prefs) => Ok(prefs),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(LibraryPreferences::default()),
            Err(e) => Err(format!("Failed to query library preferences: {}", e)),
        }
    }

    pub fn save_preferences(
        &self,
        prefs: LibraryPreferences,
    ) -> Result<LibraryPreferences, String> {
        let tab_order_str = serde_json::to_string(&prefs.tab_order)
            .map_err(|e| format!("Failed to serialize tab_order: {}", e))?;
        let hidden_tabs_str = serde_json::to_string(&prefs.hidden_tabs)
            .map_err(|e| format!("Failed to serialize hidden_tabs: {}", e))?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO library_preferences \
                 (id, tab_order, hidden_tabs, folders_view_mode) \
                 VALUES (1, ?1, ?2, ?3)",
                params![
                    tab_order_str,
                    hidden_tabs_str,
                    prefs.folders_view_mode.as_str()
                ],
            )
            .map_err(|e| format!("Failed to save library preferences: {}", e))?;
        Ok(prefs)
    }

    pub fn set_folders_view_mode(&self, mode: FoldersViewMode) -> Result<(), String> {
        // Ensure the row exists before updating; if no preferences have been
        // saved yet, fall back to inserting a default row carrying the new
        // folders_view_mode value.
        let updated = self
            .conn
            .execute(
                "UPDATE library_preferences SET folders_view_mode = ?1 WHERE id = 1",
                params![mode.as_str()],
            )
            .map_err(|e| format!("Failed to set folders_view_mode: {}", e))?;

        if updated == 0 {
            let mut prefs = LibraryPreferences::default();
            prefs.folders_view_mode = mode;
            self.save_preferences(prefs)?;
        }

        Ok(())
    }
}

pub struct LibraryPreferencesState {
    pub store: Arc<Mutex<Option<LibraryPreferencesStore>>>,
}

impl LibraryPreferencesState {
    pub fn new() -> Result<Self, String> {
        let store = LibraryPreferencesStore::new()?;
        Ok(Self {
            store: Arc::new(Mutex::new(Some(store))),
        })
    }

    pub fn new_empty() -> Self {
        Self {
            store: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init_at(&self, base_dir: &Path) -> Result<(), String> {
        let new_store = LibraryPreferencesStore::new_at(base_dir)?;
        let mut guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock library preferences store".to_string())?;
        *guard = Some(new_store);
        Ok(())
    }

    pub fn teardown(&self) -> Result<(), String> {
        let mut guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock library preferences store".to_string())?;
        *guard = None;
        Ok(())
    }
}

#[tauri::command]
pub fn get_library_preferences(
    state: tauri::State<LibraryPreferencesState>,
) -> Result<LibraryPreferences, String> {
    let guard = state
        .store
        .lock()
        .map_err(|_| "Failed to lock library preferences store".to_string())?;
    let store = guard.as_ref().ok_or("No active session - please log in")?;
    store.get_preferences()
}

#[tauri::command]
pub fn save_library_preferences(
    prefs: LibraryPreferences,
    state: tauri::State<LibraryPreferencesState>,
) -> Result<LibraryPreferences, String> {
    let guard = state
        .store
        .lock()
        .map_err(|_| "Failed to lock library preferences store".to_string())?;
    let store = guard.as_ref().ok_or("No active session - please log in")?;
    store.save_preferences(prefs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_store() -> (tempfile::TempDir, LibraryPreferencesStore) {
        let dir = tempdir().expect("tempdir");
        let store =
            LibraryPreferencesStore::new_at(dir.path()).expect("open store in temp dir");
        (dir, store)
    }

    #[test]
    fn default_is_flat() {
        let prefs = LibraryPreferences::default();
        assert_eq!(prefs.folders_view_mode, FoldersViewMode::Flat);
    }

    #[test]
    fn legacy_json_without_folders_view_mode_defaults_to_flat() {
        let legacy = r#"{
            "tab_order": ["tracks", "folders", "albums", "artists"],
            "hidden_tabs": []
        }"#;
        let prefs: LibraryPreferences =
            serde_json::from_str(legacy).expect("parse legacy json");
        assert_eq!(prefs.folders_view_mode, FoldersViewMode::Flat);
    }

    #[test]
    fn round_trip_with_tree_set() {
        let mut prefs = LibraryPreferences::default();
        prefs.folders_view_mode = FoldersViewMode::Tree;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let parsed: LibraryPreferences =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.folders_view_mode, FoldersViewMode::Tree);
    }

    #[test]
    fn folders_view_mode_str_round_trip() {
        assert_eq!(FoldersViewMode::Flat.as_str(), "flat");
        assert_eq!(FoldersViewMode::Tree.as_str(), "tree");
        assert_eq!(FoldersViewMode::from_str("flat"), FoldersViewMode::Flat);
        assert_eq!(FoldersViewMode::from_str("tree"), FoldersViewMode::Tree);
        assert_eq!(
            FoldersViewMode::from_str("garbage"),
            FoldersViewMode::Flat
        );
    }

    #[test]
    fn fresh_store_returns_default_folders_view_mode() {
        let (_dir, store) = fresh_store();
        let prefs = store.get_preferences().expect("get prefs");
        assert_eq!(prefs.folders_view_mode, FoldersViewMode::Flat);
    }

    #[test]
    fn set_folders_view_mode_persists_on_empty_table() {
        let (_dir, store) = fresh_store();
        store
            .set_folders_view_mode(FoldersViewMode::Tree)
            .expect("set mode");
        let prefs = store.get_preferences().expect("get prefs");
        assert_eq!(prefs.folders_view_mode, FoldersViewMode::Tree);
    }

    #[test]
    fn set_folders_view_mode_updates_existing_row() {
        let (_dir, store) = fresh_store();

        // Seed a row with the default save path so the UPDATE branch runs.
        let mut seeded = LibraryPreferences::default();
        seeded.tab_order = vec!["tracks".into(), "albums".into()];
        store.save_preferences(seeded).expect("save seed");

        store
            .set_folders_view_mode(FoldersViewMode::Tree)
            .expect("set mode");
        let after = store.get_preferences().expect("get prefs");
        assert_eq!(after.folders_view_mode, FoldersViewMode::Tree);
        // Sibling fields preserved.
        assert_eq!(
            after.tab_order,
            vec!["tracks".to_string(), "albums".to_string()]
        );
    }

    #[test]
    fn save_preferences_persists_folders_view_mode() {
        let (_dir, store) = fresh_store();
        let mut prefs = LibraryPreferences::default();
        prefs.folders_view_mode = FoldersViewMode::Tree;
        store.save_preferences(prefs).expect("save");

        let loaded = store.get_preferences().expect("load");
        assert_eq!(loaded.folders_view_mode, FoldersViewMode::Tree);
    }
}
