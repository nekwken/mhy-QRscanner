//! Frame capture sources.
//!
//! Only the Windows screen source exists today. The module deliberately produces
//! a uniform [`Frame`] so a future live-stream or camera source can reuse the
//! vision layer without touching callers.

use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod frame;

pub use frame::Frame;

#[cfg(windows)]
mod windows;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture is not implemented on this platform")]
    Unsupported,
    #[error("no screen device available")]
    NoScreen,
    #[error("win32 call `{call}` failed (error {code})")]
    Win32 { call: &'static str, code: u32 },
    #[error("invalid capture region: {0}")]
    BadRegion(String),
    #[error("frame buffer has unexpected length {got} (expected {expected})")]
    BufferSize { got: usize, expected: usize },
}

/// Monotonic-ish capture timestamp in milliseconds since the Unix epoch.
pub(crate) fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Capture every monitor as one image (the Windows "virtual screen").
#[cfg(windows)]
pub fn capture_virtual_screen() -> Result<Frame, CaptureError> {
    windows::capture_virtual_screen()
}

/// Capture every monitor as one image (the Windows "virtual screen").
#[cfg(not(windows))]
pub fn capture_virtual_screen() -> Result<Frame, CaptureError> {
    Err(CaptureError::Unsupported)
}

/// Capture a rectangle in virtual-screen coordinates.
#[cfg(windows)]
pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Frame, CaptureError> {
    windows::capture_region(x, y, width, height)
}

/// Capture a rectangle in virtual-screen coordinates.
#[cfg(not(windows))]
pub fn capture_region(_x: i32, _y: i32, _width: u32, _height: u32) -> Result<Frame, CaptureError> {
    Err(CaptureError::Unsupported)
}
