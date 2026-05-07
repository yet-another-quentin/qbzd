//! Linux implementation of `DeviceReservation`.
//!
//! Task 1 ships the public surface, parser helpers, and unit tests for them.
//! The actual zbus client is wired in Task 2 — for now `acquire()` validates
//! the device string and returns a degraded guard so call sites can be
//! plumbed without breaking the build.

use std::fmt;

#[derive(Debug)]
pub struct DeviceReservation {
    #[allow(dead_code)] // Read by `is_active()` once Task 2 introduces the `Active` variant.
    state: ReservationState,
}

#[derive(Debug)]
enum ReservationState {
    /// No active D-Bus reservation; calls fall through as no-ops.
    /// Task 2 reintroduces an `Active` variant carrying the zbus connection
    /// and the bus name we own, at which point `is_active()` becomes useful.
    Degraded,
}

impl DeviceReservation {
    /// Acquire a D-Bus device reservation for the given ALSA `hw:` device.
    ///
    /// Currently in Task 1: validates the device string and returns a
    /// degraded (no-op) guard. The real D-Bus client lands in Task 2, at
    /// which point this function will:
    ///   - Return `Ok(active_guard)` if the bus name is acquired.
    ///   - Return `Ok(degraded_guard)` if the session bus is unreachable.
    ///   - Return `Err(InvalidDevice)` for unparseable device strings.
    ///   - Return `Err(HigherPriorityHolder)` if another app refuses to release.
    ///   - Return `Err(DbusError)` for protocol-level failures.
    ///   - Return `Err(AlsaError)` for ALSA enumeration failures while
    ///     resolving symbolic card names.
    pub fn acquire(hw_device: &str, _app_device_name: &str) -> Result<Self, ReservationError> {
        // Validate the device string. Real D-Bus acquisition lands in Task 2.
        let _card = parse_card_index(hw_device)?;
        Ok(Self {
            state: ReservationState::Degraded,
        })
    }

    /// Whether this guard currently holds an active D-Bus reservation.
    ///
    /// Always `false` in Task 1: there is no `Active` variant yet. Task 2
    /// will reintroduce it once the zbus client wires real state in.
    pub fn is_active(&self) -> bool {
        false
    }
}

impl Drop for DeviceReservation {
    fn drop(&mut self) {
        // Real release lands in Task 2.
    }
}

#[derive(Debug)]
pub enum ReservationError {
    InvalidDevice(String),
    HigherPriorityHolder {
        holder_name: String,
        holder_priority: i32,
    },
    DbusError(String),
    AlsaError(String),
}

impl fmt::Display for ReservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDevice(s) => write!(f, "invalid ALSA device string: {}", s),
            Self::HigherPriorityHolder {
                holder_name,
                holder_priority,
            } => write!(
                f,
                "device reserved by '{}' at priority {}",
                holder_name, holder_priority
            ),
            Self::DbusError(s) => write!(f, "D-Bus error: {}", s),
            Self::AlsaError(s) => write!(f, "ALSA error: {}", s),
        }
    }
}

impl std::error::Error for ReservationError {}

/// Parse an ALSA device string and return the kernel card index.
///
/// Accepts: `"hw:0"`, `"hw:0,0"`, `"hw:1,0"`, `"plughw:1,0"`,
/// `"hw:CARD=DacMagic"`, `"hw:CARD=DacMagic,DEV=0"`.
pub(crate) fn parse_card_index(hw_device: &str) -> Result<u32, ReservationError> {
    let trimmed = hw_device.trim();
    let after_prefix = trimmed
        .strip_prefix("hw:")
        .or_else(|| trimmed.strip_prefix("plughw:"))
        .ok_or_else(|| ReservationError::InvalidDevice(hw_device.to_string()))?;

    let card_part = after_prefix.split(',').next().unwrap_or("");
    let card_part = card_part.trim();

    if card_part.is_empty() {
        return Err(ReservationError::InvalidDevice(hw_device.to_string()));
    }

    if let Some(name) = card_part.strip_prefix("CARD=") {
        resolve_card_index_by_name(name)
    } else {
        card_part
            .parse::<u32>()
            .map_err(|_| ReservationError::InvalidDevice(hw_device.to_string()))
    }
}

/// Resolve a symbolic ALSA card name (e.g., `"DacMagic"`) to its kernel index
/// by iterating over `alsa::card::Iter`.
fn resolve_card_index_by_name(name: &str) -> Result<u32, ReservationError> {
    for card in alsa::card::Iter::new() {
        let card = card.map_err(|e| ReservationError::AlsaError(e.to_string()))?;
        let id = card.get_name().unwrap_or_default();
        if id == name {
            return Ok(card.get_index() as u32);
        }
    }
    Err(ReservationError::InvalidDevice(format!(
        "ALSA card '{}' not found",
        name
    )))
}

/// Format the well-known D-Bus bus name for a given ALSA card index.
#[allow(dead_code)] // Used by the zbus client in Task 2.
pub(crate) fn bus_name_for_card(card_index: u32) -> String {
    format!("org.freedesktop.ReserveDevice1.Audio{}", card_index)
}

/// Format the D-Bus object path for a given ALSA card index.
#[allow(dead_code)] // Used by the zbus client in Task 2.
pub(crate) fn object_path_for_card(card_index: u32) -> String {
    format!("/org/freedesktop/ReserveDevice1/Audio{}", card_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_card_index_basic() {
        assert_eq!(parse_card_index("hw:0").unwrap(), 0);
        assert_eq!(parse_card_index("hw:1,0").unwrap(), 1);
        assert_eq!(parse_card_index("plughw:2,0").unwrap(), 2);
        assert_eq!(parse_card_index("hw:99,3").unwrap(), 99);
    }

    #[test]
    fn parse_card_index_rejects_garbage() {
        assert!(matches!(
            parse_card_index("default"),
            Err(ReservationError::InvalidDevice(_))
        ));
        assert!(matches!(
            parse_card_index("hw:"),
            Err(ReservationError::InvalidDevice(_))
        ));
        assert!(matches!(
            parse_card_index(""),
            Err(ReservationError::InvalidDevice(_))
        ));
    }

    #[test]
    fn bus_name_format() {
        assert_eq!(
            bus_name_for_card(0),
            "org.freedesktop.ReserveDevice1.Audio0"
        );
        assert_eq!(
            bus_name_for_card(7),
            "org.freedesktop.ReserveDevice1.Audio7"
        );
        assert_eq!(
            bus_name_for_card(99),
            "org.freedesktop.ReserveDevice1.Audio99"
        );
    }

    #[test]
    fn object_path_format() {
        assert_eq!(
            object_path_for_card(0),
            "/org/freedesktop/ReserveDevice1/Audio0"
        );
    }

    #[test]
    fn degraded_guard_reports_inactive() {
        let g = DeviceReservation::acquire("hw:0,0", "test").unwrap();
        assert!(!g.is_active());
    }
}
