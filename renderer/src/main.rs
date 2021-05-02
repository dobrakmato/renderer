use crate::camera::PerspectiveCamera;
use crate::config::RendererConfiguration;
use crate::engine::Engine;
use crate::render::object::Object;
use crate::render::ubo::DirectionalLight;
use crate::render::vertex::NormalMappedVertex;
use crate::resources::material::StaticMaterial;
use cgmath::{vec3, Deg, InnerSpace, Point3};
use log::{info, LevelFilter};
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::EventLoop;

mod assets;
mod camera;
mod config;
mod engine;
mod input;
mod movement;
mod render;
mod resources;
mod scenes;

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
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

    // load configuration
    let conf = RendererConfiguration::default();

    // start event loop
    let event_loop = EventLoop::new();

    // initialize engine
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
            directional_lights: vec![
                DirectionalLight {
                    direction: vec3(5.0, 5.0, 1.0).normalize(),
                    intensity: 2.5,
                    color: vec3(1.0, 1.0, 0.8),
                },
                DirectionalLight {
                    direction: vec3(-5.0, 5.0, 1.0).normalize(),
                    intensity: 2.5,
                    color: vec3(0.8, 1.0, 1.0),
                },
            ],
            materials: vec![],
            floor_mat: 0,
        },
        &conf,
        event_loop,
    );

    // load scene and data
    load(&mut engine);

    // run engine
    engine.run_forever();
}

fn load(engine: &mut Engine) {
    info!("Loading scene and data...");

    scenes::transparency::create(engine);
}
