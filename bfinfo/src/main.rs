use bf::{load_bf_from_bytes, Container, Data, Geometry, Image};
use image::dxt::{DXTDecoder, DXTVariant};
use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageFormat};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "bfinfo")]
struct Opt {
    #[structopt(short, long)]
    dump: bool,

    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt = Opt::from_args();
    let bytes = std::fs::read(opt.input).unwrap();
    let file = load_bf_from_bytes(bytes.as_slice()).unwrap();

    println!("magic={} (ok)", file.magic);
    println!("version={}", file.version);

    let container = match file.data {
        Data::Compressed(c) => c.0,
        Data::Uncompressed(c) => c,
    };

    match container {
        Container::Image(i) => handle_image(i, opt.dump),
        Container::Geometry(g) => handle_geometry(g),
    }
}

fn handle_image(image: Image, dump: bool) {
    println!("image");
    println!("mipmaps={}", image.mipmap_count());

    let mut width = image.width;
    let mut height = image.height;
    let mut level = 0;
    let mut index = 0;
    while index < image.mipmap_data.len() {
        let size = width as usize * height as usize * image.format.bits_per_pixel() as usize / 8;
        println!(
            "mipmap level={} width={} height={} size={}",
            level, width, height, size
        );
        let mipmap = &image.mipmap_data[index..index + size];

        if dump {
            let decoder = DXTDecoder::new(mipmap, width as u32, height as u32, DXTVariant::DXT1)
                .expect("cannot create dxt decoder");
            let raw = decoder.read_image().expect("cannot decode dxt data");
            let img = ImageBuffer::from_raw(width as u32, height as u32, raw)
                .map(DynamicImage::ImageRgb8)
                .expect("cannot create image buffer from decoded data");
            img.save_with_format(format!("dump_mipmap{}.png", level), ImageFormat::PNG)
                .expect("cannot save dumped file");
        }

        width /= 2;
        height /= 2;
        level += 1;
        index += size;
    }
}

fn handle_geometry(geo: Geometry) {
    println!("geometry");

    println!("vertex_data_format={:?}", geo.vertex_format);
    println!("index_type={:?}", geo.index_type);
    println!(
        "vertices={}",
        geo.vertex_data.len() / geo.vertex_format.size_of_one_vertex()
    );
    println!(
        "indices={}",
        geo.index_data.len() / geo.index_type.size_of_one_index()
    );
}
