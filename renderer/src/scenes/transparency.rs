use crate::assets::lookup;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::transform::Transform;
use crate::render::ubo::MaterialData;
use crate::resources::material::{create_default_fallback_maps, StaticMaterial};
use crate::resources::mesh::create_mesh_dynamic;
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

    macro_rules! material {
        ($name: expr) => {{
            let material = {
                let guard = assets.request_load(lookup($name));
                let mat = guard.wait();
                *mat
            };

            let (material, f) = StaticMaterial::from_material(
                &material,
                &assets,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                assets.transfer_queue.clone(),
                fallback_maps.clone(),
            )
            .expect("cannot create material");
            f.then_signal_fence_and_flush().ok();

            material
        }};
    }

    let start = Instant::now();
    info!("Loading scene assets...");

    let plane_mesh = mesh!("plane.obj");
    let table_mesh = mesh!("TableType_A.obj");
    let glass_mesh = mesh!("wineglass.obj");

    let state = &mut engine.game_state;

    let plane = Object::new(
        plane_mesh,
        material!("1k_floor.mat"),
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(50.0, 1.0, 50.0),
            ..Transform::default()
        },
    );

    let table = Object::new(
        table_mesh.clone(),
        material!("TableType_A.mat"),
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            position: vec3(0.0, 0.0, 0.0),
            scale: vec3(0.06, 0.06, 0.06),
            ..Transform::default()
        },
    );

    let (glass_mat, f4) = StaticMaterial::from_material_data(
        MaterialData {
            albedo_color: [0.4; 3],
            alpha_cutoff: 0.0,
            roughness: 0.5,
            metallic: 0.0,
        },
        path.buffers.geometry_pipeline.clone(),
        path.samplers.aniso_repeat.clone(),
        assets.transfer_queue.clone(),
        fallback_maps.clone(),
    )
    .ok()
    .unwrap();

    let glass = Object::new(
        glass_mesh.clone(),
        glass_mat.clone(),
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            position: vec3(0.0, 5.35, 1.0),
            scale: vec3(0.15, 0.15, 0.15),
            ..Transform::default()
        },
    );

    let glass2 = Object::new(
        glass_mesh.clone(),
        glass_mat,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            position: vec3(0.0, 5.35, -1.0),
            scale: vec3(0.15, 0.15, 0.15),
            ..Transform::default()
        },
    );

    f1.join(f4).then_signal_fence().wait(None);

    state.objects = vec![plane, table, glass, glass2];

    info!("data loaded after {}s!", start.elapsed().as_secs_f32());
}
