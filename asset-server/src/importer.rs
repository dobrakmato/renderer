//! Functions that determine best initial import configuration for individual assets.

use crate::database::Database;
use crate::library::Library;
use crate::models::{Asset, Image, Material, Mesh};
use bf::image::Format;
use bf::material::BlendMode;
use chrono::Utc;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const ALBEDO_STRINGS: &[&str] = &[
    "_col.",
    "_color.",
    "diffuse.",
    "_albedo.",
    "_basecolor.",
    "-albedo",
];
const DISPLACEMENT_STRINGS: &[&str] = &["_disp.", "_displacement."];
const NORMAL_STRINGS: &[&str] = &["_nrm.", "_normal.", "_normalmap."];
const ROUGHNESS_STRINGS: &[&str] = &["_rgh.", "_roughness."];
const GLOSSINESS_STRINGS: &[&str] = &["[gloss].", "_gloss."];
const OCCLUSION_STRINGS: &[&str] = &["_ao.", "_ambientocclusion.", "_occlusion."];
const METALLIC_STRINGS: &[&str] = &["_met.", "_metallic.", "_metalness."];
const OPACITY_STRINGS: &[&str] = &["_opacity."];

#[derive(Debug)]
pub enum ImportError {
    NothingToImport,
    BadPath,
    ReadDirError,
    DependencyNotFound,
    UnsupportedExtension,
    MissingExtension,
    AlreadyTracked(Uuid),
}

pub struct Importer {
    library: Arc<Library>,
    database: Arc<Database>,
}

impl Importer {
    pub fn import_file(&self, disk_path: &Path) -> Result<Uuid, ImportError> {
        let uuid = self.library.determine_uuid_by_path(disk_path);

        if self.database.has_asset(&uuid) {
            return Err(ImportError::AlreadyTracked(uuid));
        }

        let asset = match disk_path
            .extension()
            .and_then(OsStr::to_str)
            .map(str::to_lowercase)
        {
            Some(t) => match t.as_str() {
                "jpg" | "png" | "tiff" | "tif" | "tga" => self.try_import_image(uuid, disk_path)?,
                "obj" => self.try_import_mesh(uuid, disk_path)?,
                _ => return Err(ImportError::UnsupportedExtension),
            },
            None => self.try_import_material(uuid, disk_path)?,
        };

        self.database.insert_asset(asset);

        Ok(uuid)
    }

    pub fn find_dependency_uuid(&self, disk_path: &Path) -> Result<Uuid, ImportError> {
        let uuid = self.library.determine_uuid_by_path(disk_path);
        if self.database.has_asset(&uuid) {
            Ok(uuid)
        } else {
            Err(ImportError::DependencyNotFound)
        }
    }

    pub fn try_import_material(&self, uuid: Uuid, disk_path: &Path) -> Result<Asset, ImportError> {
        if !disk_path.is_dir() {
            return Err(ImportError::MissingExtension);
        }

        let input_path = self
            .library
            .disk_path_to_db_path(disk_path)
            .trim_end_matches(|x| x == '/' || x == '\\');

        let mut name = input_path.to_string();
        name.push_str(".mat");

        let mut is_material = false;
        let mut asset = Material {
            uuid,
            name,
            tags: vec!["material".to_string()],
            updated_at: Utc::now(),
            blend_mode: Option::None,
            albedo_color: Option::None,
            roughness: Option::None,
            metallic: Option::None,
            alpha_cutoff: Option::None,
            albedo_map: Option::None,
            normal_map: Option::None,
            displacement_map: Option::None,
            roughness_map: Option::None,
            ao_map: Option::None,
            metallic_map: Option::None,
            opacity_map: Option::None,
            opacity: Option::None,
            ior: Option::None,
            sss: Option::None,
        };

        for x in std::fs::read_dir(disk_path).map_err(|_| ImportError::ReadDirError)? {
            let x = x.unwrap().path();
            let file_name = x
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_lowercase()
                .replace("-", "_");

            if ALBEDO_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.albedo_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if DISPLACEMENT_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.displacement_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if NORMAL_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.normal_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if ROUGHNESS_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.roughness_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if GLOSSINESS_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.roughness_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if OCCLUSION_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.ao_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if METALLIC_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.metallic_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            } else if OPACITY_STRINGS.iter().any(|x| file_name.contains(x)) {
                asset.opacity_map = Some(self.find_dependency_uuid(&x)?);
                is_material = true;
            }
        }

        if asset.opacity_map.is_some() {
            asset.blend_mode = Some(BlendMode::Masked);
        }

        if is_material {
            Ok(Asset::Material(asset))
        } else {
            Err(ImportError::NothingToImport)
        }
    }

    pub fn try_import_mesh(&self, uuid: Uuid, disk_path: &Path) -> Result<Asset, ImportError> {
        let input_path = self.library.disk_path_to_db_path(disk_path).to_string();

        Ok(Asset::Mesh(Mesh {
            uuid,
            name: input_path.clone(),
            input_path,
            tags: vec!["mesh".to_string()],
            updated_at: Utc::now(),
            index_type: Option::None,
            vertex_format: Option::None,
            object_name: Option::None,
            geometry_index: Option::None,
            lod: Option::None,
            recalculate_normals: Option::None,
        }))
    }

    pub fn try_import_image(&self, uuid: Uuid, disk_path: &Path) -> Result<Asset, ImportError> {
        let input_path = self.library.disk_path_to_db_path(disk_path).to_string();

        let file_name = match disk_path
            .file_name()
            .map(OsStr::to_str)
            .flatten()
            .map(str::to_lowercase)
        {
            Some(t) => t,
            None => return Err(ImportError::BadPath),
        };
        let mut tags = vec!["texture".to_string()];
        let mut format = Format::Rgba8;
        let mut pack_normal_map = false;

        // determine correct format
        if ALBEDO_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::SrgbDxt1;
        } else if DISPLACEMENT_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        } else if NORMAL_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::Dxt5;
            pack_normal_map = true;
            tags.push("normal-map".to_string());
        } else if ROUGHNESS_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        } else if GLOSSINESS_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        } else if OCCLUSION_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        } else if METALLIC_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        } else if OPACITY_STRINGS.iter().any(|x| file_name.contains(x)) {
            format = Format::R8;
        }

        Ok(Asset::Image(Image {
            uuid,
            name: input_path.clone(),
            input_path,
            tags,
            updated_at: Utc::now(),
            format,
            pack_normal_map: Some(pack_normal_map),
            v_flip: Option::None,
            h_flip: Option::None,
        }))
    }
}

pub fn create_importer(database: Arc<Database>, library: Arc<Library>) -> Arc<Importer> {
    Arc::new(Importer { library, database })
}
