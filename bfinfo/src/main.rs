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
    println!("format={:?}", image.format);
    println!("mipmaps={}", image.mipmap_count());

    for (idx, mipmap) in image.mipmaps().enumerate() {
        let size = mipmap.width * mipmap.height * image.format.bits_per_pixel() as usize / 8;
        println!(
            "mipmap level={} width={} height={} size={}",
            idx, mipmap.width, mipmap.height, size
        );

        if dump {
            let width = mipmap.width as u32;
            let height = mipmap.height as u32;

            let decoder = DXTDecoder::new(mipmap.data, width, height, DXTVariant::DXT1)
                .expect("cannot create dxt decoder");
            let raw = decoder.read_image().expect("cannot decode dxt data");
            let img = ImageBuffer::from_raw(width, height, raw)
                .map(DynamicImage::ImageRgb8)
                .expect("cannot create image buffer from decoded data");
            img.save_with_format(format!("dump_mipmap{}.png", idx), ImageFormat::PNG)
                .expect("cannot save dumped file");
        }
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
