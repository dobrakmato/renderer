use crate::camera::PerspectiveCamera;
use crate::input::Input;
use crate::render::{FrameSystem, Renderer};
use crate::samplers::Samplers;
use crate::window::{SwapChain, Window};
use cgmath::{vec3, Deg, InnerSpace, Point3, Rad, Vector3};
use log::warn;
use std::time::Instant;
use vulkano::pipeline::viewport::Viewport;
use winit::event::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;

mod camera;
mod hosek;
mod image;
mod input;
mod io;
mod mesh;
mod pod;
mod render;
mod samplers;
mod shaders;
mod sky;
mod window;

pub struct Configuration {
    pub fullscreen: bool,
    pub resolution: [u16; 2],
    pub gpu: usize,
}

pub struct GameState {
    start: Instant,
    sun_dir: Vector3<f32>,
    camera: PerspectiveCamera,
}

fn main() {
    // initialize logging at start of the application
    simple_logger::init().unwrap();

    // load configuration from a file
    let conf = Configuration {
        fullscreen: false,
        resolution: [1600, 900],
        gpu: 0,
    };

    #[cfg(debug_assertions)]
    warn!("this is a debug build. performance may hurt.");

    // initialize vulkan and swapchain
    let mut app = Window::new(conf);
    let mut swapchain = SwapChain::new(
        app.surface.clone(),
        app.device.clone(),
        app.graphical_queue.clone(),
    );

    // grab the cursor and hide it
    app.surface.window().set_cursor_grab(true).unwrap();
    app.surface.window().set_cursor_visible(false);

    // todo: improve input handling
    let mut input = Input::default();

    let dims = swapchain.dimensions();
    let renderer = Renderer {
        viewport: Viewport {
            origin: [0.0, 0.0],
            dimensions: [dims[0] as f32, dims[1] as f32],
            depth_range: 0.0..1.0,
        },
        samplers: Samplers::new(app.device.clone()).expect("cannot create samplers"),
        graphical_queue: app.graphical_queue.clone(),
        device: app.device.clone(),
    };
    let mut state = GameState {
        start: Instant::now(),
        sun_dir: vec3(0.0, 0.5, 0.0).normalize(),
        camera: PerspectiveCamera {
            position: Point3::new(0.0, 3.0, 0.0),
            forward: vec3(1.0, 0.0, 0.0),
            up: vec3(0.0, -1.0, 0.0),
            fov: Deg(120.0).into(),
            aspect_ratio: dims[0] as f32 / dims[1] as f32,
            near: 0.01,
            far: 100.0,
        },
    };

    let mut frame_system = FrameSystem::new(&renderer, &swapchain);

    app.event_loop.run(move |ev, _, flow| match ev {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
            WindowEvent::Focused(focus) => input.set_input_state(focus),
            _ => {}
        },
        Event::DeviceEvent { event, .. } => {
            if let DeviceEvent::Key(k) = event {
                input.handle_event(k)
            }
            if let DeviceEvent::MouseMotion { delta } = event {
                if input.input_enabled {
                    state
                        .camera
                        .rotate(Rad(delta.0 as f32 * 0.001), Rad(delta.1 as f32 * 0.001))
                }
            }
        }
        Event::RedrawEventsCleared => {
            // PART 1: Render
            swapchain.render_frame(|image_num, color_attachment| {
                // todo: do not recreate frame object
                let frame = frame_system.create_frame(color_attachment);
                frame.render(&renderer, &state)
            });

            // PART 2: Update

            /* game update for next frame */
            let speed = if input.is_key_down(VirtualKeyCode::LShift) {
                0.01
            } else {
                0.005
            };
            if input.is_key_down(VirtualKeyCode::A) {
                state.camera.move_left(speed)
            }
            if input.is_key_down(VirtualKeyCode::D) {
                state.camera.move_right(speed)
            }
            if input.is_key_down(VirtualKeyCode::S) {
                state.camera.move_backward(speed)
            }
            if input.is_key_down(VirtualKeyCode::W) {
                state.camera.move_forward(speed)
            }
            if input.is_key_down(VirtualKeyCode::Space) {
                state.camera.move_up(speed)
            }
            if input.is_key_down(VirtualKeyCode::LControl) {
                state.camera.move_down(speed)
            }
        }
        _ => {}
    });
}
