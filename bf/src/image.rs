//! Single layer 2D raster with specified format and precomputed mipmaps.
//!
//! This module also provides a way to iterate over stored mip-maps in
//! the `Image` struct. You can use `Image::mipmaps()` function to get
//! `MipMaps` iterator over individual `MipMap` structs.

use serde::{Deserialize, Serialize};

/// All possible [`Image`](struct.Image.html) formats.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Format {
    // BC1
    Dxt1 = 0,
    // BC2
    Dxt3 = 1,
    // BC3
    Dxt5 = 2,
    Rgb8 = 3,
    Rgba8 = 4,
    SrgbDxt1 = 5,
    SrgbDxt3 = 6,
    SrgbDxt5 = 7,
    Srgb8 = 8,
    Srgb8A8 = 9,
    R8 = 10,
    BC6H = 11,
    BC7 = 12,
    SrgbBC7 = 13, // BC7 (srgb)
}

impl Format {
    /// Returns the number of channels this format has.
    pub fn channels(self) -> u8 {
        match self {
            Format::R8 => 1,
            Format::Dxt1 => 3,
            Format::Dxt3 => 4,
            Format::Dxt5 => 4,
            Format::Rgb8 => 3,
            Format::Rgba8 => 4,
            Format::SrgbDxt1 => 3,
            Format::SrgbDxt3 => 4,
            Format::SrgbDxt5 => 4,
            Format::Srgb8 => 3,
            Format::Srgb8A8 => 4,
            Format::BC6H => 3,
            Format::BC7 => 4,
            Format::SrgbBC7 => 3,
        }
    }

    /// Returns whether this image format is compressed or not.
    pub fn compressed(self) -> bool {
        match self {
            Format::Dxt1 => true,
            Format::Dxt3 => true,
            Format::Dxt5 => true,
            Format::Rgb8 => false,
            Format::Rgba8 => false,
            Format::SrgbDxt1 => true,
            Format::SrgbDxt3 => true,
            Format::SrgbDxt5 => true,
            Format::Srgb8 => false,
            Format::Srgb8A8 => false,
            Format::R8 => false,
            Format::BC6H => true,
            Format::BC7 => true,
            Format::SrgbBC7 => true,
        }
    }

    /// Returns the average number of bits that this format uses for one pixel of image
    /// data.
    pub fn bits_per_pixel(self) -> u16 {
        match self {
            Format::R8 => 8,
            Format::Dxt1 => 4,
            Format::Dxt3 => 8,
            Format::Dxt5 => 8,
            Format::Rgb8 => 24,
            Format::Rgba8 => 32,
            Format::SrgbDxt1 => 4,
            Format::SrgbDxt3 => 8,
            Format::SrgbDxt5 => 8,
            Format::Srgb8 => 24,
            Format::Srgb8A8 => 32,
            Format::BC6H => 8,
            Format::BC7 => 8,
            Format::SrgbBC7 => 8,
        }
    }
}

/// Asset type that is used to store single layer of 2D raster graphics in
/// various formats (channel count, color depth, gamma).
///
/// To iterate over stored mip-maps you can use `Image::mipmaps()` method that
/// provides an `Iterator` over `MipMap`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    pub format: Format,
    pub width: u16,
    pub height: u16,
    /// Bytes of individual mip-maps ordered from highest resolution to
    /// lowest. The number of mip-maps can be computed from length of the payload.
    #[serde(with = "serde_bytes")]
    pub mipmap_data: Vec<u8>,
}

impl Image {
    /// Returns the number of mip-maps stored in `mipmap_data` buffer
    /// of this Image struct. If the Image contains only one level of
    /// mip-maps this function returns 1.
    pub fn mipmap_count(&self) -> u32 {
        // todo: make recurrent relation O(1) log2(self.width)
        let mut count = 0u32;
        let mut index = 0;
        let mut width = self.width;
        let mut height = self.height;

        while index < self.mipmap_data.len() {
            index += width as usize * height as usize * self.format.bits_per_pixel() as usize / 8;
            count += 1;
            width /= 2;
            height /= 2;
        }

        count
    }

    /// Returns iterator that splits the `mipmap_data` bytes slice into
    /// type that represents individual mip-maps in this Image.
    pub fn mipmaps(&self) -> MipMaps {
        MipMaps {
            data: self.mipmap_data.as_slice(),
            format: self.format,
            width: self.width as usize,
            height: self.height as usize,
            index: 0,
        }
    }
}

/// Iterator over `Image` that provides individual mip-maps as `MipMap` structs.  
pub struct MipMaps<'a> {
    data: &'a [u8],
    format: Format,
    index: usize,
    width: usize,
    height: usize,
}

/// Struct representing a view of single mip-map of the parent `Image` object.
pub struct MipMap<'a> {
    /// Raw bytes in `format` data type of this mip-map.
    pub data: &'a [u8],
    /// Width of this mip-map in pixels.
    pub width: usize,
    /// Height of this mip-map in pixels.
    pub height: usize,
    /// Offset in bytes to the original `mipmap_data` bytes slice.
    pub offset: usize,
}

impl<'a> Iterator for MipMaps<'a> {
    type Item = MipMap<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let start = self.index;
            let len = self.width * self.height * self.format.bits_per_pixel() as usize / 8;

            self.index += len;
            self.width /= 2;
            self.height /= 2;
            return Some(MipMap {
                data: &self.data[start..start + len],
                width: self.width * 2,
                height: self.height * 2,
                offset: start,
            });
        }
        None
    }
}
