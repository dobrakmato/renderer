use crate::assets::Content;
use crate::input::Input;
use crate::movement::FpsMovement;
use crate::render::renderer::RendererState;
use crate::render::ubo::DirectionalLight;
use crate::render::vulkan::VulkanState;
use crate::{GameState, RendererConfiguration};
use cgmath::{InnerSpace, Vector3};
use rand::Rng;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

/// main struct containing everything
pub struct Engine {
    pub game_state: GameState,
    pub vulkan_state: VulkanState,
    pub renderer_state: RendererState,
    pub input_state: Input,
    pub content: Content,
    event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub fn new(
        initial_state: GameState,
        conf: &RendererConfiguration,
        event_loop: EventLoop<()>,
    ) -> Self {
        let vulkan_state = VulkanState::new(conf, &event_loop).expect("cannot create VulkanState");
        let content = Content::new(8, vulkan_state.transfer_queue(), conf.content_roots.clone());
        let renderer_state =
            RendererState::new(&vulkan_state).expect("cannot create RendererState");
        let input_state = Input::new(vulkan_state.surface());
        Self {
            game_state: initial_state,
            renderer_state,
            vulkan_state,
            content,
            input_state,
            event_loop: Some(event_loop),
        }
    }

    pub fn update(&mut self) {
        FpsMovement::update(&mut self.game_state.camera, &self.input_state);

        let sec = self.game_state.start.elapsed().as_secs_f32() * 0.1;
        let (s, c) = sec.sin_cos();

        // self.game_state.directional_lights[0].direction.x = s;
        // self.game_state.directional_lights[0].direction.z = c;
        // self.game_state.directional_lights[0].direction.normalize();
        // self.game_state.directional_lights[1].direction.x = -s;
        // self.game_state.directional_lights[1].direction.z = -c;
        // self.game_state.directional_lights[1].direction.normalize();

        self.vulkan_state
            .surface()
            .window()
            .set_title(&format!("{:?}", self.game_state.camera.position));

        if self.input_state.keyboard.was_key_pressed(VirtualKeyCode::F) {
            let obj = self.game_state.objects.get_mut(0).unwrap();
            obj.material = self.game_state.materials
                [self.game_state.floor_mat % self.game_state.materials.len()]
            .clone();
            self.game_state.floor_mat += 1;
        }

        if self.input_state.keyboard.was_key_pressed(VirtualKeyCode::L) {
            let mut rng = rand::thread_rng();
            self.game_state.directional_lights.push(DirectionalLight {
                direction: Vector3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(0.0..2.0),
                    rng.gen_range(-1.0..1.0),
                )
                .normalize(),
                intensity: 1.0,
                color: Vector3::new(
                    rng.gen_range(0.3..1.0),
                    rng.gen_range(0.3..1.0),
                    rng.gen_range(0.3..1.0),
                ),
            })
        }
    }

    pub fn run_forever(mut self) -> ! {
        self.event_loop
            .take()
            .unwrap()
            .run(move |ev, _, flow| match ev {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
                    WindowEvent::Focused(focus) => self.input_state.set_enabled(focus),
                    WindowEvent::Resized(new_size) => {
                        self.game_state.camera.aspect_ratio =
                            new_size.width as f32 / new_size.height as f32
                    }
                    _ => {}
                },
                Event::DeviceEvent { event, .. } => self.input_state.handle_device_event(&event),
                Event::RedrawEventsCleared => {
                    self.renderer_state.render_frame(&self.game_state);
                    self.update();
                    self.input_state.frame_finished();
                }
                _ => {}
            });
    }
}
