use crate::lz4::Compressed;
use bincode::config;
use serde::{Deserialize, Serialize};

pub mod lz4;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum ColorSpace {
    Linear,
    Srgb,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Format {
    R8,
    Dxt1,
    Dxt3,
    Dxt5,
    Rgb8,
    Rgba8,
    SrgbDxt1,
    SrgbDxt3,
    SrgbDxt5,
    Srgb8,
    Srgb8A8,
}

impl Format {
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
        }
    }

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
        }
    }

    pub fn color_space(self) -> ColorSpace {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Image<'a> {
    pub format: Format,
    pub width: u16,
    pub height: u16,
    pub mipmap_data: &'a [u8],
}

impl<'a> Image<'a> {
    /// Returns the number of mip-maps stored in `mipmap_data` buffer
    /// of this Image struct. If the Image contains only one level of
    /// mip-maps this function returns 1.
    pub fn mipmap_count(&self) -> u32 {
        // todo: make recurrent relation O(1)
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
    pub fn mipmaps(&self) -> MipMaps<'a> {
        MipMaps {
            data: self.mipmap_data,
            format: self.format,
            width: self.width as usize,
            height: self.height as usize,
            index: 0,
        }
    }
}

pub struct MipMaps<'a> {
    data: &'a [u8],
    format: Format,
    index: usize,
    width: usize,
    height: usize,
}

/// Struct representing a single mip-map of the parent Image object.
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

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VertexDataFormat {
    // vec3 (pos), vec3(nor), vec2(uv)
    PositionNormalUv,
}

impl VertexDataFormat {
    #[inline]
    pub fn size_of_one_vertex(self) -> usize {
        match self {
            VertexDataFormat::PositionNormalUv => std::mem::size_of::<f32>() * 8,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum IndexType {
    U8,
    U16,
    U32,
}

impl IndexType {
    #[inline]
    pub fn size_of_one_index(self) -> usize {
        match self {
            IndexType::U8 => std::mem::size_of::<u8>(),
            IndexType::U16 => std::mem::size_of::<u16>(),
            IndexType::U32 => std::mem::size_of::<u32>(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Geometry<'a> {
    pub vertex_format: VertexDataFormat,
    pub vertex_data: &'a [u8],
    pub index_type: IndexType,
    pub index_data: &'a [u8],
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Container<'a> {
    #[serde(borrow)]
    Image(Image<'a>),
    #[serde(borrow)]
    Geometry(Geometry<'a>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Data<'a> {
    #[serde(borrow)]
    Compressed(Compressed<Container<'a>>),
    #[serde(borrow)]
    Uncompressed(Container<'a>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File<'a> {
    pub magic: u16,
    pub version: u8,
    #[serde(borrow)]
    pub data: Data<'a>,
}

impl<'a> File<'a> {
    // Creates a new File object with specified Data.
    fn with_data(data: Data<'a>) -> Self {
        File {
            magic: BF_MAGIC,
            version: BF_VERSION,
            data,
        }
    }

    /// Creates a new File object with correct header and specified
    /// container value.
    pub fn create_uncompressed(container: Container<'a>) -> Self {
        Self::with_data(Data::Compressed(Compressed::new(container)))
    }

    /// Creates a new File object with correct header and specified
    /// container value which will be compressed when this object
    /// will be serialized.
    ///
    /// Note: This method does not perform any serialization and
    /// returns instantly.
    pub fn create_compressed(container: Container<'a>) -> Self {
        Self::with_data(Data::Uncompressed(container))
    }

    fn container(self) -> Container<'a> {
        match self.data {
            Data::Compressed(c) => c.0,
            Data::Uncompressed(x) => x,
        }
    }

    pub fn try_to_geometry(self) -> Result<Geometry<'a>, ()> {
        match self.container() {
            Container::Geometry(g) => Ok(g),
            _ => Err(()),
        }
    }

    pub fn try_to_image(self) -> Result<Image<'a>, ()> {
        match self.container() {
            Container::Image(i) => Ok(i),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidMagic,
    UnsupportedVersion,
    SerdeError(bincode::Error),
}

/* Constant representing the two byte magic sequence 'BF' */
pub const BF_MAGIC: u16 = 17986;
pub const BF_VERSION: u8 = 1;

fn verify_bf_file_header(file: File) -> Result<File, Error> {
    if file.magic != BF_MAGIC {
        return Err(Error::InvalidMagic);
    }

    if file.version != BF_VERSION {
        return Err(Error::UnsupportedVersion);
    }

    Ok(file)
}

/// Tries to load provided array of bytes as File using `bincode`
/// deserialize function and then verifying whether file magic
/// matches and version is supported. If these conditions are met
/// and `bincode` deserialization succeeds this function returns
/// File object. Error is returned otherwise.
pub fn load_bf_from_bytes(bytes: &[u8]) -> Result<File, Error> {
    // verify magic even before trying to deserialize. this can
    // prevent confusing errors when deserialization fails in the
    // middle of file of wrong format
    if u16::from_le_bytes([bytes[0], bytes[1]]) != BF_MAGIC {
        return Err(Error::InvalidMagic);
    }

    config()
        .little_endian()
        .deserialize(bytes)
        .map_err(Error::SerdeError)
        .and_then(verify_bf_file_header)
}

/// Serializes the specified file into a Vec of bytes using
/// `bincode` serialize function. The file object is not verified
/// as it is in `load_bf_from_bytes` function. This allows to
/// write potentially invalid Files.
pub fn save_bf_to_bytes(file: &File) -> Result<Vec<u8>, Error> {
    config()
        .little_endian()
        .serialize(file)
        .map_err(Error::SerdeError)
}
