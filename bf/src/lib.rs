use crate::image::Image;
use crate::lz4::Compressed;
use crate::material::Material;
use crate::mesh::Mesh;
use bincode::config;
use serde::{Deserialize, Serialize};

pub mod image;
pub mod lz4;
pub mod material;
pub mod mesh;

#[derive(Debug, Serialize, Deserialize)]
pub enum Container {
    Image(Image),
    Mesh(Mesh),
    Material(Material),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Data {
    Compressed(Compressed<Container>),
    Uncompressed(Container),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub magic: u16,
    pub version: u8,
    pub data: Data,
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

    fn container(self) -> Container {
        match self.data {
            Data::Compressed(c) => c.into(),
            Data::Uncompressed(x) => x,
        }
    }

    pub fn try_to_geometry(self) -> Result<Mesh, ()> {
        try_to_dynamic!(self.container(), Mesh)
    }

    pub fn try_to_image(self) -> Result<Image, ()> {
        try_to_dynamic!(self.container(), Image)
    }

    pub fn try_to_material(self) -> Result<Material, ()> {
        try_to_dynamic!(self.container(), Material)
    }
}

#[derive(Debug)]
pub enum Error {
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
pub const BF_MAGIC: u16 = 17986;
pub const BF_VERSION: u8 = 2;

fn verify_bf_file_header(file: File) -> Result<File, Error> {
    if file.magic != BF_MAGIC {
        return Err(Error::InvalidMagic);
    }

    if file.version != BF_VERSION {
        return Err(Error::UnsupportedVersion {
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
pub fn load_bf_from_bytes(bytes: &[u8]) -> Result<File, Error> {
    // the `bytes` array could be shorter than two bytes. we need
    // to verify that this is not the case before trying to verify
    // the magic.
    if bytes.len() < 2 {
        return Err(Error::FileTooShort);
    }

    // verify magic even before trying to deserialize. this can
    // prevent confusing errors when deserialization fails in the
    // middle of file of wrong format
    if u16::from_le_bytes([bytes[0], bytes[1]]) != BF_MAGIC {
        return Err(Error::InvalidMagic);
    }

    config()
        .little_endian()
        .deserialize(bytes)
        .map_err(Error::BincodeError)
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
        .map_err(Error::BincodeError)
}
