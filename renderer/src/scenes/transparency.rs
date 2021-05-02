use crate::assets::lookup;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::transform::Transform;
use crate::render::ubo::MaterialData;
use crate::render::vertex::NormalMappedVertex;
use crate::resources::material::{create_default_fallback_maps, StaticMaterial};
use crate::resources::mesh::create_mesh_dynamic;
use bf::material::BlendMode;
use cgmath::{point3, vec3};
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

    let (glass_mat1, f4) = StaticMaterial::from_material_data(
        BlendMode::Translucent,
        MaterialData {
            albedo_color: [0.0, 0.8, 0.0],
            alpha_cutoff: 0.0,
            roughness: 0.2,
            metallic: 0.0,
            opacity: 0.5,
            ior: 1.5,
        },
        path.buffers.geometry_pipeline.clone(),
        path.samplers.aniso_repeat.clone(),
        assets.transfer_queue.clone(),
        fallback_maps.clone(),
    )
    .ok()
    .unwrap();

    let (glass_mat2, f5) = StaticMaterial::from_material_data(
        BlendMode::Translucent,
        MaterialData {
            albedo_color: [0.8, 0.0, 0.0],
            alpha_cutoff: 0.0,
            roughness: 0.2,
            metallic: 0.0,
            opacity: 0.5,
            ior: 1.5,
        },
        path.buffers.geometry_pipeline.clone(),
        path.samplers.aniso_repeat.clone(),
        assets.transfer_queue.clone(),
        fallback_maps.clone(),
    )
    .ok()
    .unwrap();

    let (glass_mat3, f6) = StaticMaterial::from_material_data(
        BlendMode::Translucent,
        MaterialData {
            albedo_color: [0.0, 0.0, 0.0],
            alpha_cutoff: 0.0,
            roughness: 0.2,
            metallic: 0.0,
            opacity: 0.5,
            ior: 1.5,
        },
        path.buffers.geometry_pipeline.clone(),
        path.samplers.aniso_repeat.clone(),
        assets.transfer_queue.clone(),
        fallback_maps.clone(),
    )
    .ok()
    .unwrap();

    let glass = Object::new(
        mesh!("wineglass.obj"),
        glass_mat1,
        device.clone(),
        path.buffers.transparency.accumulation_pipeline.clone(),
        Transform {
            position: vec3(0.0, 5.35, 1.0),
            scale: vec3(0.15, 0.15, 0.15),
            ..Transform::default()
        },
    );

    let glass2 = Object::new(
        mesh!("LithuanianVodka.obj"),
        glass_mat2,
        device.clone(),
        path.buffers.transparency.accumulation_pipeline.clone(),
        Transform {
            position: vec3(0.0, 5.35, -1.0),
            scale: vec3(2.0, 2.0, 2.0),
            ..Transform::default()
        },
    );

    let glass_sphere: Object<NormalMappedVertex> = Object::new(
        mesh!("sphere.obj"),
        glass_mat3,
        device.clone(),
        path.buffers.transparency.accumulation_pipeline.clone(),
        Transform {
            position: vec3(0.0, 6.35, 0.0),
            scale: vec3(0.2, 0.2, 0.2),
            ..Transform::default()
        },
    );

    f1.join(f4).join(f5).join(f6).then_signal_fence().wait(None);

    state.camera.position = point3(0.0, 6.0, 4.0);
    state.camera.forward = vec3(1.0, 0.0, 0.0);
    state.objects = vec![plane, table, glass, glass2, glass_sphere];

    info!("data loaded after {}s!", start.elapsed().as_secs_f32());
}
