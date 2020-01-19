pub mod basic_vert {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/basic_vert.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/basic_vert.glsl"
    }
}

pub mod basic_frag {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/basic_frag.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/basic_frag.glsl"
    }
}

pub mod sky_vert {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/sky_hosek_vert.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/sky_hosek_vert.glsl"
    }
}

pub mod sky_frag {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/sky_hosek_frag.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/sky_hosek_frag.glsl"
    }
}
