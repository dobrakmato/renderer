use once_cell::sync::OnceCell;
use std::sync::Arc;
use vulkano::device::Device;

pub mod accumulation_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vs_mcguire13_accumulation.glsl"
    }
}

pub mod accumulation_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_mcguire13_accumulation.glsl"
    }
}

pub mod resolve_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_mcguire13_resolve.glsl"
    }
}

/// Runtime cell for static vertex shader.
static ACCUMULATION_VERTEX_SHADER: OnceCell<Arc<accumulation_vs::Shader>> = OnceCell::new();

/// Runtime cell for static fragment shader.
static ACCUMULATION_FRAGMENT_SHADER: OnceCell<Arc<accumulation_fs::Shader>> = OnceCell::new();

/// Runtime cell for static fragment shader.
static RESOLVE_FRAGMENT_SHADER: OnceCell<Arc<resolve_fs::Shader>> = OnceCell::new();

pub fn get_or_load_acc_vertex_shader(device: Arc<Device>) -> Arc<accumulation_vs::Shader> {
    ACCUMULATION_VERTEX_SHADER
        .get_or_init(|| {
            Arc::new(accumulation_vs::Shader::load(device.clone()).expect("cannot load shader"))
        })
        .clone()
}

pub fn get_or_load_acc_fragment_shader(device: Arc<Device>) -> Arc<accumulation_fs::Shader> {
    ACCUMULATION_FRAGMENT_SHADER
        .get_or_init(|| {
            Arc::new(accumulation_fs::Shader::load(device.clone()).expect("cannot load shader"))
        })
        .clone()
}

pub fn get_or_load_resolve_fragment_shader(device: Arc<Device>) -> Arc<resolve_fs::Shader> {
    RESOLVE_FRAGMENT_SHADER
        .get_or_init(|| {
            Arc::new(resolve_fs::Shader::load(device.clone()).expect("cannot load shader"))
        })
        .clone()
}
