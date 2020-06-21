use crate::assets::lookup;
use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::transform::Transform;
use crate::render::ubo::DirectionalLight;
use crate::render::vertex::NormalMappedVertex;
use crate::resources::material::{create_default_fallback_maps, StaticMaterial};
use crate::resources::mesh::create_mesh_dynamic;
use bf::uuid::Uuid;
use cgmath::{vec3, Deg, InnerSpace, Point3, Vector3};
use log::{info, Level};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::EventLoop;

#[cfg(debug_assertions)]
use log::warn;

mod assets;
mod camera;
mod engine;
mod input;
mod render;
mod resources;
mod samplers;
mod shaders;

#[derive(Copy, Clone)]
pub struct RendererConfiguration {
    pub fullscreen: bool,
    pub resolution: [u16; 2],
    pub gpu: usize,
}

impl Into<Size> for RendererConfiguration {
    fn into(self) -> Size {
        Size::Logical(LogicalSize::new(
            self.resolution[0] as f64,
            self.resolution[1] as f64,
        ))
    }
}

pub struct GameState {
    start: Instant,
    camera: PerspectiveCamera,
    objects: Vec<Object<NormalMappedVertex>>,
    directional_lights: Vec<DirectionalLight>,
    materials: Vec<Arc<StaticMaterial>>,
    floor_mat: usize,
}

fn main() {
    // initialize logging at start of the application
    simple_logger::init_with_level(Level::Debug).unwrap();

    #[cfg(debug_assertions)]
    warn!("this is a debug build. performance may hurt.");

    // load configuration from a file
    let conf = RendererConfiguration {
        fullscreen: false,
        resolution: [1600, 900],
        gpu: 0,
    };
    let event_loop = EventLoop::new();
    let mut engine = Engine::new(
        GameState {
            start: Instant::now(),
            camera: PerspectiveCamera {
                position: Point3::new(0.0, 3.0, 0.0),
                forward: vec3(1.0, 0.0, 0.0),
                up: vec3(0.0, -1.0, 0.0),
                fov: Deg(90.0).into(),
                aspect_ratio: conf.resolution[0] as f32 / conf.resolution[1] as f32,
                near: 0.05,
                far: 100.0,
            },
            objects: vec![],
            directional_lights: vec![DirectionalLight {
                direction: vec3(3.0, 1.0, 1.0).normalize(),
                intensity: 2.5,
                color: vec3(1.0, 1.0, 0.8),
            }],
            materials: vec![],
            floor_mat: 0,
        },
        conf,
        event_loop,
    );
    load(&mut engine);
    // run engine
    engine.run_forever();
}

fn load(engine: &mut Engine) {
    info!("loading geometry and image data...");
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

    let apple = Object::new(
        static_mesh(
            assets
                .request_load(lookup(".\\3DApple002_2K-JPG/3DApple002_2K.obj"))
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
                    ".\\autumn_casualwoman_01/autumn_casualwoman_01_lowpoly_3dsmax.obj",
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
            position: vec3(7.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let bread1 = Object::new(
        static_mesh(
            assets
                .request_load(Uuid::from_str("6f88a288-6ce9-5455-9bd8-3546c5b39467").unwrap())
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
                .request_load(Uuid::from_str("3f9e7780-6d4e-5108-9d36-23fc77339efb").unwrap())
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
                .request_load(Uuid::from_str("1a55dc06-6577-5cb5-9184-7a3b8d1e0c5a").unwrap())
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

    let mat_start = Instant::now();
    let materials = [
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
    let plane_mesh = static_mesh(assets.request_load(lookup("./plane.obj")).wait());

    // setup sky
    path.sky.sun_dir = engine.game_state.directional_lights[0].direction;
    path.sky.turbidity = 8.0;
    path.sky.ground_albedo = Vector3::new(0.0, 0.0, 0.0);

    let state = &mut engine.game_state;

    state.materials = materials;

    let plane = Object::new(
        plane_mesh,
        state.materials.get(0).unwrap().clone(),
        device.clone(),
        path.buffers.geometry_pipeline.clone(),
        Transform {
            scale: vec3(10.0, 1.0, 10.0),
            ..Transform::default()
        },
    );
    info!("data loaded after {}s!", start.elapsed().as_secs_f32());

    state.objects = vec![plane, apple, bread1, rock1, rock2, woman];
}
