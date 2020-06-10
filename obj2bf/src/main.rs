use crate::tool::Obj2Bf;
use bf::mesh::{IndexType, VertexFormat};
use std::path::PathBuf;
use structopt::StructOpt;

mod geo;
mod math;
#[macro_use]
mod perf;
mod tool;

#[derive(StructOpt, Debug)]
#[structopt(name = "obj2bf")]
pub struct Obj2BfParameters {
    /// Input file (.obj, .fbx).
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// Output file (.bf)
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Index type to use.
    #[structopt(long, parse(try_from_str = parse_index_type))]
    index_type: Option<IndexType>,

    /// Vertex data format.
    #[structopt(long, parse(try_from_str = parse_vertex_format))]
    vertex_format: Option<VertexFormat>,

    /// Target level of detail (LOD). Original = 0, Worst = 255.
    #[structopt(short, long)]
    lod: Option<u8>,

    /// Name of object to import from input file. Selects first non-empty object if not specified.
    #[structopt(long)]
    object_name: Option<String>,

    /// Index of geometry to import from input file. Selects first non-empty geometry if not specified.
    #[structopt(long)]
    geometry_index: Option<usize>,

    /// Causes the application to inspect the input file and print all possible convert commands.
    #[structopt(short, long)]
    print_options: bool,
}

fn parse_index_type(src: &str) -> Result<IndexType, &'static str> {
    match src.to_lowercase().as_str() {
        "u16" => Ok(IndexType::U16),
        "u32" => Ok(IndexType::U32),
        _ => Err("unknown format"),
    }
}

fn parse_vertex_format(src: &str) -> Result<VertexFormat, &'static str> {
    match src.to_lowercase().as_str() {
        "pnut" => Ok(VertexFormat::PositionNormalUvTangent),
        _ => Err("unknown format"),
    }
}

fn main() {
    let params: Obj2BfParameters = Obj2BfParameters::from_args();

    if params.print_options {
        Obj2Bf::print_possible_commands(params).expect("read error!");
    } else {
        let stats = Obj2Bf::convert(params).expect("conversion failed!");

        println!("load={}ms", stats.load.total_time().as_millis());
        println!("lods={}ms", stats.lods.total_time().as_millis());
        println!("normalize={}ms", stats.normalize.total_time().as_millis());
        println!("optimize={}ms", stats.optimize.total_time().as_millis());
        println!("save={}ms", stats.save.total_time().as_millis());
    }
}
