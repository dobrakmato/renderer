use bf::image::{Format, Image};
use bf::material::Material;
use bf::mesh::Mesh;
use bf::{load_bf_from_bytes, Container, Data};
use image::dxt::{DXTVariant, DxtDecoder};
use image::{DynamicImage, ImageBuffer, ImageDecoder, ImageFormat};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "bfinfo")]
struct Opt {
    #[structopt(short, long)]
    dump: bool,

    #[structopt(short, long)]
    unpack_normal_map: bool,

    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt = Opt::from_args();
    let bytes = std::fs::read(opt.input).unwrap();
    let file = load_bf_from_bytes(bytes.as_slice()).unwrap();

    println!("magic={} (ok)", file.magic());
    println!("version={}", file.version());
    println!("compressed={}", file.is_compressed());

    let container = file.into_container();

    match container {
        Container::Image(i) => handle_image(i, opt.dump, opt.unpack_normal_map),
        Container::Mesh(g) => handle_mesh(g),
        Container::Material(m) => handle_material(m),
    }
}

fn handle_image(image: Image, dump: bool, unpack: bool) {
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

            let dxt = |variant| {
                let decoder = DxtDecoder::new(mipmap.data, width, height, variant)
                    .expect("cannot create dxt decoder");
                let mut raw = vec![0; decoder.total_bytes() as usize];
                decoder
                    .read_image(&mut raw)
                    .expect("cannot decode dxt data");
                raw
            };

            let raw = match image.format {
                Format::SrgbDxt1 | Format::Dxt1 => dxt(DXTVariant::DXT1),
                Format::SrgbDxt3 | Format::Dxt3 => dxt(DXTVariant::DXT3),
                Format::SrgbDxt5 | Format::Dxt5 => dxt(DXTVariant::DXT5),
                _ => Vec::from(mipmap.data),
            };

            let img = match image.format.channels() {
                1 => DynamicImage::ImageLuma8(ImageBuffer::from_raw(width, height, raw).unwrap()),
                3 => DynamicImage::ImageRgb8(ImageBuffer::from_raw(width, height, raw).unwrap()),
                4 => DynamicImage::ImageRgba8(ImageBuffer::from_raw(width, height, raw).unwrap()),
                _ => panic!("cannot dump with {} channels", image.format.channels()),
            };

            // unpack dxt5nm
            let img = if unpack {
                let mut img = img.to_rgba();

                for x in img.pixels_mut() {
                    let snap = *x;

                    let swizzle_a = snap[3] as f32 / 255.0;
                    let swizzle_g = snap[1] as f32 / 255.0;
                    let derive_b = (1.0
                        - ((swizzle_a * swizzle_a) - (swizzle_g * swizzle_g))
                            .min(1.0)
                            .max(0.0))
                    .sqrt();

                    fn clamp_color(col: f32) -> u8 {
                        (col as u32).min(255).max(0) as u8
                    }

                    x.0[0] = clamp_color(swizzle_a * 255.0);
                    x.0[1] = clamp_color(swizzle_g * 255.0);
                    x.0[2] = clamp_color(derive_b * 255.0);
                    x.0[3] = 255;
                }

                DynamicImage::ImageRgba8(img)
            } else {
                img
            };

            img.save_with_format(format!("dump_mipmap{}.png", idx), ImageFormat::Png)
                .expect("cannot save dumped file");
        }
    }
}

fn handle_mesh(geo: Mesh) {
    println!("mesh");

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

fn handle_material(material: Material) {
    println!("material");

    println!("{:#?}", material)
}
