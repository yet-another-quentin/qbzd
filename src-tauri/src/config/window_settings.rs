//! Window decoration settings
//!
//! Stores user preferences for window title bar behavior:
//! - use_system_titlebar: Use OS native window decorations instead of custom CSD title bar

use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Titlebar rendering mode. Controls which decoration source is active and
/// how the custom titlebar component renders (or whether it renders at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitlebarMode {
    /// Default — full custom titlebar with drag, resize, dblclick, controls.
    Qbz,
    /// OS-native chrome (CSD on Wayland, KWin SSD on X11, NSWindow on macOS).
    System,
    /// KDE-only — KWin SSD via Xwayland + stripped custom strip below for search+nav.
    Plasma,
    /// No titlebar — for tiling WMs (i3/Hyprland/sway/niri).
    Hidden,
}

impl TitlebarMode {
    /// Whether this mode requests OS decorations (decorations=true at window creation).
    pub fn wants_decorations(self) -> bool {
        matches!(self, TitlebarMode::System | TitlebarMode::Plasma)
    }

    /// Whether this mode requires GDK_BACKEND=x11 to coerce SSD on Wayland.
    pub fn wants_xwayland(self) -> bool {
        matches!(self, TitlebarMode::Plasma)
    }
}

impl Default for TitlebarMode {
    fn default() -> Self {
        TitlebarMode::Qbz
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    /// Use OS native window decorations (legacy — superseded by `titlebar_mode`).
    /// Kept as a write-through shadow of `titlebar_mode in {System, Plasma}` for
    /// any consumers that still read it; will be removed in a follow-up.
    pub use_system_titlebar: bool,
    /// Last saved window width (logical pixels, non-maximized)
    pub window_width: f64,
    /// Last saved window height (logical pixels, non-maximized)
    pub window_height: f64,
    /// Whether the window was maximized when last closed
    pub is_maximized: bool,
    /// Match the active desktop's window chrome (rounded corners, edge).
    /// Implies building the window transparent on Linux so the corners
    /// can be seen through to the desktop. Requires restart to take effect.
    pub match_system_window_chrome: bool,
    /// Authoritative titlebar mode (replaces `use_system_titlebar`).
    pub titlebar_mode: TitlebarMode,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            use_system_titlebar: false,
            window_width: 1280.0,
            window_height: 800.0,
            is_maximized: false,
            match_system_window_chrome: false,
            titlebar_mode: TitlebarMode::Qbz,
        }
    }
}

pub struct WindowSettingsStore {
    conn: Connection,
}

