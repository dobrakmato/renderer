use bf::material::{BlendMode, Material};
use bf::{save_bf_to_bytes, Container, File};
use std::path::PathBuf;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(StructOpt, Debug)]
#[structopt(name = "matcomp")]
pub struct MatCompParameters {
    /// Output file (.bf)
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long, parse(try_from_str = parse_blend_mode))]
    blend_mode: Option<BlendMode>,

    #[structopt(long, parse(try_from_str = parse_color))]
    albedo_color: Option<[f32; 3]>,

    #[structopt(long)]
    roughness: Option<f32>,

    #[structopt(long)]
    metallic: Option<f32>,

    #[structopt(long)]
    alpha_cutoff: Option<f32>,

    #[structopt(long)]
    opacity: Option<f32>,

    #[structopt(long)]
    ior: Option<f32>,

    #[structopt(long)]
    sss: Option<f32>,

    #[structopt(long)]
    albedo_map: Option<String>,

    #[structopt(long)]
    normal_map: Option<String>,

    #[structopt(long)]
    displacement_map: Option<String>,

    #[structopt(long)]
    roughness_map: Option<String>,

    #[structopt(long)]
    opacity_map: Option<String>,

    #[structopt(long)]
    ao_map: Option<String>,

    #[structopt(long)]
    metallic_map: Option<String>,
}

fn parse_blend_mode(src: &str) -> Result<BlendMode, &'static str> {
    match src.to_lowercase().as_str() {
        "opaque" => Ok(BlendMode::Opaque),
        "masked" => Ok(BlendMode::Masked),
        "translucent" => Ok(BlendMode::Translucent),
        _ => Err("invalid blend mode"),
    }
}

fn parse_color(src: &str) -> Result<[f32; 3], &'static str> {
    let mut itr = src.split(',');
    let mut parse = || {
        itr.next()
            .unwrap()
            .parse::<f32>()
            .expect("cannot parse float")
    };
    Ok([parse(), parse(), parse()])
}

fn parse_uuid(str: Option<String>) -> Option<Uuid> {
    str.map(|x| Uuid::parse_str(x.as_str()).expect("cannot parse uuid"))
}

fn main() {
    let params = MatCompParameters::from_args();
    let material = Material {
        blend_mode: params.blend_mode.unwrap_or(BlendMode::Opaque),
        albedo_color: params.albedo_color.unwrap_or([1.0, 1.0, 1.0]),
        roughness: params
            .roughness
            .unwrap_or(if params.roughness_map.is_none() {
                0.5
            } else {
                1.0
            }),
        metallic: params.metallic.unwrap_or(if params.metallic_map.is_none() {
            0.0
        } else {
            1.0
        }),
        opacity: params.opacity.unwrap_or(1.0),
        ior: params.opacity.unwrap_or(1.0),
        sss: params.sss.unwrap_or(0.0),
        alpha_cutoff: params.alpha_cutoff.unwrap_or(0.5),
        albedo_map: parse_uuid(params.albedo_map),
        normal_map: parse_uuid(params.normal_map),
        displacement_map: parse_uuid(params.displacement_map),
        roughness_map: parse_uuid(params.roughness_map),
        ao_map: parse_uuid(params.ao_map),
        metallic_map: parse_uuid(params.metallic_map),
        opacity_map: parse_uuid(params.opacity_map),
    };

    let file = File::create_uncompressed(Container::Material(material));
    let bytes = save_bf_to_bytes(&file).expect("cannot convert bf::material::Material");

    std::fs::write(params.output, bytes).expect("cannot save file!");
}
