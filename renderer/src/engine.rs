use crate::content::Content;
use crate::input::Input;
use crate::render::{RendererState, VulkanState};
use crate::{GameState, RendererConfiguration};
use cgmath::{vec3, InnerSpace, Rad};
use winit::event::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

/// main struct containing everything
pub struct Engine {
    pub game_state: GameState,
    vulkan_state: VulkanState,
    pub renderer_state: RendererState,
    input_state: Input,
    pub content: Content,
    event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub fn new(
        initial_state: GameState,
        conf: RendererConfiguration,
        event_loop: EventLoop<()>,
    ) -> Self {
        let vulkan_state = VulkanState::new(conf, &event_loop);
        let content = Content::new(vulkan_state.transfer_queue());
        let renderer_state = RendererState::new(&vulkan_state, &content);
        Self {
            game_state: initial_state,
            content,
            renderer_state,
            vulkan_state,
            input_state: Default::default(),
            event_loop: Some(event_loop),
        }
    }

    pub fn update(&mut self) {
        let (s, c) = self.game_state.start.elapsed().as_secs_f32().sin_cos();
        self.game_state.sun_dir = vec3(s, 1.33, c).normalize();

        /* game update for next frame */
        let speed = if self.input_state.is_key_down(VirtualKeyCode::LShift) {
            0.005
        } else {
            0.00125
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

        if self.input_state.was_key_pressed(VirtualKeyCode::F) {
            let obj = self.game_state.objects_u16.get_mut(0).unwrap();
            obj.material = self.game_state.materials
                [self.game_state.floor_mat % self.game_state.materials.len()]
            .clone();
            self.game_state.floor_mat += 1;
        }

        self.input_state.frame_finished();
    }

    pub fn run_forever(mut self) -> ! {
        self.event_loop
            .take()
            .unwrap()
            .run(move |ev, _, flow| match ev {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
                    WindowEvent::Resized(new_size) => {
                        self.game_state.camera.aspect_ratio =
                            new_size.width as f32 / new_size.height as f32
                    }
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