impl WindowSettingsStore {
    fn open_at(dir: &Path, db_name: &str) -> Result<Self, String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let db_path = dir.join(db_name);
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open window settings database: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| format!("Failed to enable WAL for window settings database: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS window_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                use_system_titlebar INTEGER NOT NULL DEFAULT 0
            );",
        )
        .map_err(|e| format!("Failed to create window settings table: {}", e))?;

        conn.execute(
            "INSERT OR IGNORE INTO window_settings (id, use_system_titlebar)
            VALUES (1, 0)",
            [],
        )
        .map_err(|e| format!("Failed to insert default window settings: {}", e))?;

        // Migrations: add window size columns on existing DBs (errors ignored when column exists)
        let _ = conn.execute(
            "ALTER TABLE window_settings ADD COLUMN window_width REAL NOT NULL DEFAULT 1280.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE window_settings ADD COLUMN window_height REAL NOT NULL DEFAULT 800.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE window_settings ADD COLUMN is_maximized INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE window_settings ADD COLUMN match_system_window_chrome INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Add titlebar_mode column. Defaults to NULL so we can detect
        // pre-existing rows and migrate from use_system_titlebar.
        let _ = conn.execute(
            "ALTER TABLE window_settings ADD COLUMN titlebar_mode TEXT",
            [],
        );

        // One-time migration: if titlebar_mode is NULL but use_system_titlebar
        // exists, derive mode. This runs at every open but only does work on
        // rows where titlebar_mode is still NULL (idempotent).
        let _ = conn.execute(
            "UPDATE window_settings
             SET titlebar_mode = CASE
                 WHEN use_system_titlebar = 1 THEN 'system'
                 ELSE 'qbz'
             END
             WHERE titlebar_mode IS NULL AND id = 1",
            [],
        );

        info!("[WindowSettings] Database initialized");

        Ok(Self { conn })
    }

    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("qbz");
        Self::open_at(&data_dir, "window_settings.db")
    }

    pub fn new_at(base_dir: &Path) -> Result<Self, String> {
        Self::open_at(base_dir, "window_settings.db")
    }

    /// Lightweight read-only open for startup (before Tauri state).
    /// Opens existing DB without creating tables/running migrations.
    pub fn new_readonly() -> Result<Self, String> {
        let db_path = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("qbz")
            .join("window_settings.db");

        if !db_path.exists() {
            return Err("Window settings DB does not exist yet".to_string());
        }

        let conn = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("Failed to open window settings database (readonly): {}", e))?;

        Ok(Self { conn })
    }

    pub fn get_settings(&self) -> Result<WindowSettings, String> {
        self.conn
            .query_row(
                "SELECT use_system_titlebar, window_width, window_height, is_maximized,
                        match_system_window_chrome, titlebar_mode
                 FROM window_settings WHERE id = 1",
                [],
                |row| {
                    let use_system_titlebar: i32 = row.get(0)?;
                    let window_width: f64 = row.get(1)?;
                    let window_height: f64 = row.get(2)?;
                    let is_maximized: i32 = row.get(3)?;
                    let match_chrome: i32 = row.get(4)?;
                    let mode_str: Option<String> = row.get(5)?;
                    let titlebar_mode = match mode_str.as_deref() {
                        Some("system") => TitlebarMode::System,
                        Some("plasma") => TitlebarMode::Plasma,
                        Some("hidden") => TitlebarMode::Hidden,
                        _ => TitlebarMode::Qbz,
                    };
                    let defaults = WindowSettings::default();
                    let (w, h) = if is_valid_window_size(window_width, window_height) {
                        (window_width, window_height)
                    } else {
                        log::warn!(
                            "[WindowSettings] Corrupt size in DB: {}x{}, using defaults",
                            window_width,
                            window_height
                        );
                        (defaults.window_width, defaults.window_height)
                    };
                    Ok(WindowSettings {
                        use_system_titlebar: use_system_titlebar != 0,
                        window_width: w,
                        window_height: h,
                        is_maximized: is_maximized != 0,
                        match_system_window_chrome: match_chrome != 0,
                        titlebar_mode,
                    })
                },
            )
            .map_err(|e| format!("Failed to get window settings: {}", e))
    }

    pub fn set_match_system_window_chrome(&self, value: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE window_settings SET match_system_window_chrome = ?1 WHERE id = 1",
                params![if value { 1 } else { 0 }],
            )
            .map_err(|e| format!("Failed to set match_system_window_chrome: {}", e))?;
        Ok(())
    }

    pub fn set_use_system_titlebar(&self, value: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE window_settings SET use_system_titlebar = ?1 WHERE id = 1",
                params![if value { 1 } else { 0 }],
            )
            .map_err(|e| format!("Failed to set use_system_titlebar: {}", e))?;
        Ok(())
    }

    pub fn set_titlebar_mode(&self, mode: TitlebarMode) -> Result<(), String> {
        let mode_str = match mode {
            TitlebarMode::Qbz => "qbz",
            TitlebarMode::System => "system",
            TitlebarMode::Plasma => "plasma",
            TitlebarMode::Hidden => "hidden",
        };
        let use_system = mode.wants_decorations() as i64;
        self.conn
            .execute(
                "UPDATE window_settings
                 SET titlebar_mode = ?1, use_system_titlebar = ?2
                 WHERE id = 1",
                params![mode_str, use_system],
            )
            .map_err(|e| format!("Failed to set titlebar_mode: {}", e))?;
        info!("Window titlebar_mode set to {}", mode_str);
        Ok(())
    }

    /// Save the non-maximized window dimensions (called on resize while not maximized).
    pub fn set_window_size(&self, width: f64, height: f64) -> Result<(), String> {
        if !is_valid_window_size(width, height) {
            log::warn!(
                "[WindowSettings] Ignoring invalid window size: {}x{}",
                width,
                height
            );
            return Ok(());
        }
        self.conn
            .execute(
                "UPDATE window_settings SET window_width = ?1, window_height = ?2 WHERE id = 1",
                params![width, height],
            )
            .map_err(|e| format!("Failed to save window size: {}", e))?;
        Ok(())
    }

    /// Save the maximized state (called on window close).
    pub fn set_is_maximized(&self, value: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE window_settings SET is_maximized = ?1 WHERE id = 1",
                params![if value { 1 } else { 0 }],
            )
            .map_err(|e| format!("Failed to save maximized state: {}", e))?;
        Ok(())
    }
}

