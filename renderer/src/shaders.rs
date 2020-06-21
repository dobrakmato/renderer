pub mod vs_deferred_geometry {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/vs_deferred_geometry.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vs_deferred_geometry.glsl"
    }
}

pub mod fs_deferred_geometry {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/fs_deferred_geometry.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_deferred_geometry.glsl"
    }
}

pub mod fs_deferred_lighting {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/fs_deferred_lighting.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_deferred_lighting.glsl"
    }
}

pub mod vs_passtrough {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/vs_passtrough.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vs_passtrough.glsl"
    }
}

pub mod fs_tonemap {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/fs_tonemap.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fs_tonemap.glsl"
    }
}
