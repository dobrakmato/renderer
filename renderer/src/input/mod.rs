//! Keyboard, mouse & virtual input (keybindings).

use crate::input::keyboard::Keyboard;
use crate::input::mouse::Mouse;
use crate::input::universal::Universal;
use std::sync::Arc;
use vulkano::swapchain::Surface;
use winit::event::DeviceEvent;
use winit::window::Window;

mod keyboard;
mod mouse;
mod universal;

/// Provides access to keyboard & mouse input.
pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
    pub universal: Universal,
}

impl Input {
    pub fn new(window: Arc<Surface<Window>>) -> Self {
        Self {
            keyboard: Keyboard::default(),
            mouse: Mouse::new(window),
            universal: Universal::default(),
        }
    }

    /// Enables or disables the handling of input events on all
    /// input types (mouse, keyboard, universal).
    pub fn set_enabled(&mut self, input_enabled: bool) {
        self.keyboard.set_enabled(input_enabled);
        self.mouse.set_enabled(input_enabled);
        self.universal.set_enabled(input_enabled);
    }

    /// Should be called once per frame to maintain internal state.
    pub fn frame_finished(&mut self) {
        self.universal.frame_finished();
        self.keyboard.frame_finished();
        self.mouse.frame_finished();
    }

    /// Handles mouse & keyboard related `winit` events. Other events are silently ignored.
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::Key(k) = event {
            self.keyboard.handle_event(*k);
        }

        if let DeviceEvent::MouseMotion { .. } | DeviceEvent::MouseWheel { .. } = event {
            self.mouse.handle_device_event(event);
        }

        self.universal.handle_event(event);
    }
}
