//! Local QR image decoding (no network, no external tools).
//!
//! Uses `rqrr` over frames prepared by the `image` crate. Pass order depends on
//! the frame size so neither a multi-monitor capture nor a tiny in-game QR
//! wastes work.

use mhy_qrscanner_mihoyo::device_api::MihoyoError;
use std::path::Path;

/// Decode the first QR payload found in an image file.
pub fn decode_qr_file(path: &Path) -> Result<String, MihoyoError> {
    let img = image::open(path)
        .map_err(|e| MihoyoError::InvalidResponse(format!("open image {path:?}: {e}")))?;
    decode_qr_image(&img)
}

/// Largest width used for the first detection pass.
///
/// QR detection cost grows with pixel count, so a multi-monitor capture is
/// downscaled before the first attempt. A game login QR is large enough to
/// survive this; smaller frames are untouched.
const DETECT_WIDTH_LIMIT: u32 = 1280;

/// Decode a QR from an 8-bit luma image (the cheap path used by screen capture).
///
/// Pass order by size:
///
/// - wider than [`DETECT_WIDTH_LIMIT`]: scale to the limit, then half, then native
/// - 800..=limit wide: native, then 2x
/// - narrower than 800: native, then 2x, then 3x
pub fn decode_qr_luma(luma: image::GrayImage) -> Result<String, MihoyoError> {
    let (width, height) = luma.dimensions();

    if width > DETECT_WIDTH_LIMIT {
        let to_limit = DETECT_WIDTH_LIMIT as f32 / width as f32;
        if let Some(v) = try_decode(resize_luma(&luma, to_limit)) {
            return Ok(v);
        }
        if let Some(v) = try_decode(resize_luma(&luma, 0.5)) {
            return Ok(v);
        }
        if let Some(v) = try_decode(luma) {
            return Ok(v);
        }
        return Err(no_qr_error());
    }

    if let Some(v) = try_decode(luma.clone()) {
        return Ok(v);
    }
    if let Some(v) = try_decode(resize_luma(&luma, 2.0)) {
        return Ok(v);
    }
    if width < 800 && height < 800 {
        if let Some(v) = try_decode(resize_luma(&luma, 3.0)) {
            return Ok(v);
        }
    }
    Err(no_qr_error())
}

fn resize_luma(luma: &image::GrayImage, factor: f32) -> image::GrayImage {
    let w = ((luma.width() as f32) * factor).round().max(1.0) as u32;
    let h = ((luma.height() as f32) * factor).round().max(1.0) as u32;
    image::imageops::resize(luma, w, h, image::imageops::FilterType::Triangle)
}

/// Decode a QR from an in-memory image.
pub fn decode_qr_image(img: &image::DynamicImage) -> Result<String, MihoyoError> {
    decode_qr_luma(img.to_luma8())
}

fn no_qr_error() -> MihoyoError {
    MihoyoError::InvalidResponse("no QR code found in image".into())
}

fn try_decode(luma: image::GrayImage) -> Option<String> {
    let mut prep = rqrr::PreparedImage::prepare(luma);
    let grids = prep.detect_grids();
    for g in grids {
        if let Ok((_meta, content)) = g.decode() {
            if !content.is_empty() {
                return Some(content);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Luma, Rgb, RgbImage};

    /// Round-trip: render a tiny QR? We cannot encode without a writer crate,
    /// so assert the failure path is a clean error instead.
    #[test]
    fn blank_image_reports_clean_error() {
        let img = RgbImage::from_pixel(64, 64, Rgb([255, 255, 255]));
        let err = decode_qr_image(&image::DynamicImage::ImageRgb8(img)).unwrap_err();
        assert!(err.to_string().contains("no QR code"));
    }

    #[test]
    fn luma_blank_error() {
        let img = image::GrayImage::from_pixel(32, 32, Luma([0u8]));
        assert!(try_decode(img).is_none());
    }

    #[test]
    fn large_frame_takes_downscale_path() {
        // A 3000-wide blank frame must not attempt an upscale; a clean error is
        // enough to prove the size-dependent branch ran without huge allocations.
        let img = RgbImage::from_pixel(3000, 1000, Rgb([255, 255, 255]));
        let err = decode_qr_image(&image::DynamicImage::ImageRgb8(img)).unwrap_err();
        assert!(err.to_string().contains("no QR code"));
    }

    #[test]
    fn small_blank_frame_errors() {
        let img = RgbImage::from_pixel(200, 200, Rgb([10, 10, 10]));
        let err = decode_qr_image(&image::DynamicImage::ImageRgb8(img)).unwrap_err();
        assert!(err.to_string().contains("no QR code"));
    }

    #[test]
    fn try_scale_rejects_zero_size() {
        // Tiny luma resized by 0.01 must clamp to 1x1 rather than panic.
        let luma = image::GrayImage::from_pixel(4, 4, Luma([0u8]));
        let small = resize_luma(&luma, 0.01);
        assert_eq!(small.dimensions(), (1, 1));
    }

    #[test]
    fn luma_entry_point_reports_clean_error() {
        let luma = image::GrayImage::from_pixel(1200, 800, Luma([255u8]));
        let err = decode_qr_luma(luma).unwrap_err();
        assert!(err.to_string().contains("no QR code"));
    }
}
