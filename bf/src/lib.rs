//! This is a library for loading and storing BF files.

use crate::image::Image;
use crate::lz4::Compressed;
use crate::material::Material;
use crate::mesh::Mesh;
use bincode::config;
use serde::{Deserialize, Serialize};

pub use uuid;
pub mod image;
pub mod lz4;
pub mod material;
pub mod mesh;

/// Possible BF file types (Image, Mesh...).
#[derive(Debug, Serialize, Deserialize)]
pub enum Container {
    Image(Image),
    Mesh(Mesh),
    Material(Material),
}

/// Different data storage modes (compressed, uncompressed).
#[derive(Debug, Serialize, Deserialize)]
pub enum Data {
    Compressed(Compressed<Container>),
    Uncompressed(Container),
}

/// BF file with its header and payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    magic: u16,
    version: u8,
    data: Data,
}

/// This macro generates code required for converting the `Container` type
/// into a specific type in fallible way. The generated code evaluates to
/// ```Ok($type)``` if the requested type is correct, ```Err(())```
/// otherwise.  
macro_rules! try_to_dynamic {
    ($container: expr, $type: ident) => {
        match $container {
            Container::$type(t) => Ok(t),
            _ => Err(()),
        };
    };
}

impl File {
    /// Returns the magic bytes as `u16` from beginning of this file.
    #[inline]
    pub fn magic(&self) -> u16 {
        self.magic
    }

    /// Returns the version of BF codec this file was created in.
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns whether this file is compressed or not.
    #[inline]
    pub fn is_compressed(&self) -> bool {
        match &self.data {
            Data::Compressed(_) => true,
            Data::Uncompressed(_) => false,
        }
    }

    // Creates a new File object with specified Data.
    fn with_data(data: Data) -> Self {
        File {
            magic: BF_MAGIC,
            version: BF_VERSION,
            data,
        }
    }

    /// Creates a new File object with correct header and specified
    /// container value.
    pub fn create_uncompressed(container: Container) -> Self {
        Self::with_data(Data::Uncompressed(container))
    }

    /// Creates a new File object with correct header and specified
    /// container value which will be compressed when this object
    /// will be serialized.
    ///
    /// Note: This method does not perform any compression and
    /// returns instantly.
    pub fn create_compressed(container: Container) -> Self {
        Self::with_data(Data::Compressed(Compressed::new(container)))
    }

    /// Unwraps the `Container` struct of this `File` and returns it.
    pub fn into_container(self) -> Container {
        match self.data {
            Data::Compressed(c) => c.into(),
            Data::Uncompressed(x) => x,
        }
    }

    /// Tries to unwrap container (data) of this file as `Mesh`.
    ///
    /// This function returns `Ok(Mesh)` if the file contains a `Mesh` and `Err(())` otherwise.
    pub fn try_to_mesh(self) -> Result<Mesh, ()> {
        try_to_dynamic!(self.into_container(), Mesh)
    }

    /// Tries to unwrap container (data) of this file as `Image`.
    ///
    /// This function returns `Ok(Image)` if the file contains a `Image` and `Err(())` otherwise.
    pub fn try_to_image(self) -> Result<Image, ()> {
        try_to_dynamic!(self.into_container(), Image)
    }

    /// Tries to unwrap container (data) of this file as `Material`.
    ///
    /// This function returns `Ok(Material)` if the file contains a `Material` and `Err(())` otherwise.
    pub fn try_to_material(self) -> Result<Material, ()> {
        try_to_dynamic!(self.into_container(), Material)
    }
}

/// Enumeration of all possible errors that can happen when loading a .bf file
/// using the [`load_bf_from_bytes()`](fn.load_bf_from_bytes.html) function.
#[derive(Debug)]
pub enum LoadError {
    /// File is too short to be valid .bf file.
    FileTooShort,
    /// File has invalid magic bytes.
    InvalidMagic,
    /// The opened file has different version then this library can decode.
    UnsupportedVersion { library: u8, file: u8 },
    /// Internal `bincode` error.
    BincodeError(bincode::Error),
}

/* Constant representing the two byte magic sequence 'BF' */

/// Two bytes magic that is present at the start of every .bf file.
pub const BF_MAGIC: u16 = 17986;

/// Version of BF format this version is able to read.
pub const BF_VERSION: u8 = 2;

fn verify_bf_file_header(file: File) -> Result<File, LoadError> {
    if file.magic != BF_MAGIC {
        return Err(LoadError::InvalidMagic);
    }

    if file.version != BF_VERSION {
        return Err(LoadError::UnsupportedVersion {
            library: BF_VERSION,
            file: file.version,
        });
    }

    Ok(file)
}

/// Tries to load provided array of bytes as File using `bincode`
/// deserialize function and then verifying whether file magic
/// matches and version is supported. If these conditions are met
/// and `bincode` deserialization succeeds this function returns
/// File object. Error is returned otherwise.
pub fn load_bf_from_bytes(bytes: &[u8]) -> Result<File, LoadError> {
    // the `bytes` array could be shorter than two bytes. we need
    // to verify that this is not the case before trying to verify
    // the magic.
    if bytes.len() < 2 {
        return Err(LoadError::FileTooShort);
    }

    // verify magic even before trying to deserialize. this can
    // prevent confusing errors when deserialization fails in the
    // middle of file of wrong format
    if u16::from_le_bytes([bytes[0], bytes[1]]) != BF_MAGIC {
        return Err(LoadError::InvalidMagic);
    }

    config()
        .little_endian()
        .deserialize(bytes)
        .map_err(LoadError::BincodeError)
        .and_then(verify_bf_file_header)
}

/// Serializes the specified file into a Vec of bytes using
/// `bincode` serialize function. The file object is not verified
/// as it is in `load_bf_from_bytes` function. This allows to
/// write potentially invalid Files.
pub fn save_bf_to_bytes(file: &File) -> Result<Vec<u8>, LoadError> {
    config()
        .little_endian()
        .serialize(file)
        .map_err(LoadError::BincodeError)
}
