//! Transform struct that is used to represent *position*, *rotation* and *scale* of objects.

use crate::render::ubo::ObjectMatrixData;
use cgmath::{Matrix4, Quaternion, Vector3};

/// Transform is a struct that is used to represent *position*, *rotation* and *scale*
/// of an object in *world space*.
#[derive(Copy, Clone)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Into<Matrix4<f32>> for Transform {
    fn into(self) -> Matrix4<f32> {
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let rotation = Matrix4::from(self.rotation);
        let translate = Matrix4::from_translation(self.position);

        translate * scale * rotation
    }
}

impl Into<ObjectMatrixData> for Transform {
    fn into(self) -> ObjectMatrixData {
        ObjectMatrixData { model: self.into() }
    }
}
