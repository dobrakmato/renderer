use crate::header::{Header, BF_MAGIC};
use crate::Kind;
use zerocopy::LayoutVerified;

/// Structure for holding loaded BfFile using zero-copy loading mechanism.
#[derive(Debug)]
pub struct File<'a> {
    pub header: LayoutVerified<&'a [u8], Header>,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub enum Error {
    NotEnoughDataOrUnaligned,
    InvalidFileSignature,
    VersionTooHigh,
    InvalidKindValue,
}

/// Loads and deserializes byte array to BfFile using zero-copy mechanism. If
/// the specified byte sequence is invalid Error is returned.
pub fn load_bf_from_bytes(bytes: &[u8]) -> Result<File, Error> {
    // verify magic, version and kind values
    if u16::from_le_bytes([bytes[0], bytes[1]]) != BF_MAGIC {
        return Err(Error::InvalidFileSignature);
    }
    if bytes[2] > Kind::MaxValue as u8 {
        return Err(Error::InvalidKindValue);
    }
    if bytes[3] > 1 {
        return Err(Error::VersionTooHigh);
    }

    // transmute the slice
    match LayoutVerified::new_from_prefix(bytes) {
        None => Err(Error::NotEnoughDataOrUnaligned),
        Some((header, rest)) => Ok(File { header, data: rest }),
    }
}
