use crate::input::Input;
use crate::render::RendererState;
use crate::GameState;
use cgmath::Rad;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

/// main struct containing everything
pub struct Engine {
    pub game_state: GameState,
    pub renderer_state: RendererState,
    pub input_state: Input,
    pub event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub fn update(&mut self) {
        /* game update for next frame */
        let speed = if self.input_state.is_key_down(VirtualKeyCode::LShift) {
            0.01
        } else {
            0.005
        };
        if self.input_state.is_key_down(VirtualKeyCode::A) {
            self.game_state.camera.move_left(speed)
        }
        if self.input_state.is_key_down(VirtualKeyCode::D) {
            self.game_state.camera.move_right(speed)
        }
        if self.input_state.is_key_down(VirtualKeyCode::S) {
            self.game_state.camera.move_backward(speed)
        }
        if self.input_state.is_key_down(VirtualKeyCode::W) {
            self.game_state.camera.move_forward(speed)
        }
        if self.input_state.is_key_down(VirtualKeyCode::Space) {
            self.game_state.camera.move_up(speed)
        }
        if self.input_state.is_key_down(VirtualKeyCode::LControl) {
            self.game_state.camera.move_down(speed)
        }

        // todo: this will be fixed when https://github.com/rust-windowing/winit/pull/1461 gets merged
        if self.input_state.is_key_down(VirtualKeyCode::F) {
            self.renderer_state
                .set_window_size(LogicalSize::new(1280.0, 720.0))
        }
    }

    pub fn run_forever(mut self) -> ! {
        self.event_loop
            .take()
            .unwrap()
            .run(move |ev, _, flow| match ev {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
                    WindowEvent::Focused(focus) => self.input_state.set_input_enabled(focus),
                    _ => {}
                },
                Event::DeviceEvent { event, .. } => {
                    if let DeviceEvent::Key(k) = event {
                        self.input_state.handle_event(k)
                    }
                    if let DeviceEvent::MouseMotion { delta } = event {
                        if self.input_state.input_enabled {
                            self.game_state
                                .camera
                                .rotate(Rad(delta.0 as f32 * 0.001), Rad(delta.1 as f32 * 0.001))
                        }
                    }
                }
                Event::RedrawEventsCleared => {
                    self.renderer_state.render_frame(&self.game_state);
                    self.update();
                }
                _ => {}
            });
    }
}
