use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use crate::lookup::lookup;
use crate::material::{FallbackMaps, StaticMaterial};
use crate::pod::DirectionalLight;
use crate::render::{BasicVertex, Object, Transform};
use cgmath::{vec3, Deg, InnerSpace, Point3};
use log::info;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::EventLoop;

#[cfg(debug_assertions)]
use log::warn;

mod camera;
#[macro_use]
mod content;
mod engine;
mod hosek;
mod image;
mod input;
mod io;
mod lookup;
mod material;
mod mesh;
mod pod;
mod render;
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
    objects_u16: Vec<Object<BasicVertex, u16>>,
    objects_u32: Vec<Object<BasicVertex, u32>>,
    directional_lights: Vec<DirectionalLight>,
    materials: Vec<Arc<StaticMaterial>>,
    floor_mat: usize,
}

fn main() {
    // initialize logging at start of the application
    simple_logger::init().unwrap();

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
            objects_u16: vec![],
            objects_u32: vec![],
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
    let content = &engine.content;
    let path = &engine.renderer_state.render_path;

    let fallback_maps = Arc::new(FallbackMaps {
        fallback_white: path.white_texture.clone(),
        fallback_black: path.white_texture.clone(),
        fallback_normal: path.white_texture.clone(),
    });

    let static_material = |mat: Arc<bf::material::Material>| {
        StaticMaterial::from_material(
            mat.as_ref(),
            content,
            path.buffers.geometry_pipeline.clone(),
            path.samplers.aniso_repeat.clone(),
            engine.renderer_state.graphical_queue.clone(),
            fallback_maps.clone(),
        )
        .unwrap()
    };

    let apple = Object::new(
        content.load("apple.bf"),
        static_material(
            content
                .load_uuid::<bf::material::Material>(lookup("3DApple002_2K-JPG.mat"))
                .wait_for_then_unwrap(),
        )
        .0,
        Transform {
            scale: vec3(6.0, 6.0, 6.0),
            position: vec3(0.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let woman = Object::new(
        content.load_uuid(lookup(
            ".\\autumn_casualwoman_01/autumn_casualwoman_01_lowpoly_3dsmax.obj",
        )),
        static_material(
            content
                .load_uuid::<bf::material::Material>(lookup("autumn_casualwoman_01.mat"))
                .wait_for_then_unwrap(),
        )
        .0,
        Transform {
            scale: vec3(0.1, 0.1, 0.1),
            position: vec3(7.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let bread1 = Object::new(
        content.load("6f88a288-6ce9-5455-9bd8-3546c5b39467.bf"),
        static_material(
            content
                .load_uuid::<bf::material::Material>(lookup("3DBread001_LowPoly.mat"))
                .wait_for_then_unwrap(),
        )
        .0,
        Transform {
            scale: vec3(5.0, 5.0, 5.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock1 = Object::new(
        content.load("3f9e7780-6d4e-5108-9d36-23fc77339efb.bf"),
        static_material(
            content
                .load_uuid::<bf::material::Material>(lookup("3DRock001_2K.mat"))
                .wait_for_then_unwrap(),
        )
        .0,
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock2 = Object::new(
        content.load("1a55dc06-6577-5cb5-9184-7a3b8d1e0c5a.bf"),
        static_material(
            content
                .load_uuid::<bf::material::Material>(lookup("3DRock002_9K.mat"))
                .wait_for_then_unwrap(),
        )
        .0,
        Transform {
            scale: vec3(2.0, 2.0, 2.0),
            position: vec3(-3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

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
    .map(|x| content.load_uuid(lookup(x)))
    .map(|x| x.wait_for_then_unwrap())
    .map(|x: Arc<bf::material::Material>| static_material(x).0)
    .collect();

    let state = &mut engine.game_state;
    state.materials = materials;

    let plane = Object::new(
        content.load("plane.bf"),
        state.materials.get(0).unwrap().clone(),
        Transform {
            scale: vec3(10.0, 1.0, 10.0),
            ..Transform::default()
        },
    );
    info!("data loaded after {}s!", start.elapsed().as_secs_f32());

    state.objects_u16 = vec![plane, apple, bread1, rock1, rock2];
    state.objects_u32 = vec![woman];
}
