use bf::image::Format as BfFormat;
use vulkano::format::Format;

/// Trait used to implement conversion from `bf::Format` to
/// `vulkano::format::Format`.
pub trait ToVulkanFormat {
    fn to_vulkan_format(&self) -> Format;
}

impl ToVulkanFormat for BfFormat {
    fn to_vulkan_format(&self) -> Format {
        match self {
            BfFormat::R8 => Format::R8Unorm,
            BfFormat::Dxt1 => Format::BC1_RGBUnormBlock,
            BfFormat::Dxt3 => Format::BC2UnormBlock,
            BfFormat::Dxt5 => Format::BC3UnormBlock,
            BfFormat::Rgb8 => Format::R8G8B8Unorm,
            BfFormat::Rgba8 => Format::R8G8B8A8Unorm,
            BfFormat::SrgbDxt1 => Format::BC1_RGBSrgbBlock,
            BfFormat::SrgbDxt3 => Format::BC2SrgbBlock,
            BfFormat::SrgbDxt5 => Format::BC3SrgbBlock,
            BfFormat::Srgb8 => Format::R8G8B8Srgb,
            BfFormat::Srgb8A8 => Format::R8G8B8A8Srgb,
            BfFormat::BC6H => Format::BC6HUfloatBlock,
            BfFormat::BC7 => Format::BC7UnormBlock,
            BfFormat::SrgbBC7 => Format::BC7SrgbBlock,
        }
    }
}
