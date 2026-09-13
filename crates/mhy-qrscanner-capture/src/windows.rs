//! Windows GDI screen capture.
//!
//! Uses `BitBlt` from the screen DC into a 32-bit top-down DIB, then converts
//! BGRA to RGB. Works on Windows 10/11 for both the virtual screen and a region.
//!
//! Note: when the process is DPI-unaware, Windows may virtualise the reported
//! metrics, producing a scaled capture. The QR payload survives scaling (the
//! decoder retries upscaled variants), so no process-wide DPI setting is changed
//! here — a library must not mutate global process state.

use crate::{CaptureError, Frame};
use std::ptr;
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use winapi::um::winuser::{
    GetDC, GetSystemMetrics, ReleaseDC, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
};

const SOURCE_VIRTUAL: &str = "screen:virtual";

fn last_error(call: &'static str) -> CaptureError {
    CaptureError::Win32 {
        call,
        code: unsafe { winapi::um::errhandlingapi::GetLastError() },
    }
}

/// RAII wrapper so every early return still releases the GDI objects.
struct GdiGuard {
    screen_dc: HDC,
    mem_dc: HDC,
    bitmap: HBITMAP,
}

impl Drop for GdiGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.mem_dc.is_null() {
                if !self.bitmap.is_null() {
                    SelectObject(self.mem_dc, self.bitmap as HGDIOBJ);
                }
                DeleteDC(self.mem_dc);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as HGDIOBJ);
            }
            if !self.screen_dc.is_null() {
                ReleaseDC(ptr::null_mut(), self.screen_dc);
            }
        }
    }
}

fn capture(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source_id: &str,
) -> Result<Frame, CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::BadRegion(format!("{width}x{height}")));
    }

    let screen_dc = unsafe { GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err(CaptureError::NoScreen);
    }

    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(ptr::null_mut(), screen_dc) };
        return Err(last_error("CreateCompatibleDC"));
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
        }
        return Err(last_error("CreateCompatibleBitmap"));
    }

    let guard = GdiGuard {
        screen_dc,
        mem_dc,
        bitmap,
    };

    let previous = unsafe { SelectObject(guard.mem_dc, guard.bitmap as HGDIOBJ) };
    if previous.is_null() {
        return Err(last_error("SelectObject"));
    }

    let blitted = unsafe {
        BitBlt(
            guard.mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            guard.screen_dc,
            x,
            y,
            SRCCOPY,
        )
    };
    if blitted == 0 {
        return Err(last_error("BitBlt"));
    }

    // Top-down 32bpp DIB: negative height avoids a vertical flip on read.
    let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width as i32;
    info.bmiHeader.biHeight = -(height as i32);
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;

    let stride = width as usize * 4;
    let mut bgra = vec![0u8; stride * height as usize];
    let rows = unsafe {
        GetDIBits(
            guard.mem_dc,
            guard.bitmap,
            0,
            height,
            bgra.as_mut_ptr() as *mut _,
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if rows == 0 {
        return Err(last_error("GetDIBits"));
    }

    // BGRA -> RGB. Convert whole rows at once: it is the hot loop for a
    // multi-monitor capture and avoids a per-pixel push.
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for row in bgra.chunks_exact(stride) {
        for px in row.as_chunks::<4>().0 {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]);
        }
    }

    Frame::new(source_id, width, height, rgb)
}

pub(crate) fn capture_virtual_screen() -> Result<Frame, CaptureError> {
    let (x, y, w, h) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    if w <= 0 || h <= 0 {
        return Err(CaptureError::NoScreen);
    }
    capture(x, y, w as u32, h as u32, SOURCE_VIRTUAL)
}

pub(crate) fn capture_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<Frame, CaptureError> {
    capture(x, y, width, height, "screen:region")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_virtual_screen_with_expected_buffer() {
        let frame = capture_virtual_screen().expect("capture should succeed on Windows");
        assert!(frame.width > 0 && frame.height > 0);
        assert_eq!(
            frame.byte_len(),
            frame.width as usize * frame.height as usize * 3
        );
        assert_eq!(frame.source_id, SOURCE_VIRTUAL);
    }

    #[test]
    fn captures_region() {
        let frame = capture_region(0, 0, 64, 32).expect("region capture should succeed");
        assert_eq!(frame.width, 64);
        assert_eq!(frame.height, 32);
        assert_eq!(frame.byte_len(), 64 * 32 * 3);
    }

    #[test]
    fn rejects_zero_sized_region() {
        let err = capture_region(0, 0, 0, 10).unwrap_err();
        assert!(matches!(err, CaptureError::BadRegion(_)));
    }
}