/// Global state wrapper for thread-safe access
pub struct WindowSettingsState {
    pub store: Arc<Mutex<Option<WindowSettingsStore>>>,
}

impl WindowSettingsState {
    pub fn new() -> Result<Self, String> {
        let store = WindowSettingsStore::new()?;
        Ok(Self {
            store: Arc::new(Mutex::new(Some(store))),
        })
    }

    pub fn new_empty() -> Self {
        Self {
            store: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_settings(&self) -> Result<WindowSettings, String> {
        let guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock window settings store".to_string())?;
        let store = guard
            .as_ref()
            .ok_or("Window settings store not initialized")?;
        store.get_settings()
    }

    pub fn set_use_system_titlebar(&self, value: bool) -> Result<(), String> {
        let guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock window settings store".to_string())?;
        let store = guard
            .as_ref()
            .ok_or("Window settings store not initialized")?;
        store.set_use_system_titlebar(value)
    }

    pub fn set_window_size(&self, width: f64, height: f64) -> Result<(), String> {
        let guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock window settings store".to_string())?;
        let store = guard
            .as_ref()
            .ok_or("Window settings store not initialized")?;
        store.set_window_size(width, height)
    }

    pub fn set_is_maximized(&self, value: bool) -> Result<(), String> {
        let guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock window settings store".to_string())?;
        let store = guard
            .as_ref()
            .ok_or("Window settings store not initialized")?;
        store.set_is_maximized(value)
    }

    pub fn set_match_system_window_chrome(&self, value: bool) -> Result<(), String> {
        let guard = self
            .store
            .lock()
            .map_err(|_| "Failed to lock window settings store".to_string())?;
        let store = guard
            .as_ref()
            .ok_or("Window settings store not initialized")?;
        store.set_match_system_window_chrome(value)
    }

    pub fn set_titlebar_mode(&self, mode: TitlebarMode) -> Result<(), String> {
        let store = self.store.lock().map_err(|e| format!("lock poisoned: {}", e))?;
        store
            .as_ref()
            .ok_or_else(|| "Window settings store not initialized".to_string())?
            .set_titlebar_mode(mode)
    }

    pub fn get_titlebar_mode(&self) -> Result<TitlebarMode, String> {
        let store = self.store.lock().map_err(|e| format!("lock poisoned: {}", e))?;
        let settings = store
            .as_ref()
            .ok_or_else(|| "Window settings store not initialized".to_string())?
            .get_settings()?;
        Ok(settings.titlebar_mode)
    }
}

/// Check that window dimensions are within a sane range.
/// Prevents GDK/Cairo crashes from corrupt values (e.g. 9084748x62267212).
fn is_valid_window_size(width: f64, height: f64) -> bool {
    const MIN: f64 = 200.0;
    const MAX: f64 = 32767.0;
    width.is_finite()
        && height.is_finite()
        && width >= MIN
        && width <= MAX
        && height >= MIN
        && height <= MAX
}

// Tauri commands

#[tauri::command]
pub fn get_window_settings(
    state: tauri::State<WindowSettingsState>,
) -> Result<WindowSettings, String> {
    state.get_settings()
}

#[tauri::command]
pub fn set_use_system_titlebar(
    value: bool,
    state: tauri::State<WindowSettingsState>,
) -> Result<(), String> {
    info!("[WindowSettings] Setting use_system_titlebar to {}", value);
    state.set_use_system_titlebar(value)
}

#[tauri::command]
pub fn set_match_system_window_chrome(
    value: bool,
    state: tauri::State<WindowSettingsState>,
) -> Result<(), String> {
    info!(
        "[WindowSettings] Setting match_system_window_chrome to {} (restart required)",
        value
    );
    state.set_match_system_window_chrome(value)
}
