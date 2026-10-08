//! Rendered pixel buffers.

/// An 8 bit RGBA image, rows top to bottom, no padding between rows.
#[derive(Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
    /// Wraps a tightly packed RGBA buffer. Returns `None` if the length does not match.
    pub fn from_raw(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        (pixels.len() == expected).then_some(Self {
            width,
            height,
            pixels,
        })
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The RGBA bytes, `width * height * 4` long.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Takes the RGBA bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.pixels
    }

    /// The RGBA value at `x`, `y`, or `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        let p = self.pixels.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }
}

impl std::fmt::Debug for RgbaImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RgbaImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::RgbaImage;

    #[test]
    fn from_raw_checks_length() {
        assert!(RgbaImage::from_raw(2, 2, vec![0; 16]).is_some());
        assert!(RgbaImage::from_raw(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn pixel_access() {
        let mut bytes = vec![0; 16];
        bytes[12..16].copy_from_slice(&[1, 2, 3, 4]);
        let img = RgbaImage::from_raw(2, 2, bytes).unwrap();
        assert_eq!(img.pixel(1, 1), Some([1, 2, 3, 4]));
        assert_eq!(img.pixel(2, 0), None);
    }
}
