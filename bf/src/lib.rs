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

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VertexDataFormat {
    // vec3 (pos), vec3(nor), vec2(uv)
    PositionNormalUv,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum IndexType {
    U8,
    U16,
    U32,
}

impl IndexType {
    #[inline]
    pub fn size_of_one_element(self) -> usize {
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
