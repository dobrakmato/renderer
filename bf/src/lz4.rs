//! Helper module for easy integration of compressed parts of
//! struct into `serde`.

use bincode::config;
use lz4::block::{compress, decompress, CompressionMode};
use serde::de::{DeserializeOwned, Error, Visitor};
use serde::export::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Compression level for `lz4` compression.
///
/// We need this level as the enum `lz4` crate provides is not `Clone` nor `Copy`.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
pub enum CompressionLevel {
    Default,
    Fast(i32),
    High(i32),
}

impl Into<Option<CompressionMode>> for CompressionLevel {
    fn into(self) -> Option<CompressionMode> {
        Some(match self {
            CompressionLevel::Default => CompressionMode::DEFAULT,
            CompressionLevel::Fast(t) => CompressionMode::FAST(t),
            CompressionLevel::High(t) => CompressionMode::HIGHCOMPRESSION(t),
        })
    }
}

/// Wrapper struct that causes the wrapped type to be converted to
/// bytes using `bincode` crate and compressed using `lz4` when this
/// struct is serialized.
///
/// The similar process happens when this struct is deserialized.
///
/// Note: no parameters in the `T` type can be borrowed because
/// this decompression process involves allocation.
#[derive(Copy, Clone, Debug)]
pub struct Compressed<T>(T, CompressionLevel);

impl<T: Eq> PartialEq for Compressed<T> {
    fn eq(&self, other: &Self) -> bool {
        other.0.eq(&self.0)
    }
}

impl<T: Eq> Eq for Compressed<T> {}

impl<T: Hash> Hash for Compressed<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.0.hash(state)
    }
}

impl<T> Compressed<T> {
    /// Creates a new `Compressed` wrapped with specified data and default
    /// compression level.
    ///
    /// You can specify the compression level manually by using
    /// `new_with_compression_level` function.
    pub fn new(t: T) -> Self {
        Self::new_with_compression_level(t, CompressionLevel::High(17))
    }

    /// Creates a new `Compressed` wrapped with specified data and specified
    /// compression level.
    pub fn new_with_compression_level(t: T, lvl: CompressionLevel) -> Self {
        Self(t, lvl)
    }

    /// Converts this struct into `T`.
    pub fn into(self) -> T {
        self.0
    }
}

impl<T> Serialize for Compressed<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        // cannot serialize ZSTs because lz4-sys causes segfault.
        assert!(std::mem::size_of::<T>() > 0);

        // 1. convert the `T` to bytes using `bincode`
        // 2. compress the serialized bytes using `lz4`

        let serialized = config().little_endian().serialize(&self.0).ok().unwrap();
        let compressed = compress(serialized.as_slice(), self.1.into(), true)
            .ok()
            .unwrap();

        serializer.serialize_bytes(compressed.as_slice())
    }
}

struct CompressedVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for CompressedVisitor<T>
where
    T: DeserializeOwned,
{
    type Value = Compressed<T>;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_fmt(format_args!("Compressed<{}>", std::any::type_name::<T>()))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        // 1. decompress bytes using `lz4`
        // 2. deserialize decompressed bytes to `Compressed<T>` using `bincode`

        let decompressed = decompress(v, None).ok().unwrap();
        let deserialized: T = config()
            .little_endian()
            .deserialize(decompressed.as_slice())
            .ok()
            .unwrap();

        Ok(Compressed(deserialized, CompressionLevel::Default))
    }
}

impl<'de, T> Deserialize<'de> for Compressed<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(CompressedVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use quickcheck_macros::quickcheck;

    use crate::lz4::Compressed;
    use bincode::{deserialize, serialize};
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_basic_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct ExtraData {
            owned_integer: u64,
            big_array_1: Vec<u8>,
            big_array_2: Vec<u8>,
            big_array_3: Vec<u8>,
            big_array_4: Vec<u8>,
        }

        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct Data {
            number: u32,
            extra_data: Compressed<ExtraData>,
        }

        let value = Data {
            number: 123,
            extra_data: Compressed::new(ExtraData {
                owned_integer: 123_456_798u64,
                big_array_1: vec![0u8; 255],
                big_array_2: vec![1u8; 255],
                big_array_3: vec![2u8; 255],
                big_array_4: vec![3u8; 255],
            }),
        };

        let serialized = serialize(&value).unwrap();
        let deserialized: Data = deserialize(serialized.as_slice()).unwrap();

        assert_eq!(value.extra_data.into(), deserialized.extra_data.into());
    }

    #[test]
    #[should_panic]
    #[allow(clippy::unit_cmp)]
    fn test_empty_should_panic() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct Data {
            number: u32,
            extra_data: Compressed<()>,
        }

        let value = Data {
            number: 456,
            extra_data: Compressed::new(()),
        };

        let serialized = serialize(&value).unwrap();
        let deserialized: Data = deserialize(serialized.as_slice()).unwrap();

        assert_eq!(value.extra_data.0, deserialized.extra_data.0);
    }

    #[test]
    fn test_primitive() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct Data {
            number: u32,
            extra_data: Compressed<u64>,
        }

        let value = Data {
            number: 456,
            extra_data: Compressed::new(111_222_333_444),
        };

        let serialized = serialize(&value).unwrap();
        let deserialized: Data = deserialize(serialized.as_slice()).unwrap();

        assert_eq!(value.extra_data.0, deserialized.extra_data.0);
    }

    #[quickcheck]
    fn test_random(n1: u32, n2: u8, data_inner: Vec<u8>) -> bool {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct Data {
            number: u32,
            number2: u8,
            extra_data: Compressed<ByteData>,
        }
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct ByteData {
            n1: u32,
            n2: u8,
            #[serde(with = "serde_bytes")]
            data: Vec<u8>,
        }

        let value = Data {
            number: n1,
            number2: n2,
            extra_data: Compressed::new(ByteData {
                n1,
                n2,
                data: data_inner,
            }),
        };

        let serialized = serialize(&value).unwrap();
        let deserialized: Data = deserialize(serialized.as_slice()).unwrap();

        value == deserialized
    }
}
