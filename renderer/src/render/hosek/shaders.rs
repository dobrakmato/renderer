//! Shaders for Hosek-Wilkie sky model.

use once_cell::sync::OnceCell;
use std::sync::Arc;
use vulkano::device::Device;

pub mod vertex {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../../../shaders/sky_hosek_vert.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/sky_hosek_vert.glsl"
    }
}

pub mod fragment {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../../../shaders/sky_hosek_frag.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/sky_hosek_frag.glsl"
    }
}

/// Runtime cell for static vertex shader.
static VERTEX_SHADER: OnceCell<Arc<vertex::Shader>> = OnceCell::new();

/// Runtime cell for static fragment shader.
static FRAGMENT_SHADER: OnceCell<Arc<fragment::Shader>> = OnceCell::new();

pub fn get_or_load_vertex_shader(device: Arc<Device>) -> Arc<vertex::Shader> {
    VERTEX_SHADER
        .get_or_init(|| Arc::new(vertex::Shader::load(device.clone()).expect("cannot load shader")))
        .clone()
}

pub fn get_or_load_fragment_shader(device: Arc<Device>) -> Arc<fragment::Shader> {
    FRAGMENT_SHADER
        .get_or_init(|| {
            Arc::new(fragment::Shader::load(device.clone()).expect("cannot load shader"))
        })
        .clone()
}
