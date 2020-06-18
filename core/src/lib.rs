//! Core functionality shared between all (or many) of the renderer crates.

pub mod notification;
pub mod perf;

/// Statically asserts that the alignment of specified type is
/// specified number of bytes.
///
/// # Example
///
/// This code will compile just fine.
///
/// ```
/// # use core::assert_alignment;
/// assert_alignment!(u8, 1);
/// assert_alignment!(u16, 2);
/// assert_alignment!(u32, 4);
/// ```
///
/// This will however fail to compile as alignemnt of `u32` is not 8 bytes.
///
/// ```compile_fail
/// # use core::assert_alignment;
/// assert_alignment!(u32, 8);
/// ```
#[macro_export]
macro_rules! assert_alignment {
    ($typ:ty, $to:expr) => {
        const _: fn() = || {
            let _: [(); std::mem::align_of::<$typ>()] = [(); $to];
        };
    };
}
