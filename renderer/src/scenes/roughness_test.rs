use crate::assets::lookup;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::transform::Transform;
use crate::render::ubo::MaterialData;
use crate::resources::material::{create_default_fallback_maps, StaticMaterial};
use crate::resources::mesh::create_mesh_dynamic;
use bf::material::BlendMode;
use cgmath::vec3;
use log::info;
use std::time::Instant;
use vulkano::sync::GpuFuture;

pub fn create(engine: &mut Engine) {
    let device = &engine.vulkan_state.device();
    let assets = &engine.content;
    let path = &mut engine.renderer_state.render_path;

    let (fallback_maps, f1) = create_default_fallback_maps(engine.vulkan_state.transfer_queue());

    macro_rules! mesh {
        ($name: expr) => {{
            let guard = assets.request_load(lookup($name));

            let mesh = guard.wait::<bf::mesh::Mesh>();

            let (mesh, f) = create_mesh_dynamic(&mesh, assets.transfer_queue.clone())
                .expect("cannot create mesh");
            f.then_signal_fence_and_flush().ok();

            mesh
        }};
    }

    let start = Instant::now();
    info!("Loading scene assets...");

    let plane_mesh = mesh!("plane.obj");
    let sphere_mesh = mesh!("sphere.obj");

    let state = &mut engine.game_state;

    let (floor_mat, f2) = StaticMaterial::from_material_data(
        BlendMode::Opaque,
        MaterialData {
            albedo_color: [1.0; 3],
            alpha_cutoff: 0.0,
            roughness: 0.5,
            metallic: 0.0,
            opacity: 1.0,
            ior: 1.0,
        },
        path.buffers.geometry_pipeline.clone(),
        path.samplers.aniso_repeat.clone(),
        assets.transfer_queue.clone(),
        fallback_maps.clone(),
    )
    .expect("Cannot create material");

    f1.join(f2).then_signal_fence().wait(None);

    let plane = Object::new(
        plane_mesh,
        floor_mat,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(50.0, 1.0, 50.0),
            ..Transform::default()
        },
    );

    state.objects = vec![plane];

    let steps = 10;

    for r in 0..steps {
        for m in 0..steps {
            let roughness = (r as f32) / (steps as f32) + 0.01;
            let metallic = (m as f32) / (steps as f32) + 0.01;

            let (sphere_mat, f) = StaticMaterial::from_material_data(
                BlendMode::Opaque,
                MaterialData {
                    albedo_color: [0.8, 0.4, 0.3],
                    alpha_cutoff: 0.0,
                    roughness,
                    metallic,
                    opacity: 1.0,
                    ior: 1.0,
                },
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                assets.transfer_queue.clone(),
                fallback_maps.clone(),
            )
            .ok()
            .unwrap();

            f.then_signal_fence().wait(None);

            let sphere = Object::new(
                sphere_mesh.clone(),
                sphere_mat,
                device.clone(),
                path.buffers.geometry_pipeline.clone(),
                Transform {
                    position: vec3(0.0, 3.0 + m as f32, 0.0 + r as f32),
                    scale: vec3(0.5, 0.5, 0.5),
                    ..Transform::default()
                },
            );

            state.objects.push(sphere);
        }
    }

    info!("data loaded after {}s!", start.elapsed().as_secs_f32());
}
