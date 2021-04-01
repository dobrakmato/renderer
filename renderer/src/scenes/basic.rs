use crate::assets::lookup;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::transform::Transform;
use crate::render::vertex::NormalMappedVertex;
use crate::resources::material::{create_default_fallback_maps, StaticMaterial};
use crate::resources::mesh::create_mesh_dynamic;
use crate::GameState;
use cgmath::{vec3, Deg, Quaternion, Rotation3, Vector3};
use log::info;
use std::sync::Arc;
use std::time::Instant;

pub fn create(engine: &mut Engine) {
    let start = Instant::now();
    let device = &engine.vulkan_state.device();
    let assets = &engine.asset_storage;
    let path = &mut engine.renderer_state.render_path;

    let (fallback_maps, _) = create_default_fallback_maps(engine.vulkan_state.transfer_queue());

    let static_material = |mat: Arc<bf::material::Material>| {
        StaticMaterial::from_material(
            mat.as_ref(),
            &assets,
            path.buffers.geometry_pipeline.clone(),
            path.samplers.aniso_repeat.clone(),
            assets.transfer_queue.clone(),
            fallback_maps.clone(),
        )
        .unwrap()
    };

    let static_mesh = |mesh: Arc<bf::mesh::Mesh>| {
        create_mesh_dynamic::<NormalMappedVertex>(&mesh, assets.transfer_queue.clone())
            .expect("cannot create mesh from bf::mesh::Mesh")
            .0
    };

    let sneakers = Object::new(
        static_mesh(
            assets
                .request_load(lookup("pbr_sneaker\\PB170_Sneaker_Sm.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("pbr_sneaker.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(0.1, 0.1, 0.1),
            position: vec3(3.0, 5.0, 3.0),
            rotation: Quaternion::from_angle_x(Deg(-90.0)),
        },
    );

    let cabinet = Object::new(
        static_mesh(
            assets
                .request_load(lookup("pbr_cabinet\\cabinet.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("pbr_cabinet.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(0.05, 0.05, 0.05),
            position: vec3(3.0, 5.0, 9.0),
            rotation: Quaternion::from_angle_y(Deg(-45.0)),
        },
    );

    let welding_setup = Object::new(
        static_mesh(
            assets
                .request_load(lookup("pbr_welding_setup\\WeldingSetup_obj.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("pbr_welding_setup.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(0.01, 0.01, 0.01),
            position: vec3(-3.0, 0.1, -3.0),
            ..Transform::default()
        },
    );

    let cottage = Object::new(
        static_mesh(
            assets
                .request_load(lookup("pbr_cottage\\Cottage_FREE.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("pbr_cottage.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(0.0, 0.0, -15.0),
            ..Transform::default()
        },
    );

    let red_barn = Object::new(
        static_mesh(
            assets
                .request_load(lookup("pbr_red_barn\\Rbarn15.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("pbr_red_barn.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(0.0, 0.1, 30.0),
            ..Transform::default()
        },
    );

    let apple = Object::new(
        static_mesh(
            assets
                .request_load(lookup("3DApple002_2K-JPG\\3DApple002_2K.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("3DApple002_2K-JPG.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(6.0, 6.0, 6.0),
            position: vec3(0.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let woman = Object::new(
        static_mesh(
            assets
                .request_load(lookup(
                    "autumn_casualwoman_01\\autumn_casualwoman_01_lowpoly_3dsmax.obj",
                ))
                .wait(),
        ),
        static_material(
            assets
                .request_load(lookup("autumn_casualwoman_01.mat"))
                .wait(),
        )
        .0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(0.1, 0.1, 0.1),
            position: vec3(7.0, 0.0, 0.0),
            ..Transform::default()
        },
    );

    let bread1 = Object::new(
        static_mesh(
            assets
                .request_load(lookup("3DBread001_LowPoly\\3DBread001_LowPoly.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("3DBread001_LowPoly.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(5.0, 5.0, 5.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock1 = Object::new(
        static_mesh(
            assets
                .request_load(lookup("3DRock001_2K\\3DRock001_2K.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("3DRock001_2K.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock2 = Object::new(
        static_mesh(
            assets
                .request_load(lookup("3DRock002_9K\\3DRock002_9K.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("3DRock002_9K.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(2.0, 2.0, 2.0),
            position: vec3(-3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let jess = Object::new(
        static_mesh(
            assets
                .request_load(lookup(
                    "Jess_Casual_Walking_001\\Jess_Casual_Walking_001.obj",
                ))
                .wait(),
        ),
        static_material(
            assets
                .request_load(lookup("Jess_Casual_Walking_001.mat"))
                .wait(),
        )
        .0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(0.001, 0.001, 0.001),
            position: vec3(-1.65, 0.5, -9.72),
            rotation: Quaternion::from_angle_x(Deg(-90.0)),
        },
    );

    let fern = Object::new(
        static_mesh(
            assets
                .request_load(lookup("Soi_Foliage_OBJ\\SM_Fern_01.obj"))
                .wait(),
        ),
        static_material(
            assets
                .request_load(lookup("Soi_Foliage_OBJ\\T_Ferns.mat"))
                .wait(),
        )
        .0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(0.0, 0.0, -9.5),
            ..Transform::default()
        },
    );

    let test_cube = Object::new(
        static_mesh(
            assets
                .request_load(lookup("test_cube\\test_cube_default.obj"))
                .wait(),
        ),
        static_material(assets.request_load(lookup("test_cube.mat")).wait()).0,
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(-5.0, 0.5, -5.0),
            ..Transform::default()
        },
    );

    let mat_start = Instant::now();
    let materials = [
        "1k_floor.mat",
        "copper-rock1.mat",
        "sandstonecliff-ue.mat",
        "Moss001_2K-JPG.mat",
        "PavingStones066_2K-JPG.mat",
        "PavingStones084_2K-JPG.mat",
        "sand1-ue.mat",
        "Fabric008_2K-JPG.mat",
        "Ground033_2K-JPG.mat",
        "Ground035_2K-JPG.mat",
        "Leather012_2K-JPG.mat",
        "Leather016_2K-JPG.mat",
        "Metal006_2K-JPG.mat",
        "Metal012_2K-JPG.mat",
        "MetalPlates004_2K-JPG.mat",
        "MetalPlates006_2K-JPG.mat",
        "[2K]Bricks22.mat",
        "[2K]Concrete07.mat",
        "[2K]Ground27.mat",
        "[2K]Ground30.mat",
        "[2K]Ground37.mat",
        "[2K]Leather11.mat",
        "[2K]Marble04.mat",
        "[2K]Marble06.mat",
        "[2K]Metal07.mat",
        "[2K]Metal08.mat",
        "[2K]Metal27.mat",
        "[2K]Metal28.mat",
        "[2K]PaintedPlaster05.mat",
        "[2K]PavingStones42.mat",
        "[2K]PavingStones53.mat",
        "[2K]Planks12.mat",
        "[2K]SolarPanel03.mat",
        "[2K]Tiles15.mat",
        "[2K]Tiles44.mat",
        "[2K]Tiles52.mat",
        "[2K]Wood18.mat",
        "[2K]Wood35.mat",
        "[2K]WoodFloor12.mat",
        "[2K]WoodFloor32.mat",
        "Bricks027_2K-JPG.mat",
        "Bricks037_2K-JPG.mat",
        "Carpet013_2K-JPG.mat",
        "CorrugatedSteel005_2K-JPG.mat",
        "Fabric031_2K-JPG.mat",
        "Fabric032_2K-JPG.mat",
        "Ground036_2K-JPG.mat",
        "Ice004_2K-JPG.mat",
        "Leather021_2K-JPG.mat",
        "Metal017_2K-JPG.mat",
        "Paint002_2K-JPG.mat",
        "PaintedWood005_2K-JPG.mat",
        "PavingStones055_2K-JPG.mat",
        "Road006_2K-JPG.mat",
        "Rock020_2K-JPG.mat",
        "Rocks017_2K-JPG.mat",
        "Terrazzo003_2K-JPG.mat",
        "Tiles059_2K-JPG.mat",
        "Tiles072_2K-JPG.mat",
        "WoodSiding007_2K-JPG.mat",
    ]
    .iter()
    .map(|x| lookup(x))
    .map(|x| assets.request_load::<bf::material::Material>(x).wait())
    .map(|x| static_material(x).0)
    .collect();
    println!(
        "Material load took {} seconds!",
        mat_start.elapsed().as_secs_f32()
    );
    let plane_mesh = static_mesh(assets.request_load(lookup("plane.obj")).wait());

    // setup sky
    path.sky.sun_dir = engine.game_state.directional_lights[0].direction;
    path.sky.turbidity = 8.0;
    path.sky.ground_albedo = Vector3::new(1.0, 0.0, 0.0);

    let state = &mut engine.game_state;

    state.materials = materials;

    let plane = Object::new(
        plane_mesh,
        state.materials.get(0).unwrap().clone(),
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(50.0, 1.0, 50.0),
            ..Transform::default()
        },
    );
    info!("data loaded after {}s!", start.elapsed().as_secs_f32());

    state.objects = vec![
        plane,
        fern,
        test_cube,
        apple,
        bread1,
        rock1,
        rock2,
        woman,
        jess,
        cottage,
        welding_setup,
        sneakers,
        red_barn,
        cabinet,
    ];
}
