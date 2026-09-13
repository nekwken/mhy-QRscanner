//! Uniform captured frame handed to the vision layer.

use crate::CaptureError;

/// A captured image in RGB8, top-down row order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Which source produced this frame (e.g. `screen:virtual`).
    pub source_id: String,
    /// Milliseconds since the Unix epoch.
    pub captured_at_unix_ms: u128,
    pub width: u32,
    pub height: u32,
    /// `width * height * 3` bytes, RGB, rows top to bottom.
    pub rgb: Vec<u8>,
}

impl Frame {
    pub fn new(
        source_id: impl Into<String>,
        width: u32,
        height: u32,
        rgb: Vec<u8>,
    ) -> Result<Self, CaptureError> {
        let expected = (width as usize) * (height as usize) * 3;
        if rgb.len() != expected {
            return Err(CaptureError::BufferSize {
                got: rgb.len(),
                expected,
            });
        }
        Ok(Self {
            source_id: source_id.into(),
            captured_at_unix_ms: crate::now_unix_ms(),
            width,
            height,
            rgb,
        })
    }

    pub fn byte_len(&self) -> usize {
        self.rgb.len()
    }

    /// Convert to an image buffer for the QR decoder.
    pub fn to_dynamic_image(&self) -> image::DynamicImage {
        let buf = image::RgbImage::from_raw(self.width, self.height, self.rgb.clone())
            .expect("Frame invariants guarantee a valid RGB buffer");
        image::DynamicImage::ImageRgb8(buf)
    }

    /// Convert straight to 8-bit luma.
    ///
    /// Cheaper than `to_dynamic_image()` for the QR path: no RGB buffer clone and
    /// no intermediate colour image, which matters for multi-monitor captures
    /// (a 5120x1440 frame is ~22 MB of RGB).
    pub fn to_luma8(&self) -> image::GrayImage {
        let mut luma = image::GrayImage::new(self.width, self.height);
        for (px, out) in self.rgb.as_chunks::<3>().0.iter().zip(luma.pixels_mut()) {
            // Rec. 601 luma with integer weights.
            let y = (77 * px[0] as u32 + 150 * px[1] as u32 + 29 * px[2] as u32) >> 8;
            *out = image::Luma([y as u8]);
        }
        luma
    }

    /// Write the frame to a PNG file (debug / audit trail).
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), CaptureError> {
        self.to_dynamic_image()
            .save(path)
            .map_err(|e| CaptureError::BadRegion(format!("cannot write {}: {e}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rejects_wrong_buffer_size() {
        let err = Frame::new("test", 2, 2, vec![0u8; 10]).unwrap_err();
        assert!(matches!(err, CaptureError::BufferSize { .. }));
    }

    #[test]
    fn frame_converts_to_luma() {
        let f = Frame::new("test", 2, 1, vec![255, 255, 255, 0, 0, 0]).unwrap();
        let luma = f.to_luma8();
        assert_eq!(luma.dimensions(), (2, 1));
        assert_eq!(luma.get_pixel(0, 0)[0], 255);
        assert_eq!(luma.get_pixel(1, 0)[0], 0);
    }

    #[test]
    fn frame_converts_to_dynamic_image() {
        let f = Frame::new("test", 2, 1, vec![255, 0, 0, 0, 255, 0]).unwrap();
        let img = f.to_dynamic_image();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 1);
        assert_eq!(f.byte_len(), 6);
        assert!(f.captured_at_unix_ms > 0);
    }
}
