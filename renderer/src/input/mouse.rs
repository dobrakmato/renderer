//! Functionality related to handling mouse input.

use log::error;
use std::sync::Arc;
use vulkano::swapchain::Surface;
use winit::event::{DeviceEvent, MouseScrollDelta, WindowEvent};
use winit::window::Window;

/// Mouse input and state.
pub struct Mouse {
    input_enabled: bool,
    wheel_delta: (f64, f64),
    move_delta: (f64, f64),
    position: (f64, f64),
    window: Arc<Surface<Window>>,
}

// todo: decide when to use logical and physical position & deltas

impl Mouse {
    /// Creates a new Mouse input tracker tied to specified `Window` instance.
    pub fn new(window: Arc<Surface<Window>>) -> Self {
        Self {
            input_enabled: true,
            wheel_delta: (0.0, 0.0),
            move_delta: (0.0, 0.0),
            position: (0.0, 0.0),
            window,
        }
    }

    /// Returns whether is the mouse input currently responding
    /// to incoming mouse input events.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.input_enabled
    }

    /// Enables or disables the handling of `winit` mouse events.
    pub fn set_enabled(&mut self, input_enabled: bool) {
        self.input_enabled = input_enabled;
    }

    /// Sets whether the cursor is grabbed or not.
    pub fn set_cursor_grabbed(&self, grabbed: bool) {
        if let Err(e) = self.window.window().set_cursor_grab(grabbed) {
            error!("Cannot grab cursor: {:?}", e)
        }
    }

    /// Hides or shows the cursor.
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.window.window().set_cursor_visible(visible)
    }

    /// Returns the current cursor position in physical pixels. Because the range of this data is
    /// limited by the display area and it may have been transformed by the OS to implement effects such as cursor
    /// acceleration, it should not be used to implement non-cursor-like interactions such as 3D camera control.
    pub fn position(&self) -> (f64, f64) {
        self.position
    }

    /// Returns the cursor delta (x, y) between last frame and this frame.
    pub fn delta(&self) -> (f64, f64) {
        self.move_delta
    }

    /// Returns the mouse wheel delta (x, y) between last and this frame.
    pub fn wheel_delta(&self) -> (f64, f64) {
        self.wheel_delta
    }

    /// Should be called once per frame to maintain internal state to provide useful
    /// per-frame functions as "was keyboard button pressed during this frame".
    pub fn frame_finished(&mut self) {
        self.move_delta = (0.0, 0.0); // reset aggregated delta
        self.wheel_delta = (0.0, 0.0);
    }

    /// Handles mouse related `winit` events. Other events are silently ignored.
    pub fn handle_mouse_event(&mut self, event: &WindowEvent) {
        if !self.input_enabled {
            return;
        }

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.position.0 = position.x;
                self.position.1 = position.y;
            }
            _ => {}
        }
    }

    /// Handles mouse related `winit` events. Other events are silently ignored.
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if !self.input_enabled {
            return;
        }

        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.move_delta.0 += delta.0;
                self.move_delta.1 += delta.1;
            }
            DeviceEvent::MouseWheel { delta } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.wheel_delta.0 += *x as f64;
                    self.wheel_delta.1 += *y as f64;
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    self.wheel_delta.0 += pos.x;
                    self.wheel_delta.1 += pos.y;
                }
            },
            _ => {}
        }
    }
}
