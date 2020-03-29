use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use crate::material::{Material, MaterialDesc};
use crate::render::{BasicVertex, Object, Transform};
use cgmath::{vec3, Deg, InnerSpace, Point3, Vector3};
use log::{info, warn};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::EventLoop;

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
    sun_dir: Vector3<f32>,
    camera: PerspectiveCamera,
    objects_u16: Vec<Object<BasicVertex, u16>>,
    objects_u32: Vec<Object<BasicVertex, u32>>,
    materials: Vec<Arc<Material>>,
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
            sun_dir: vec3(0.0, 0.5, 0.0).normalize(),
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
            materials: vec![],
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

    let plane = Object::new(
        content.load("plane.bf"),
        content
            .load::<MaterialDesc, _>("[2K]Leather11.json")
            .wait_for_then_unwrap()
            .to_material(
                content,
                path.buffers.geometry_pipeline.clone(),
                path.samplers.aniso_repeat.clone(),
                path.white_texture.clone(),
            ),
        Transform {
            scale: vec3(10.0, 1.0, 10.0),
            ..Transform::default()
        },
    );

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

    let materials = [
        "[2K]Bricks22.json",
        "[2K]Concrete07.json",
        "[2K]Ground27.json",
        "[2K]Ground30.json",
        "[2K]Marble04.json",
        "[2K]Marble06.json",
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

    info!("data loaded after {}s!", start.elapsed().as_secs_f32());

    let state = &mut engine.game_state;
    state.materials = materials;
    state.objects_u16 = vec![plane, rock, apple];
}
