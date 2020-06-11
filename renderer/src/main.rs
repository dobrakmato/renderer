use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use crate::material::{Material, MaterialExt};
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
    materials: Vec<Arc<Material>>,
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
                direction: vec3(3.0, 5.0, 1.0).normalize(),
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

    let apple = Object::new(
        content.load("apple.bf"),
        content
            .load::<bf::material::Material, _>("3DApple002_2K-JPG.bf")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(6.0, 6.0, 6.0),
            position: vec3(0.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let woman = Object::new(
        content.load("autumn_casualwoman_01_lowpoly_3dsmax.bf"),
        content
            .load::<bf::material::Material, _>("autumn_casualwoman_01.bf")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(0.1, 0.1, 0.1),
            position: vec3(7.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let bread1 = Object::new(
        content.load("6f88a288-6ce9-5455-9bd8-3546c5b39467.bf"),
        content
            .load::<bf::material::Material, _>("3DBread001_LowPoly.bf")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(5.0, 5.0, 5.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock1 = Object::new(
        content.load("3f9e7780-6d4e-5108-9d36-23fc77339efb.bf"),
        content
            .load::<bf::material::Material, _>("3DRock001_2K.bf")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(1.0, 1.0, 1.0),
            position: vec3(3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let rock2 = Object::new(
        content.load("1a55dc06-6577-5cb5-9184-7a3b8d1e0c5a.bf"),
        content
            .load::<bf::material::Material, _>("3DRock002_9K.bf")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(2.0, 2.0, 2.0),
            position: vec3(-3.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let materials = [
        "[2K]Bricks22.bf",
        "[2K]Concrete07.bf",
        "[2K]Ground27.bf",
        "[2K]Ground30.bf",
        "[2K]Ground37.bf",
        "[2K]Leather11.bf",
        "[2K]Marble04.bf",
        "[2K]Marble06.bf",
        "[2K]Metal07.bf",
        "[2K]Metal08.bf",
        "[2K]Metal27.bf",
        "[2K]Metal28.bf",
        "[2K]PaintedPlaster05.bf",
        "[2K]PavingStones42.bf",
        "[2K]PavingStones53.bf",
        "[2K]Planks12.bf",
        "[2K]SolarPanel03.bf",
        "[2K]Tiles15.bf",
        "[2K]Tiles44.bf",
        "[2K]Tiles52.bf",
        "[2K]Wood18.bf",
        "[2K]Wood35.bf",
        "[2K]WoodFloor12.bf",
        "[2K]WoodFloor32.bf",
        "Bricks027_2K-JPG.bf",
        "Bricks037_2K-JPG.bf",
        "Carpet013_2K-JPG.bf",
        "CorrugatedSteel005_2K-JPG.bf",
        "Fabric031_2K-JPG.bf",
        "Fabric032_2K-JPG.bf",
        "Ground036_2K-JPG.bf",
        "Ice004_2K-JPG.bf",
        "Leather021_2K-JPG.bf",
        "Metal017_2K-JPG.bf",
        "Paint002_2K-JPG.bf",
        "PaintedWood005_2K-JPG.bf",
        "PavingStones055_2K-JPG.bf",
        "Road006_2K-JPG.bf",
        "Rock020_2K-JPG.bf",
        "Rocks017_2K-JPG.bf",
        "Terrazzo003_2K-JPG.bf",
        "Tiles059_2K-JPG.bf",
        "Tiles072_2K-JPG.bf",
        "WoodSiding007_2K-JPG.bf",
    ]
    .iter()
    .map(|x| content.load(*x))
    .map(|x| x.wait_for_then_unwrap())
    .map(|x: Arc<bf::material::Material>| {
        x.to_material(
            content,
            path.buffers.geometry_pipeline.clone(),
            path.samplers.aniso_repeat.clone(),
            path.white_texture.clone(),
        )
    })
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
