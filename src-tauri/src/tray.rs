//! System tray icon implementation for QBZ.
//!
//! On Linux we use a custom [`ksni`] (StatusNotifierItem) implementation so
//! primary-click actually toggles the window (Tauri's libayatana-appindicator
//! backend cannot dispatch left-click — issue #310). On macOS we keep the
//! Tauri tray. No Windows client is shipped, so the windows cfg is absent.

#[cfg(target_os = "linux")]
use crate::tray_linux_ksni;

#[cfg(not(target_os = "linux"))]
use image::GenericImageView;
#[cfg(not(target_os = "linux"))]
use std::path::PathBuf;
#[cfg(not(target_os = "linux"))]
use std::sync::Mutex;
#[cfg(not(target_os = "linux"))]
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
#[cfg(target_os = "linux")]
use tauri::Manager;
use tauri::AppHandle;

#[cfg(not(target_os = "linux"))]
const TRAY_ICON_COLOR_PNG: &[u8] = include_bytes!("../icons/tray.png");
#[cfg(not(target_os = "linux"))]
const TRAY_ICON_MONO_WHITE_PNG: &[u8] = include_bytes!("../icons/tray-dark-64.png");
#[cfg(not(target_os = "linux"))]
const TRAY_ICON_MONO_BLACK_PNG: &[u8] = include_bytes!("../icons/tray-light-64.png");

#[cfg(not(target_os = "linux"))]
#[derive(Clone, Copy, Debug)]
enum IconVariant {
    /// Black glyph — for light menu bars.
    MonoBlack,
    /// White glyph — for dark menu bars.
    MonoWhite,
    /// Full colour vinyl logo.
    Color,
}

#[cfg(not(target_os = "linux"))]
pub struct NativeTrayHandle {
    tray: Mutex<Option<TrayIcon>>,
}

#[cfg(not(target_os = "linux"))]
impl NativeTrayHandle {
    fn empty() -> Self {
        Self {
            tray: Mutex::new(None),
        }
    }

    fn install(&self, tray: TrayIcon) {
        if let Ok(mut guard) = self.tray.lock() {
            *guard = Some(tray);
        }
    }

    pub fn set_icon_theme(&self, theme: String) {
        let tray = match self.tray.lock() {
            Ok(guard) => guard.as_ref().cloned(),
            Err(_) => None,
        };
        let Some(tray) = tray else {
            return;
        };

        if let Err(e) = tray.set_icon(Some(load_tray_icon(Some(&theme)))) {
            log::error!("[tray] failed to set macOS tray icon theme '{}': {}", theme, e);
        }
    }
}

/// Ensure tray icon is available in the user's icon theme directory.
/// This makes the icon discoverable by libayatana-appindicator via
/// StatusNotifierItem name lookup on DEs where pixmap data is not supported.
#[cfg(not(target_os = "linux"))]
fn ensure_tray_icon_in_theme() {
    let icon_dirs = [
        // Flatpak: /app has icons installed by manifest
        "/app/share/icons/hicolor/32x32/apps/com.blitzfc.qbz.png",
    ];

    // If icon already exists in a known location, nothing to do
    for path in &icon_dirs {
        if std::path::Path::new(path).exists() {
            return;
        }
    }

    // Write embedded tray icon to user's local icon dir so panels can find it
    if let Some(data_dir) = dirs::data_dir() {
        let icon_dir = data_dir.join("icons/hicolor/32x32/apps");
        if std::fs::create_dir_all(&icon_dir).is_ok() {
            let icon_path = icon_dir.join("com.blitzfc.qbz.png");
            if !icon_path.exists() {
                if let Err(e) = std::fs::write(&icon_path, TRAY_ICON_COLOR_PNG) {
                    log::warn!("Failed to write tray icon to theme dir: {}", e);
                } else {
                    log::info!("Installed tray icon to {:?}", icon_path);
                }
            }
        }
    }
}

/// Check if running inside Flatpak sandbox (macOS has no Flatpak; this is
/// kept for symmetry with the older Linux path)
#[cfg(not(target_os = "linux"))]
fn is_flatpak() -> bool {
    std::env::var("FLATPAK_ID").is_ok() || std::path::Path::new("/.flatpak-info").exists()
}

/// Detect whether the macOS menu bar is currently dark. We key off the global
/// Apple interface style so `auto` can mirror the Linux tray theme behavior:
/// white glyph on dark chrome, black glyph on light chrome.
#[cfg(target_os = "macos")]
fn prefer_dark_tray() -> bool {
    if let Ok(out) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
    {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim() == "Dark";
        }
    }
    false
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
fn prefer_dark_tray() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn resolve_variant(theme_override: Option<&str>) -> IconVariant {
    match theme_override {
        Some("mono-light") => IconVariant::MonoWhite,
        Some("mono-dark") => IconVariant::MonoBlack,
        Some("color") => IconVariant::Color,
        _ => {
            if prefer_dark_tray() {
                IconVariant::MonoWhite
            } else {
                IconVariant::MonoBlack
            }
        }
    }
}

