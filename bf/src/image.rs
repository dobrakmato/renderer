use static_assertions::assert_eq_size;
use std::convert::TryFrom;

/// Structure for representing all additional data the Image kind
/// of Bf file encodes.
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct ImageAdditional {
    pub width: u16,
    pub height: u16,
    pub format: u8,
    _padding1: u8,
    _padding2: u16,
}

assert_eq_size!(ImageAdditional, u64);

impl ImageAdditional {
    pub fn new(width: u16, height: u16, format: u8) -> Self {
        ImageAdditional {
            width,
            height,
            format,
            _padding1: 0,
            _padding2: 0,
        }
    }

    pub fn into_u64(self) -> u64 {
        // todo: check safety
        unsafe { std::mem::transmute(self) }
    }

    pub fn from_u64(data: u64) -> Self {
        // todo: check safety
        unsafe { std::mem::transmute(data) }
    }
}

/// Enum representing the color space.
pub enum ColorSpace {
    Linear,
    Srgb,
}

/// Enum representing all supported image formats
/// inside the Image kind of BF files.
#[repr(u8)]
pub enum Format {
    // linear variants
    Dxt1 = 0,
    Dxt3 = 1,
    Dxt5 = 2,
    Rgb8 = 3,
    Rgba8 = 4,
    // srgb variants
    SrgbDxt1 = 5,
    SrgbDxt3 = 6,
    SrgbDxt5 = 7,
    Srgb8 = 8,
    Srgb8A8 = 9,
}

impl Format {
    pub fn channels(&self) -> u8 {
        match self {
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
        }
    }

    pub fn bits_per_pixel(&self) -> u16 {
        match self {
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
        }
    }

    pub fn color_space(&self) -> ColorSpace {
        match self {
            Format::SrgbDxt1 => ColorSpace::Srgb,
            Format::SrgbDxt3 => ColorSpace::Srgb,
            Format::SrgbDxt5 => ColorSpace::Srgb,
            Format::Srgb8 => ColorSpace::Srgb,
            Format::Srgb8A8 => ColorSpace::Srgb,
            _ => ColorSpace::Linear,
        }
    }
}

// todo: derive instead of manual implementation
impl TryFrom<u8> for Format {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Format::Dxt1),
            1 => Ok(Format::Dxt3),
            2 => Ok(Format::Dxt5),
            3 => Ok(Format::Rgb8),
            4 => Ok(Format::Rgba8),
            5 => Ok(Format::SrgbDxt1),
            6 => Ok(Format::SrgbDxt3),
            7 => Ok(Format::SrgbDxt5),
            8 => Ok(Format::Srgb8),
            9 => Ok(Format::Srgb8A8),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for Format {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "dxt1" => Ok(Format::Dxt1),
            "dxt3" => Ok(Format::Dxt3),
            "dxt5" => Ok(Format::Dxt5),
            "rgb" => Ok(Format::Rgb8),
            "rgba" => Ok(Format::Rgba8),
            "srgb_dxt1" => Ok(Format::SrgbDxt1),
            "srgb_dxt3" => Ok(Format::SrgbDxt3),
            "srgb_dxt5" => Ok(Format::SrgbDxt5),
            "srgb" => Ok(Format::Srgb8),
            "srgb_a" => Ok(Format::Srgb8A8),
            _ => Err(()),
        }
    }
}

/// Enum representing possible types of images.
pub enum ImageType {
    DXT1,
    DXT5,
    RGBA8,
    RGBA16,
    RGB8,
    RGB16,
    RG16,
    R16,
}

#[cfg(test)]
mod tests {
    use crate::{Format, ImageAdditional};

    #[test]
    fn bf_image_additional_data() {
        let a = ImageAdditional {
            width: 4096,
            height: 2048,
            format: Format::Srgb8 as u8,
            _padding1: 0,
            _padding2: 0,
        };

        let b = a;
        let a_u64 = b.into_u64();

        assert_eq!(a, ImageAdditional::from_u64(a_u64));
    }
}
