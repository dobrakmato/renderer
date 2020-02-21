use crate::camera::PerspectiveCamera;
use crate::engine::Engine;
use cgmath::{vec3, Deg, InnerSpace, Point3, Vector3};
use log::warn;
use std::time::Instant;
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::EventLoop;

mod camera;
mod content;
mod engine;
mod hosek;
mod image;
mod input;
mod io;
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
    let engine = Engine::new(
        GameState {
            start: Instant::now(),
            sun_dir: vec3(0.0, 0.5, 0.0).normalize(),
            camera: PerspectiveCamera {
                position: Point3::new(0.0, 3.0, 0.0),
                forward: vec3(1.0, 0.0, 0.0),
                up: vec3(0.0, -1.0, 0.0),
                fov: Deg(90.0).into(),
                aspect_ratio: conf.resolution[0] as f32 / conf.resolution[1] as f32,
                near: 0.01,
                far: 100.0,
            },
        },
        conf,
        event_loop,
    );
    engine.run_forever();
}
