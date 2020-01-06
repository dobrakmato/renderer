use bincode::config;
use lz4::block::{compress, decompress, CompressionMode};
use serde::de::Visitor;
use serde::export::fmt::Error;
use serde::export::Formatter;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::pin::Pin;

/// Marker struct specifying that data inside it should be serialized
/// and LZ4 compressed when this struct is serialized.
pub struct Compressed<T>(pub T, Option<Pin<Vec<u8>>>);

impl<T> PartialEq for Compressed<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Compressed<T> where T: Eq {}

impl<T> Debug for Compressed<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_fmt(format_args!("Compressed({:?})", self.0))
    }
}

impl<T> Compressed<T> {
    pub fn new(value: T) -> Self {
        Compressed(value, None)
    }

    fn decompress<'a>(bytes: &'a [u8]) -> Compressed<T>
    where
        T: Deserialize<'a>,
    {
        // todo: verify safety
        let decompressed = decompress(bytes, None).unwrap();
        let mem = MaybeUninit::<T>::zeroed();
        let mut obj = Compressed(unsafe { mem.assume_init() }, Some(Pin::new(decompressed)));
        let reference = unsafe {
            std::slice::from_raw_parts(
                obj.1.as_ref().unwrap().as_ptr(),
                obj.1.as_ref().unwrap().len(),
            )
        };

        let deserialized: T = config().little_endian().deserialize(reference).unwrap();

        unsafe { std::ptr::write(&mut obj.0 as *mut T, deserialized) }

        obj
    }
}

impl<T> Serialize for Compressed<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        assert_ne!(std::mem::size_of::<T>(), 0);

        let serialized = config().little_endian().serialize(&self.0).unwrap();
        let compressed = compress(
            serialized.as_slice(),
            Some(CompressionMode::HIGHCOMPRESSION(17)),
            true,
        )
        .unwrap();
        serializer.serialize_bytes(compressed.as_slice())
    }
}

struct CompressedVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for CompressedVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Compressed<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("compressed {}", std::any::type_name::<T>()))
    }

    fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        assert_ne!(std::mem::size_of::<T>(), 0);

        Ok(Compressed::decompress(value))
    }
}

impl<'de, T> Deserialize<'de> for Compressed<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Compressed<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(CompressedVisitor(PhantomData::default()))
    }
}

#[cfg(test)]
mod tests {
    use crate::lz4::Compressed;
    use bincode::{deserialize, serialize};
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_basic_struct() {
        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct ExtraData<'a> {
            owned_integer: u64,
            big_array_1: &'a [u8],
            big_array_2: &'a [u8],
            big_array_3: &'a [u8],
            big_array_4: &'a [u8],
        }

        #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
        struct Data<'a> {
            number: u32,
            #[serde(borrow)]
            extra_data: Compressed<ExtraData<'a>>,
        }

        let value = Data {
            number: 123,
            extra_data: Compressed::new(ExtraData {
                owned_integer: 123_456_798u64,
                big_array_1: &[0u8; 255],
                big_array_2: &[1u8; 255],
                big_array_3: &[2u8; 255],
                big_array_4: &[3u8; 255],
            }),
        };

        let serialized = serialize(&value).unwrap();
        let deserialized: Data = deserialize(serialized.as_slice()).unwrap();

        assert_eq!(value.extra_data.0, deserialized.extra_data.0);
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
}
