use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use crate::material::{Material, MaterialDesc};
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
                intensity: 0.3,
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

    let rock = Object::new(
        content.load("Rock_1.bf"),
        content
            .load::<MaterialDesc, _>("mat_rock.json")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(0.03, 0.03, 0.03),
            position: vec3(5.0, 0.3, 0.0),
            ..Transform::default()
        },
    );

    let apple = Object::new(
        content.load("apple.bf"),
        content
            .load::<MaterialDesc, _>("3DApple002_2K-JPG.json")
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
            .load::<MaterialDesc, _>("autumn_casualwoman_01.json")
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
        content.load("1a0a130b5af1159eaaac0bad497b1595.bf"),
        content
            .load::<MaterialDesc, _>("3DBread001_LowPoly.json")
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
        content.load("8a67105b70c7ea6c75a8216a52efd628.bf"),
        content
            .load::<MaterialDesc, _>("3DRock001_2K.json")
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
        content.load("8ae3a34c17f6b4bf2470941fafef88b7.bf"),
        content
            .load::<MaterialDesc, _>("3DRock002_9K.json")
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
        "[2K]Bricks22.json",
        "[2K]Concrete07.json",
        "[2K]Ground27.json",
        "[2K]Ground30.json",
        "[2K]Ground37.json",
        "[2K]Leather11.json",
        "[2K]Marble04.json",
        "[2K]Marble06.json",
        "[2K]Metal07.json",
        "[2K]Metal08.json",
        "[2K]Metal27.json",
        "[2K]Metal28.json",
        "[2K]PaintedPlaster05.json",
        "[2K]PavingStones42.json",
        "[2K]PavingStones53.json",
        "[2K]Planks12.json",
        "[2K]SolarPanel03.json",
        "[2K]Tiles15.json",
        "[2K]Tiles44.json",
        "[2K]Tiles52.json",
        "[2K]Wood18.json",
        "[2K]Wood35.json",
        "[2K]WoodFloor12.json",
        "[2K]WoodFloor32.json",
        "Bricks027_2K-JPG.json",
        "Bricks037_2K-JPG.json",
        "Carpet013_2K-JPG.json",
        "CorrugatedSteel005_2K-JPG.json",
        "Fabric031_2K-JPG.json",
        "Fabric032_2K-JPG.json",
        "Ground036_2K-JPG.json",
        "Ice004_2K-JPG.json",
        "Leather021_2K-JPG.json",
        "Metal017_2K-JPG.json",
        "Paint002_2K-JPG.json",
        "PaintedWood005_2K-JPG.json",
        "PavingStones055_2K-JPG.json",
        "Road006_2K-JPG.json",
        "Rock020_2K-JPG.json",
        "Rocks017_2K-JPG.json",
        "Terrazzo003_2K-JPG.json",
        "Tiles059_2K-JPG.json",
        "Tiles072_2K-JPG.json",
        "WoodSiding007_2K-JPG.json",
    ]
    .iter()
    .map(|x| content.load(*x))
    .map(|x| x.wait_for_then_unwrap())
    .map(|x: Arc<MaterialDesc>| {
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

    state.objects_u16 = vec![plane, rock, apple, bread1, rock1, rock2];
    state.objects_u32 = vec![woman];
}
