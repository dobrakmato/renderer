use std::fs::File;
use std::io::Read;

use bf::{load_bf_from_bytes, Format, ImageAdditional, Kind};
use clap::{App, Arg};
use image::dxt::{DXTDecoder, DXTVariant};
use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageFormat};
use lz4::block::decompress;
use std::convert::TryFrom;

fn main() {
    let matches = App::new("bfinfo")
        .version("1.0")
        .author("Matej K. <dobrakmato@gmail.com>")
        .about("Inspect various BF files")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("INPUT_FILE")
                .help("Path to the file to inspect")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dump")
                .short("d")
                .long("dump")
                .help("Dump contents of the file"),
        )
        .get_matches();

    let mut file = File::open(matches.value_of("input").unwrap())
        .map_err(|e| panic!("cannot open input file: {}", e))
        .unwrap();

    let mut cnts = vec![];
    file.read_to_end(&mut cnts).expect("read error");

    let file = load_bf_from_bytes(&cnts)
        .map_err(|e| panic!("cannot decode input file: {:?}", e))
        .unwrap();
    let header = file.header;
    let payload = file.data;

    let kind = Kind::try_from(header.kind)
        .map_err(|_| panic!("invalid kind value: {}", header.kind))
        .unwrap();

    println!("magic={}", header.magic);
    println!("version={}", header.version);
    println!("kind={:?}", kind);

    match kind {
        Kind::Image => println!(
            "additional={:?}",
            ImageAdditional::from_u64(header.additional)
        ),
        _ => println!("additional={}", header.additional),
    }

    println!("uncompressed={}", header.uncompressed);
    println!("compressed={}", header.compressed);

    let uncompressed = decompress(payload, Some(header.uncompressed as i32))
        .map_err(|e| panic!("payload decompression failed: {}", e))
        .unwrap();

    if let Kind::Image = kind {
        let additional = ImageAdditional::from_u64(header.additional);
        let mut level = 0;
        let format = Format::try_from(additional.format).expect("invalid format value");
        let mut width = additional.width;
        let mut height = additional.height;
        let mut index = 0;
        while index < uncompressed.len() {
            let size = width as usize * height as usize * format.bits_per_pixel() / 8;
            println!(
                "mipmap level={} width={} height={} size={}",
                level, width, height, size
            );
            let mipmap = &uncompressed[index..index + size];

            if matches.is_present("dump") {
                let decoder =
                    DXTDecoder::new(mipmap, width as u32, height as u32, DXTVariant::DXT1)
                        .map_err(|e| panic!("cannot create dxt decoder: {}", e))
                        .unwrap();
                let raw = decoder
                    .read_image()
                    .map_err(|e| panic!("cannot decode dxt data: {}", e))
                    .unwrap();
                let img = ImageBuffer::from_raw(width as u32, height as u32, raw)
                    .map(DynamicImage::ImageRgb8)
                    .expect("cannot create image buffer from decoded data");
                img.save_with_format(format!("dump_mipmap{}.png", level), ImageFormat::PNG)
                    .map_err(|e| panic!("cannot save dumped file: {}", e))
                    .unwrap();
            }

            width /= 2;
            height /= 2;
            level += 1;
            index += size;
        }
    } else {
        eprintln!("Sorry, this type is not yet supported!");
    }
}
