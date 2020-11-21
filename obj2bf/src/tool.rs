use crate::geo::{Geometry, ObjImportError};
use crate::Obj2BfParameters;
use bf::mesh::{Mesh, VertexFormat};
use bf::{save_bf_to_bytes, Container, File};
use core::impl_stats_struct;
use core::measure_scope;
use std::convert::TryFrom;
use std::io::Error;
use wavefront_obj::obj::{parse, ObjSet, Object};
use wavefront_obj::ParseError;

// generate `Statistics` struct with `CPUProfiler`s
impl_stats_struct!(pub Statistics; load, lods, normalize, optimize, save);

// default vertex format to use when no is specified
const DEFAULT_VERTEX_FORMAT: VertexFormat = VertexFormat::PositionNormalUvTangent;

#[derive(Debug)]
pub enum Obj2BfError {
    InvalidInputFile(&'static str),
    InputFileIoError(Error),
    ObjParseError(ParseError),
    ObjectNotFound(String),
    CannotNormalizeObj(ObjImportError),
    NoNonEmptyGeometriesFound,
    SerializationError(bf::LoadError),
    SaveIOError(std::io::Error),
}

pub struct Obj2Bf {
    params: Obj2BfParameters,
    stats: Statistics<'static>,
}

impl Obj2Bf {
    /// Loads the input file and parses it as .obj file.
    fn load(&mut self) -> Result<ObjSet, Obj2BfError> {
        measure_scope!(self.stats.load);

        if self
            .params
            .input
            .extension()
            .unwrap()
            .to_string_lossy()
            .to_lowercase()
            != "obj"
        {
            return Err(Obj2BfError::InvalidInputFile("not a .obj file!"));
        }

        let obj_text =
            std::fs::read_to_string(&self.params.input).map_err(Obj2BfError::InputFileIoError)?;
        parse(obj_text).map_err(Obj2BfError::ObjParseError)
    }

    /// Select the geometry to convert from the input file.
    // need explicit lifetime annotation because compiler cannot figure out
    fn select_object<'a>(&mut self, obj_set: &'a ObjSet) -> Result<&'a Object, Obj2BfError> {
        match obj_set.objects.iter().find(|it| {
            if let Some(ref t) = self.params.object_name {
                if t == &it.name {
                    return true;
                }
            }

            !it.geometry.is_empty()
        }) {
            None => Err(Obj2BfError::ObjectNotFound(format!(
                "object with name {:?} not found in .obj file",
                self.params.object_name
            ))),
            Some(t) => Ok(t),
        }
    }

    /// Selects the geometry from object and normalizes (normals, computes tangents) it to
    /// internal representation.
    fn select_geo_and_normalize(&mut self, object: &Object) -> Result<Geometry, Obj2BfError> {
        measure_scope!(self.stats.normalize);

        // try to choose geometry index if not provided by parameters
        let geo_idx = match self.params.geometry_index {
            Some(t) => t,
            None => object
                .geometry
                .iter()
                .enumerate()
                .find(|(_, g)| !g.shapes.is_empty())
                .map(|(idx, _)| idx)
                .ok_or(Obj2BfError::NoNonEmptyGeometriesFound)?,
        };

        let mut geometry =
            Geometry::try_from((object, geo_idx)).map_err(Obj2BfError::CannotNormalizeObj)?;

        if self.params.recalculate_normals {
            geometry.recalculate_normals();
        }

        geometry.recalculate_tangents();

        Ok(geometry)
    }

    /// Chooses appropriate vertex and index formats and encodes the mesh and saves the output
    /// file.
    fn save_bf_mesh(&mut self, geo: Geometry) -> Result<(), Obj2BfError> {
        measure_scope!(self.stats.save);

        // choose vertex format or use default vertex format
        let vertex_format = self.params.vertex_format.unwrap_or(DEFAULT_VERTEX_FORMAT);
        let vertex_data = geo.generate_vertex_data(vertex_format);

        // choose index format or suggest from vertices size
        let index_type = self
            .params
            .index_type
            .unwrap_or_else(|| geo.suggest_index_type());
        let index_data = geo.generate_index_data(index_type);

        let file = File::create_compressed(Container::Mesh(Mesh {
            vertex_format,
            index_type,
            vertex_data,
            index_data,
        }));

        let default_output = self.params.input.with_extension("bf");
        let save_path = self.params.output.clone().unwrap_or(default_output);
        let save_bytes = save_bf_to_bytes(&file).map_err(Obj2BfError::SerializationError)?;

        if self.params.dump_obj {
            std::fs::write("./obj2bf_dump.obj", geo.to_obj()).expect("cannot dump .obj file");
        }

        std::fs::write(save_path, save_bytes).map_err(Obj2BfError::SaveIOError)
    }

    /// Calling this method performs the conversion specified by `Obj2BfParameters` parameter.
    /// If the conversion is successful the `Statistics` object will be returned which
    /// contains statistic information about the conversion. Error will be returned otherwise.
    pub fn convert(params: Obj2BfParameters) -> Result<Statistics<'static>, Obj2BfError> {
        let mut tool = Obj2Bf {
            params,
            stats: Statistics::default(),
        };

        // todo: add support for importing materials

        let obj_set = tool.load()?;
        let object = tool.select_object(&obj_set)?;
        let geo = tool.select_geo_and_normalize(object)?;

        // todo: generate lods (simplify mesh)
        // todo: optimize meshes (forsyth)

        tool.save_bf_mesh(geo)?;

        Ok(tool.stats)
    }

    /// Prints all available import options from the specified input file.
    pub fn print_possible_commands(params: Obj2BfParameters) -> Result<(), Obj2BfError> {
        let mut tool = Obj2Bf {
            params,
            stats: Statistics::default(),
        };

        let obj_set = tool.load()?;
        let mut option = 1;

        println!("Possible import options:\n");

        for obj in obj_set.objects {
            for (geo_idx, geo) in obj.geometry.iter().enumerate() {
                println!(
                    " {}. Object '{}', Geometry {} ({} verts, {} faces, material {})\n    Command: '{}'",
                    option,
                    obj.name,
                    geo_idx,
                    obj.vertices.len(),
                    geo.shapes.len(),
                    geo.material_name.as_deref().unwrap_or("None"),
                    format!(
                        "obj2bf.exe -i \"{}\" --object-name \"{}\" --geometry-index {}",
                        tool.params.input.to_str().unwrap(),
                        obj.name,
                        geo_idx
                    )
                );

                option += 1;
            }
        }

        for mat in obj_set.material_library.iter() {
            println!(
                " {}. Material '{}'\n    Command: '{}'",
                option,
                mat,
                format!(
                    "obj2bf.exe -i \"{}\" --material-name {}",
                    tool.params.input.to_str().unwrap(),
                    mat
                )
            );

            option += 1;
        }

        Ok(())
    }
}
