use crate::perf::Stopwatch;
use bf::{save_bf_to_bytes, Container, File, Geometry, IndexType, VertexDataFormat};
use std::convert::TryFrom;
use std::path::PathBuf;
use structopt::StructOpt;
use wavefront_obj::obj::parse;

mod geo;
mod math;
mod perf;

#[derive(StructOpt, Debug)]
#[structopt(name = "obj2bf")]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

struct Timers<'a> {
    load: Stopwatch<'a>,
    lods: Stopwatch<'a>,
    normalize: Stopwatch<'a>,
    optimize: Stopwatch<'a>,
    save: Stopwatch<'a>,
}

impl<'a> Default for Timers<'a> {
    fn default() -> Self {
        Timers {
            load: Stopwatch::new("load"),
            lods: Stopwatch::new("lods"),
            normalize: Stopwatch::new("normalize"),
            optimize: Stopwatch::new("optimize"),
            save: Stopwatch::new("save"),
        }
    }
}

fn main() {
    let mut timers = Timers::default();
    let opt = Opt::from_args();

    timers.load.start();
    let cnts = std::fs::read_to_string(&opt.input).expect("cannot read input file");
    let obj = parse(cnts).expect("cannot parse input file");
    timers.load.end();

    println!("objects={}", obj.objects.len());

    timers.normalize.start();
    let obj = obj
        .objects
        .iter()
        .find(|it| !it.geometry.is_empty())
        .expect("no object with non-empty geometry found!");
    let geo = geo::Geometry::try_from(obj).ok().unwrap();
    timers.normalize.end();

    println!("geo.positions={}", geo.positions.len());
    println!("geo.normals={}", geo.normals.len());
    println!("geo.tex_coords={}", geo.tex_coords.len());
    println!("geo.indices={}", geo.indices.len());

    // todo: generate lods (simplify mesh)
    // todo: optimize meshes (forsyth)

    // compress
    // save
    std::fs::write("dump.obj", geo.to_obj()).unwrap();

    timers.save.start();
    let vertex_format = VertexDataFormat::PositionNormalUv;
    let vertex_data = geo.generate_vertex_data(vertex_format);
    let index_type = geo.suggest_index_type();
    let index_data = geo.generate_index_data(index_type);

    let file = File::create_compressed(Container::Geometry(Geometry {
        vertex_format,
        index_type,
        vertex_data: vertex_data.as_slice(),
        index_data: index_data.as_slice(),
    }));

    std::fs::write(
        opt.output.unwrap_or(opt.input.with_extension("bf")),
        save_bf_to_bytes(&file).expect("cannot serialize image"),
    )
    .expect("cannot write data to disk");
    timers.save.end();

    //println!("raw={} compressed={} ratio={}", bf_header.uncompressed, bf_header.compressed, 100.0 * bf_header.compressed as f32 / bf_header.uncompressed as f32);
    println!("time load={}ms", timers.load.total_time().as_millis());
    println!("time lods={}ms", timers.lods.total_time().as_millis());
    println!(
        "time normalize={}ms",
        timers.normalize.total_time().as_millis()
    );
    println!(
        "time optimize={}ms",
        timers.optimize.total_time().as_millis()
    );
    println!("time save={}ms", timers.save.total_time().as_millis());
}
