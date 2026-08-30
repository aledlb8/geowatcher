//! A single grayscale analysis frame.

use std::fmt;

/// 8-bit grayscale, row-major, always [`super::ANALYSIS_WIDTH`] x
/// [`super::ANALYSIS_HEIGHT`] so calibration coordinates stay comparable
/// across channels and source resolutions.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub gray: Vec<u8>,
}

impl Frame {
    #[must_use]
    pub fn new(width: u32, height: u32, gray: Vec<u8>) -> Self {
        Self {
            width,
            height,
            gray,
        }
    }

    /// Luminance at `(x, y)`, or 0 out of bounds.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        self.gray.get(index).copied().unwrap_or(0)
    }

    /// One row of pixels, or empty out of bounds.
    #[must_use]
    pub fn row(&self, y: u32) -> &[u8] {
        if y >= self.height {
            return &[];
        }
        let width = self.width as usize;
        let start = (y as usize) * width;
        self.gray.get(start..start + width).unwrap_or(&[])
    }

    /// Mean luminance, used to spot black or frozen frames.
    ///
    /// A full frame maxes out around 1e8, well inside `u32`.
    #[must_use]
    pub fn mean_luminance(&self) -> f64 {
        if self.gray.is_empty() {
            return 0.0;
        }
        let sum: u32 = self.gray.iter().map(|&p| u32::from(p)).sum();
        let count = u32::try_from(self.gray.len()).unwrap_or(u32::MAX);
        f64::from(sum) / f64::from(count)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.gray.len())
            .finish()
    }
}
