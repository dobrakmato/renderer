use std::path::PathBuf;

use crate::perf::Stopwatch;
use bf::{save_bf_to_bytes, Container, File, Format, Image};
use image::dxt::{DXTEncoder, DXTVariant};
use image::{DynamicImage, FilterType, GenericImageView};
use structopt::StructOpt;

mod perf;

#[derive(StructOpt, Debug)]
#[structopt(name = "img2bf")]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(short, long, parse(try_from_str = parse_format))]
    format: Format,

    #[structopt(short, long)]
    not_v_flip: bool,
}

fn parse_format(src: &str) -> Result<Format, &'static str> {
    match src.to_lowercase().as_str() {
        "dxt1" => Ok(Format::Dxt1),
        "dxt3" => Ok(Format::Dxt3),
        "dxt5" => Ok(Format::Dxt5),
        "rgb" => Ok(Format::Rgb8),
        "rgba" => Ok(Format::Rgba8),
        "srgb_dxt1" => Ok(Format::SrgbDxt1),
        "srgb_dxt3" => Ok(Format::SrgbDxt3),
        "srgb_dxt5" => Ok(Format::SrgbDxt5),
        "srgb" => Ok(Format::Rgb8),
        "srgba" => Ok(Format::Rgba8),
        _ => Err("unknown format"),
    }
}

struct Timers<'a> {
    load: Stopwatch<'a>,
    vflip: Stopwatch<'a>,
    channels: Stopwatch<'a>,
    mipmaps: Stopwatch<'a>,
    dxt: Stopwatch<'a>,
    save: Stopwatch<'a>,
}

impl<'a> Default for Timers<'a> {
    fn default() -> Self {
        Timers {
            load: Stopwatch::new("load"),
            vflip: Stopwatch::new("vflip"),
            channels: Stopwatch::new("channels"),
            mipmaps: Stopwatch::new("mipmaps"),
            dxt: Stopwatch::new("dxt"),
            save: Stopwatch::new("save"),
        }
    }
}

fn main() {
    let mut timers = Timers::default();
    let opt = Opt::from_args();

    // 1. load image
    timers.load.start();
    let mut input_image = image::open(&opt.input).expect("cannot load input file as image");
    timers.load.end();

    let (width, height) = (input_image.width(), input_image.height());

    println!("width={}", width);
    println!("height={}", height);
    println!("color={:?}", input_image.color());

    // 2. vflip
    timers.vflip.start();
    if !opt.not_v_flip {
        input_image = input_image.flipv();
    }
    timers.vflip.end();

    // 3. rgba <-> rgb
    timers.channels.start();
    if input_image.color().channel_count() != opt.format.channels() {
        if input_image.color().channel_count() > opt.format.channels() {
            input_image = DynamicImage::ImageRgb8(input_image.to_rgb());
        } else {
            input_image = DynamicImage::ImageRgba8(input_image.to_rgba());
        }
    }
    timers.channels.end();

    // 4. mipmaps
    timers.mipmaps.start();
    let mut mipmaps = vec![input_image];
    while mipmaps.last().unwrap().width() > 4 {
        // 4 is the minimal size for dxt texture
        let higher = mipmaps.last().unwrap();
        let lower = higher.clone().resize(
            higher.width() / 2,
            higher.height() / 2,
            FilterType::Lanczos3,
        );
        mipmaps.push(lower);
    }
    timers.mipmaps.end();

    // 5. convert to output format
    timers.dxt.start();
    let mut payload = vec![];
    for img in mipmaps {
        let raw = img.raw_pixels();
        let raw = raw.as_slice();

        let dxt = |variant| {
            let mut storage: Vec<u8> = vec![];
            DXTEncoder::new(&mut storage)
                .encode(raw, img.width(), img.height(), variant)
                .expect("dxt compression failed");
            storage
        };

        let result = match opt.format {
            // we need to perform dxt compression
            Format::SrgbDxt1 | Format::Dxt1 => dxt(DXTVariant::DXT1),
            Format::SrgbDxt3 | Format::Dxt3 => dxt(DXTVariant::DXT3),
            Format::SrgbDxt5 | Format::Dxt5 => dxt(DXTVariant::DXT5),

            // for uncompressed formats we just copy the buffer
            _ => Vec::from(raw), // todo: optimize needless copy
        };

        payload.extend(result);
    }
    timers.dxt.end();

    // 6. write file_out
    timers.save.start();
    let file = File::create_compressed(Container::Image(Image {
        format: opt.format,
        width: width as u16,
        height: height as u16,
        mipmap_data: payload.as_slice(),
    }));

    std::fs::write(
        opt.output.unwrap_or(opt.input.with_extension("bf")),
        save_bf_to_bytes(&file).expect("cannot serialize image"),
    )
    .expect("cannot write data to disk");
    timers.save.end();

    println!("time load={}ms", timers.load.total_time().as_millis());
    println!("time vflip={}ms", timers.vflip.total_time().as_millis());
    println!(
        "time channels={}ms",
        timers.channels.total_time().as_millis()
    );
    println!("time mipmaps={}ms", timers.mipmaps.total_time().as_millis());
    println!("time dxt={}ms", timers.dxt.total_time().as_millis());
    println!("time save={}ms", timers.save.total_time().as_millis());
}