/// Get the tray icon - loads from file in Flatpak, embedded data otherwise.
#[cfg(not(target_os = "linux"))]
fn load_tray_icon(theme_override: Option<&str>) -> Image<'static> {
    // In Flatpak, try to use the installed icon file first
    // This works better with StatusNotifierItem/libayatana-appindicator
    if is_flatpak() {
        let icon_path = PathBuf::from("/app/share/icons/hicolor/32x32/apps/com.blitzfc.qbz.png");
        if icon_path.exists() {
            log::info!("Flatpak detected, loading tray icon from: {:?}", icon_path);
            if let Ok(icon_data) = std::fs::read(&icon_path) {
                if let Ok(img) = image::load_from_memory(&icon_data) {
                    let (width, height) = img.dimensions();
                    let rgba = img.into_rgba8().into_raw();
                    return Image::new_owned(rgba, width, height);
                }
            }
            log::warn!("Failed to load icon from path, falling back to embedded");
        }
    }

    // Default: decode embedded PNG
    let icon_bytes = match resolve_variant(theme_override) {
        IconVariant::MonoBlack => TRAY_ICON_MONO_BLACK_PNG,
        IconVariant::MonoWhite => TRAY_ICON_MONO_WHITE_PNG,
        IconVariant::Color => TRAY_ICON_COLOR_PNG,
    };
    let img = image::load_from_memory(icon_bytes).expect("Failed to decode tray icon PNG");
    let (width, height) = img.dimensions();
    let rgba = img.into_rgba8().into_raw();
    Image::new_owned(rgba, width, height)
}

/// Initialize the system tray icon. Dispatches to the platform-specific
/// backend: ksni on Linux (see `tray_linux_ksni`), Tauri's built-in tray on
/// macOS. Falls back to a clean error on unknown targets.
///
/// On Linux this also installs the live `LinuxTrayHandle` into Tauri state
/// so the rest of the backend can push live tooltip updates as the player
/// state changes. `theme_override` is the persisted user preference for
/// the icon variant.
pub fn init_tray(
    app: &AppHandle,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    theme_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        let handle = tray_linux_ksni::init(app, theme_override)?;
        app.manage(handle);
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let handle = init_tray_tauri(app, theme_override)?;
        app.manage(handle);
        Ok(())
    }
}

/// Tauri-backed tray implementation used on macOS. Kept as a separate fn so
/// the Linux path doesn't pay to compile the Tauri tray at all.
#[cfg(not(target_os = "linux"))]
fn init_tray_tauri(
    app: &AppHandle,
    theme_override: Option<&str>,
) -> Result<NativeTrayHandle, Box<dyn std::error::Error>> {
    log::info!("Initializing system tray icon (Tauri backend)");

    // Create menu items
    let play_pause = MenuItem::with_id(app, "play_pause", "Play/Pause", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "Next Track", true, None::<&str>)?;
    let previous = MenuItem::with_id(app, "previous", "Previous Track", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide Window", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit QBZ", true, None::<&str>)?;

    // Build tray menu
    let tray_menu = Menu::with_items(
        app,
        &[
            &play_pause,
            &next,
            &previous,
            &separator1,
            &show_hide,
            &separator2,
            &quit,
        ],
    )?;

    // Ensure tray icon is available in icon theme for StatusNotifierItem lookup
    ensure_tray_icon_in_theme();

    // Load custom tray icon (with transparent background)
    let tray_icon = load_tray_icon(theme_override);

    // Build and display tray icon
    let mut builder = TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&tray_menu)
        .tooltip("QBZ - Music Player")
        .show_menu_on_left_click(false); // Left click toggles window, right click shows menu

    // Set temp dir for the icon file that libayatana-appindicator writes.
    // In Flatpak, the default temp dir is inside the sandbox and invisible
    // to the host's KDE StatusNotifierWatcher. Use ~/.local/share/icons
    // which is exported to the host via Flatpak's filesystem permissions.
    if is_flatpak() {
        if let Some(data_dir) = dirs::data_dir() {
            let tray_dir = data_dir.join("icons/hicolor/32x32/apps");
            if std::fs::create_dir_all(&tray_dir).is_ok() {
                builder = builder.temp_dir_path(&tray_dir);
            }
        }
    } else if let Some(runtime_dir) = dirs::runtime_dir() {
        let tray_dir = runtime_dir.join("qbz-tray");
        if std::fs::create_dir_all(&tray_dir).is_ok() {
            builder = builder.temp_dir_path(&tray_dir);
        }
    }

    let tray = builder
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            log::info!("Tray menu event: {}", id);

            match id {
                "play_pause" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("tray:play_pause", ());
                    }
                }
                "next" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("tray:next", ());
                    }
                }
                "previous" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("tray:previous", ());
                    }
                }
                "show_hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        log::info!("Show/Hide: window visible = {}", is_visible);
                        if is_visible {
                            log::info!("Hiding window");
                            let _ = window.hide();
                        } else {
                            log::info!("Showing window");
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                }
                "quit" => {
                    log::info!("Quit from tray menu");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                // Left click toggles window visibility
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    log::info!("Tray icon left-click");
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(true);
                        let is_minimized = window.is_minimized().unwrap_or(false);

                        if is_visible && !is_minimized {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            if is_minimized {
                                let _ = window.unminimize();
                            }
                            let _ = window.set_focus();
                        }
                    }
                }
                // Double click always brings window to front
                TrayIconEvent::DoubleClick { .. } => {
                    log::info!("Tray icon double-click");
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        // Ensure window is visible first
                        let _ = window.show();
                        // Unminimize if minimized
                        if window.is_minimized().unwrap_or(false) {
                            let _ = window.unminimize();
                        }
                        // Always bring to front and focus
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    log::info!("System tray icon initialized");
    let live = NativeTrayHandle::empty();
    live.install(tray);
    Ok(live)
}
