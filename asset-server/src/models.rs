//! Serializable application data objects / models.

use bf::image::Format;
use bf::material::BlendMode;
use bf::mesh::{IndexType, VertexFormat};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    pub uuid: Uuid,
    pub name: String,
    pub input_path: String,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub format: Format,
    pub pack_normal_map: Option<bool>,
    pub v_flip: Option<bool>,
    pub h_flip: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Mesh {
    pub uuid: Uuid,
    pub name: String,
    pub input_path: String,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub index_type: Option<IndexType>,
    pub vertex_format: Option<VertexFormat>,
    pub object_name: Option<String>,
    pub geometry_index: Option<usize>,
    pub lod: Option<u8>,
    pub recalculate_normals: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Material {
    pub uuid: Uuid,
    pub name: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
    pub blend_mode: Option<BlendMode>,
    pub albedo_color: Option<[f32; 3]>,
    pub roughness: Option<f32>,
    pub metallic: Option<f32>,
    pub alpha_cutoff: Option<f32>,
    pub albedo_map: Option<Uuid>,
    pub normal_map: Option<Uuid>,
    pub displacement_map: Option<Uuid>,
    pub roughness_map: Option<Uuid>,
    pub ao_map: Option<Uuid>,
    pub metallic_map: Option<Uuid>,
    pub opacity_map: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Asset {
    Image(Image),
    Mesh(Mesh),
    Material(Material),
}

impl Asset {
    #[inline]
    pub fn uuid(&self) -> Uuid {
        match self {
            Asset::Image(t) => t.uuid,
            Asset::Mesh(t) => t.uuid,
            Asset::Material(t) => t.uuid,
        }
    }

    #[inline]
    pub fn name(&self) -> &String {
        match self {
            Asset::Image(t) => &t.name,
            Asset::Mesh(t) => &t.name,
            Asset::Material(t) => &t.name,
        }
    }

    #[inline]
    pub fn tags(&self) -> &[String] {
        match self {
            Asset::Image(t) => t.tags.as_slice(),
            Asset::Mesh(t) => t.tags.as_slice(),
            Asset::Material(t) => t.tags.as_slice(),
        }
    }

    #[inline]
    pub fn updated_at(&self) -> DateTime<Utc> {
        match self {
            Asset::Image(t) => t.updated_at,
            Asset::Mesh(t) => t.updated_at,
            Asset::Material(t) => t.updated_at,
        }
    }

    #[inline]
    pub fn input_path(&self) -> Option<&String> {
        match self {
            Asset::Image(t) => Some(&t.input_path),
            Asset::Mesh(t) => Some(&t.input_path),
            Asset::Material(_) => None,
        }
    }

    #[inline]
    pub fn set_input_path<S: Into<String>>(&mut self, path: S) {
        match self {
            Asset::Image(t) => t.input_path = path.into(),
            Asset::Mesh(t) => t.input_path = path.into(),
            Asset::Material(_) => {}
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Compilation {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub cmd: String,
    pub error: Option<String>,
}
