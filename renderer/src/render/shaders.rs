pub mod vs_deferred_geometry {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vs_deferred_geometry.glsl"
    }
}

pub mod fs_deferred_geometry {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_deferred_geometry.glsl"
    }
}

pub mod fs_deferred_lighting {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_deferred_lighting.glsl"
    }
}

pub mod vs_passtrough {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vs_passtrough.glsl"
    }
}

pub mod fs_tonemap {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_tonemap.glsl"
    }
}
