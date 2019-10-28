use crate::Kind;

/// Header of every BF file. All data inside the header is not
/// compressed.
#[repr(C)]
#[derive(FromBytes, AsBytes, Eq, PartialEq, Hash, Debug)]
pub struct Header {
    pub magic: u16,
    pub kind: u8,
    pub version: u8,
    pub reserved: u32,
    pub additional: u64,
    pub uncompressed: u64,
    pub compressed: u64,
}

/* Constant representing the two byte magic sequence 'BF' */
pub const BF_MAGIC: u16 = 17986;
pub const BF_MAX_SUPPORTED_VERSION: u8 = 1;

impl Header {
    pub fn new(
        kind: Kind,
        version: u8,
        additional: u64,
        uncompressed: u64,
        compressed: u64,
    ) -> Self {
        Header {
            magic: BF_MAGIC,
            kind: kind as u8,
            version,
            reserved: 0,
            additional,
            uncompressed,
            compressed,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{load_bf_from_bytes, Error, Header, Kind, BF_MAGIC, BF_MAX_SUPPORTED_VERSION};
    use matches::assert_matches;
    use zerocopy::AsBytes;

    #[test]
    fn test_load_bf_from_bytes() {
        let header = Header {
            magic: BF_MAGIC,
            kind: Kind::CompiledShader as u8,
            version: 1,
            reserved: 0,
            additional: 66,
            uncompressed: 1024,
            compressed: 1023,
        };
        let data = &[1, 2, 3, 4, 1, 2, 3, 4];

        let mut bytes = Vec::new();
        bytes.extend(header.as_bytes());
        bytes.extend(data.as_bytes());

        // load from bytes
        let result = load_bf_from_bytes(&bytes);

        assert!(result.is_ok());

        let file = result.ok().unwrap();

        assert_eq!(file.header.magic, header.magic);
        assert_eq!(file.header.kind, header.kind);
        assert_eq!(file.header.version, header.version);
        assert_eq!(file.header.reserved, header.reserved);
        assert_eq!(file.header.uncompressed, header.uncompressed);
        assert_eq!(file.header.compressed, header.compressed);
        assert_eq!(file.header.additional, header.additional);
        assert_eq!(file.data, data);
    }

    #[test]
    fn test_invalid_header_variants() {
        assert_matches!(
            load_bf_from_bytes(&[0, 0, 0]),
            Err(Error::InvalidFileSignature)
        );
        assert_matches!(
            load_bf_from_bytes(&[66, 70, 255]),
            Err(Error::InvalidKindValue)
        );
        assert_matches!(
            load_bf_from_bytes(&[66, 70, 1, BF_MAX_SUPPORTED_VERSION + 1]),
            Err(Error::VersionTooHigh)
        );
        assert_matches!(
            load_bf_from_bytes(&[66, 70, 1, 1, 0, 1]),
            Err(Error::NotEnoughDataOrUnaligned)
        );
    }
}
