use crate::library::Library;
use crate::models::{Asset, Image, Material, Mesh};
use bf::image::Format;
use bf::material::BlendMode;
use bf::mesh::{IndexType, VertexFormat};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt::Formatter;

/// Command for launching image importer (`img2bf`) tool.
pub const IMG2BF: &str = "img2bf.exe";
/// Command for launching mesh importer (`obj2bf`) tool.
pub const OBJ2BF: &str = "obj2bf.exe";
/// Command for launching material compiler (`matcomp`) tool.
pub const MATCOMP: &str = "matcomp.exe";
/// Command for launching information extractor (`bfinfo`) tool.
pub const BFINFO: &str = "bfinfo.exe";

/// Custom command struct that is serializable.
#[derive(Serialize, Deserialize)]
pub struct Command {
    program: String,
    args: Vec<String>,
}

impl Command {
    pub fn new<S: Into<String>>(program_name: S) -> Self {
        Self {
            program: program_name.into(),
            args: vec![],
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args
            .push(arg.as_ref().to_str().map(str::to_string).unwrap());
        self
    }
}

impl Into<tokio::process::Command> for Command {
    fn into(self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(self.program);

        for x in self.args {
            cmd.arg(x);
        }

        cmd
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.program.as_str())?;
        f.write_str(" ")?;

        for (idx, val) in self.args.iter().enumerate() {
            f.write_str(val.as_str())?;
            if idx != self.args.len() - 1 {
                f.write_str(" ")?;
            }
        }

        Ok(())
    }
}

/// Generates an cmd line flag argument based on value
/// of Option<bool>. We currently use this macro to reduce
/// boilerplate code.
///
/// Will be replaced when `if let guard` is ready.
/// TODO: https://github.com/rust-lang/rust/issues/51114
macro_rules! cmd_flag {
    ($cmd: expr, $arg: expr, $field: expr) => {
        if let Some(t) = $field {
            if t {
                $cmd.arg($arg);
            }
        }
    };
}

macro_rules! cmd_optional_arg {
    ($cmd: expr, $arg: expr, $field: expr) => {
        if let Some(ref t) = $field {
            $cmd.arg($arg).arg(format!("{}", t));
        }
    };
}

/// Objects with this trait can be compiled by running
/// the command generated by the `compile_command` function.
pub trait CompileCommand {
    /// Generates a command that can be used to compile this asset.
    fn compile_command(&self, library: &Library) -> Command;
}

// implementations for individual asset types

impl CompileCommand for Image {
    fn compile_command(&self, library: &Library) -> Command {
        let mut cmd = Command::new(IMG2BF);

        cmd.arg("--input")
            .arg(library.db_path_to_disk_path(&self.input_path));
        cmd.arg("--output")
            .arg(library.compute_output_path(&self.uuid));

        cmd.arg("--format");
        match self.format {
            Format::Dxt1 => cmd.arg("dxt1"),
            Format::Dxt3 => cmd.arg("dxt3"),
            Format::Dxt5 => cmd.arg("dxt5"),
            Format::Rgb8 => cmd.arg("rgb"),
            Format::Rgba8 => cmd.arg("rgba"),
            Format::SrgbDxt1 => cmd.arg("srgb_dxt1"),
            Format::SrgbDxt3 => cmd.arg("srgb_dxt3"),
            Format::SrgbDxt5 => cmd.arg("srgb_dxt5"),
            Format::Srgb8 => cmd.arg("srgb"),
            Format::Srgb8A8 => cmd.arg("dxt1"),
            Format::R8 => cmd.arg("r8"),
            Format::BC6H => cmd.arg("bc6h"),
            Format::BC7 => cmd.arg("bc7"),
            Format::SrgbBC7 => cmd.arg("srgb_bc7"),
        };

        cmd_flag!(cmd, "--pack-normal-map", self.pack_normal_map);
        cmd_flag!(cmd, "--v-flip", self.v_flip);
        cmd_flag!(cmd, "--h-flip", self.h_flip);

        cmd
    }
}

impl CompileCommand for Mesh {
    fn compile_command(&self, library: &Library) -> Command {
        let mut cmd = Command::new(OBJ2BF);

        cmd.arg("--input")
            .arg(library.db_path_to_disk_path(&self.input_path));
        cmd.arg("--output")
            .arg(library.compute_output_path(&self.uuid));

        if let Some(t) = self.index_type {
            cmd.arg("--index-type");
            match t {
                IndexType::U16 => cmd.arg("u16"),
                IndexType::U32 => cmd.arg("u32"),
            };
        }

        if let Some(t) = self.vertex_format {
            cmd.arg("--vertex-format");
            match t {
                VertexFormat::PositionNormalUvTangent => cmd.arg("pnut"),
                VertexFormat::PositionNormalUv => cmd.arg("pnu"),
                VertexFormat::Position => cmd.arg("p"),
            };
        }

        cmd_optional_arg!(cmd, "--object-name", self.object_name);
        cmd_optional_arg!(cmd, "--geometry-index", self.geometry_index);
        cmd_optional_arg!(cmd, "--lod", self.lod);
        cmd_flag!(cmd, "--recalculate-normals", self.recalculate_normals);

        cmd
    }
}

impl CompileCommand for Material {
    fn compile_command(&self, library: &Library) -> Command {
        let mut cmd = Command::new(MATCOMP);

        cmd.arg("--output")
            .arg(library.compute_output_path(&self.uuid));

        if let Some(t) = self.blend_mode {
            cmd.arg("--blend-mode");
            match t {
                BlendMode::Opaque => cmd.arg("opaque"),
                BlendMode::Masked => cmd.arg("masked"),
                BlendMode::Translucent => cmd.arg("translucent"),
            };
        }

        if let Some(t) = self.albedo_color {
            cmd.arg("--albedo-color")
                .arg(format!("{},{},{}", t[0], t[1], t[2]));
        }

        cmd_optional_arg!(cmd, "--roughness", self.roughness);
        cmd_optional_arg!(cmd, "--metallic", self.metallic);
        cmd_optional_arg!(cmd, "--alpha-cutoff", self.alpha_cutoff);
        cmd_optional_arg!(cmd, "--ior", self.ior);
        cmd_optional_arg!(cmd, "--opacity", self.opacity);

        cmd_optional_arg!(cmd, "--albedo-map", self.albedo_map);
        cmd_optional_arg!(cmd, "--normal-map", self.normal_map);
        cmd_optional_arg!(cmd, "--displacement-map", self.displacement_map);
        cmd_optional_arg!(cmd, "--roughness-map", self.roughness_map);
        cmd_optional_arg!(cmd, "--opacity-map", self.opacity_map);
        cmd_optional_arg!(cmd, "--ao-map", self.ao_map);
        cmd_optional_arg!(cmd, "--metallic-map", self.metallic_map);

        cmd
    }
}

// delegating impl for Asset type
impl CompileCommand for Asset {
    fn compile_command(&self, library: &Library) -> Command {
        match self {
            Asset::Image(t) => t.compile_command(library),
            Asset::Mesh(t) => t.compile_command(library),
            Asset::Material(t) => t.compile_command(library),
        }
    }
}
