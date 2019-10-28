#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate zerocopy_derive;

mod image;
mod geometry;
mod header;
mod file;
mod kind;

pub use kind::Kind;
pub use file::{File, load_bf_from_bytes, Error};
pub use header::{Header, BF_MAGIC, BF_MAX_SUPPORTED_VERSION};
pub use geometry::{GeometryList, GeometryListType};
pub use image::{ImageAdditional, ImageType, Format, ColorSpace};
