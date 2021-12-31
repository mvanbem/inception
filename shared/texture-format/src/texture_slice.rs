use core::ops::Range;

use crate::TextureFormat;

#[derive(Clone, Copy)]
pub struct TextureSlice<'a> {
    pub(crate) format: TextureFormat,
    pub(crate) x0: usize,
    pub(crate) y0: usize,
    pub(crate) x1: usize,
    pub(crate) y1: usize,
    pub(crate) physical_width: usize,
    pub(crate) physical_height: usize,
    pub(crate) data: &'a [u8],
}

impl<'a> TextureSlice<'a> {
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn x_range(self) -> Range<usize> {
        self.x0..self.x1
    }

    pub fn y_range(self) -> Range<usize> {
        self.y0..self.y1
    }

    pub fn width(self) -> usize {
        self.x1 - self.x0
    }

    pub fn height(self) -> usize {
        self.y1 - self.y0
    }

    /// Gets the underlying data only if it is the physical representation of this slice.
    pub fn data(self) -> Option<&'a [u8]> {
        if self.x0 == 0 && self.y0 == 0 {
            let metrics = self.format.metrics();
            let physical_width = metrics.physical_width(self.x1);
            let physical_height = metrics.physical_height(self.y1);
            if physical_width == self.physical_width && physical_height == self.physical_height {
                return Some(self.data);
            }
        }
        None
    }

    pub fn get_texel(self, x: usize, y: usize) -> [u8; 4] {
        assert!(x < self.width() && y < self.height());
        self.format.dyn_codec().get_texel(
            self.physical_width,
            self.physical_height,
            self.data,
            x + self.x0,
            y + self.y0,
        )
    }
}
