use crate::tool::Img2Bf;
use bf::image::Format;
use image::imageops::FilterType;
use std::path::PathBuf;
use structopt::StructOpt;

#[macro_use]
mod perf;
mod tool;

/// You can use destination parameters to swizzle channels around or replace some channel
/// with a constant.
#[derive(StructOpt, Debug)]
#[structopt(name = "img2bf")]
pub struct Img2BfParameters {
    /// Input file (.jpeg, .png, .bmp, ...)
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// Output file (.bf)
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Desired conversion format (eg. "dxt1")
    #[structopt(short, long, parse(try_from_str = parse_format))]
    format: Format,

    /// Filter that will be used when downscaling mip-maps
    #[structopt(short, long, parse(try_from_str = parse_mip_filter))]
    mip_filter: Option<FilterType>,

    /// Whether to vertically flip image data
    #[structopt(short, long)]
    v_flip: bool,

    /// Whether to pack input image as normal map (DXT5nm).
    #[structopt(short, long)]
    pack_normal_map: bool,

    /// Swizzle destination: red channel
    #[structopt(short, long)]
    destination_r: Option<String>,

    /// Swizzle destination: green channel
    #[structopt(short, long)]
    destination_g: Option<String>,

    /// Swizzle destination: blue channel
    #[structopt(short, long)]
    destination_b: Option<String>,

    /// Swizzle destination: alpha channel
    #[structopt(short, long)]
    destination_a: Option<String>,
}

fn parse_format(src: &str) -> Result<Format, &'static str> {
    match src.to_lowercase().as_str() {
        "bc1" | "dxt1" => Ok(Format::Dxt1),
        "bc2" | "dxt3" => Ok(Format::Dxt3),
        "bc3" | "dxt5" => Ok(Format::Dxt5),
        "bc6h" => Ok(Format::BC6H),
        "bc7" => Ok(Format::BC7),
        "r8" => Ok(Format::R8),
        "rgb" => Ok(Format::Rgb8),
        "rgba" => Ok(Format::Rgba8),
        "srgb_dxt1" => Ok(Format::SrgbDxt1),
        "srgb_dxt3" => Ok(Format::SrgbDxt3),
        "srgb_dxt5" => Ok(Format::SrgbDxt5),
        "srgb_bc7" => Ok(Format::SrgbBC7),
        "srgb" => Ok(Format::Rgb8),
        "srgba" => Ok(Format::Rgba8),
        _ => Err("unknown format"),
    }
}

fn parse_mip_filter(src: &str) -> Result<FilterType, &'static str> {
    match src.to_lowercase().as_str() {
        "nearest" => Ok(FilterType::Nearest),
        "linear" => Ok(FilterType::Triangle),
        "gaussian" => Ok(FilterType::Gaussian),
        "cubic" => Ok(FilterType::CatmullRom),
        "lanczos" => Ok(FilterType::Lanczos3),
        _ => Err("unknown fitler type"),
    }
}

fn main() {
    let params = Img2BfParameters::from_args();
    let stats = Img2Bf::convert(params).expect("conversion failed!");

    println!("load={}ms", stats.load.total_time().as_millis());
    println!("vflip={}ms", stats.vflip.total_time().as_millis());
    println!("channels={}ms", stats.channels.total_time().as_millis());
    println!("swizzle={}ms", stats.swizzle.total_time().as_millis());
    println!("mipmaps={}ms", stats.mipmaps.total_time().as_millis());
    println!("dxt={}ms", stats.dxt.total_time().as_millis());
    println!("save={}ms", stats.save.total_time().as_millis());
}
