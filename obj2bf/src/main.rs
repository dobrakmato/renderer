use std::path::{Path, PathBuf};

use clap::{App, Arg, ArgMatches};

use crate::geo::Geometry;
use crate::perf::Stopwatch;
use std::fs;
use wavefront_obj::obj::parse;

mod geo;
mod perf;

/// Derives output path from input path by changing the file's extension.
pub fn derive_output_from(input: &str) -> PathBuf {
    let stem = Path::new(input)
        .file_stem()
        .expect("input file is not a valid file");

    let mut owned = stem.to_owned();
    owned.push(".bf");
    PathBuf::from(owned)
}

/// Creates Path-like objects for input and output file from the arguments
/// passed to the application.
pub fn derive_input_and_output(matches: &ArgMatches) -> (PathBuf, PathBuf) {
    let input = matches.value_of("input").unwrap();
    let output = match matches.value_of("output") {
        None => derive_output_from(input),
        Some(t) => PathBuf::from(t),
    };
    let input = PathBuf::from(input);
    (input, output)
}

struct Timers<'a> {
    load: Stopwatch<'a>,
    lods: Stopwatch<'a>,
    normalize: Stopwatch<'a>,
    optimize: Stopwatch<'a>,
    lz4: Stopwatch<'a>,
    save: Stopwatch<'a>,
}

impl<'a> Default for Timers<'a> {
    fn default() -> Self {
        Timers {
            load: Stopwatch::new("load"),
            lods: Stopwatch::new("lods"),
            normalize: Stopwatch::new("normalize"),
            optimize: Stopwatch::new("optimize"),
            lz4: Stopwatch::new("lz4"),
            save: Stopwatch::new("save"),
        }
    }
}

fn main() {
    let mut timers = Timers::default();

    let matches = App::new("obj2bf")
        .version("1.0")
        .author("Matej K. <dobrakmato@gmail.com>")
        .about("Converts OBJ file format to BF optimized format")
        .arg(
            Arg::with_name("content")
                .long("content")
                .value_name("CONTENT_PATH")
                .help("Specifies the content root directory to import the file into")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("INPUT_FILE")
                .help("Path to file to convert / import")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT_FILE")
                .help("Path to output file to generate")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("LOD_LEVELS")
                .long("lod-levels")
                .help("Specify number of LOD levels generated")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("optimize")
                .long("optimize")
                .help("Optimize the mesh for cache accesses"),
        )
        .get_matches();

    let (input, output) = derive_input_and_output(&matches);

    timers.load.start();
    let cnts = std::fs::read_to_string(&input)
        .map_err(|e| panic!("cannot read file: {}", e))
        .unwrap();
    let obj = parse(cnts)
        .map_err(|e| panic!("cannot parse obj file: {:?}", e))
        .unwrap();
    timers.load.end();

    println!("objects={}", obj.objects.len());

    let obj = obj
        .objects
        .iter()
        .find(|it| !it.geometry.is_empty())
        .expect("no object with non-empty geometry found!");
    let mut geo = Geometry::from(obj);

    println!("geo.positions={}", geo.positions.len());
    println!("geo.normals={}", geo.normals.len());
    println!("geo.tex_coords={}", geo.tex_coords.len());
    println!("geo.indices={}", geo.indices.len());

    // todo: generate lods (simplify mesh)
    // todo: optimize meshes (forsyth)

    // generate deduped indices
    // geo.dedupe_vertices();

    // rewrite to indexed (duplicate values)

    // compress
    // save
    fs::write("dump.obj", geo.to_obj());

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
    println!("time lz4={}ms", timers.lz4.total_time().as_millis());
    println!("time save={}ms", timers.save.total_time().as_millis());
}
