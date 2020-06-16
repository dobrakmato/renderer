//! Functionality related to handling keyboard input.

use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

/// Keyboard input and state.
pub struct Keyboard {
    current_key_state: [bool; 512],
    previous_key_state: [bool; 512],
    pub input_enabled: bool,
}

// need this because we want input to be enabled from start
impl Default for Keyboard {
    fn default() -> Self {
        Self {
            current_key_state: [false; 512],
            previous_key_state: [false; 512],
            input_enabled: true,
        }
    }
}

impl Keyboard {
    /// Returns whether is the keyboard input currently responding
    /// to incoming keyboard input events.
    #[inline]
    pub fn is_enabled(&mut self) -> bool {
        self.input_enabled
    }

    /// Enables or disables the handling of `winit` keyboard events.
    pub fn set_enabled(&mut self, input_enabled: bool) {
        // when we lose focus we trigger release all pressed keys to prevent bugs
        if !input_enabled {
            self.current_key_state = [false; 512];
        }

        self.input_enabled = input_enabled;
    }

    /// Returns whether the user is currently (in this frame)
    /// holding down the key specified by `VirtualKeyCode`.
    ///
    /// If you want to trigger an action on key press use
    /// `was_key_pressed` as it returns only on frame when the
    /// specified key was pressed the first time.
    pub fn is_key_down(&self, key: VirtualKeyCode) -> bool {
        self.current_key_state[key as u32 as usize]
    }

    /// Returns whether the user is currently (in this frame)
    /// not holding down the key specified by `VirtualKeyCode`.
    ///
    /// If you want to trigger an action on key release use
    /// `was_key_release` as it returns only on frame when the
    /// specified key was released the first time.
    pub fn is_key_up(&self, key: VirtualKeyCode) -> bool {
        !self.current_key_state[key as u32 as usize]
    }

    /// Returns whether the user starter pressing the key specified
    /// by `VirtualKeyCode` in this frame.
    ///
    /// If you want to run code on each frame the key is pressed
    /// use `is_key_down` method.
    pub fn was_key_pressed(&self, key: VirtualKeyCode) -> bool {
        !self.previous_key_state[key as u32 as usize] && self.is_key_down(key)
    }

    /// Returns whether the user starter pressing the key specified
    /// by `VirtualKeyCode` in this frame.
    ///
    /// If you want to run code on each frame the key is pressed
    /// use `is_key_down` method.
    pub fn was_key_released(&self, key: VirtualKeyCode) -> bool {
        self.previous_key_state[key as u32 as usize] && self.is_key_up(key)
    }

    /// Should be called once per frame to maintain internal state to provide useful
    /// per-frame functions as "was keyboard button pressed during this frame".
    pub fn frame_finished(&mut self) {
        self.previous_key_state = self.current_key_state;
    }

    /// Modifies the internal state according to specified `winit` keyboard event.
    pub fn handle_event(&mut self, event: KeyboardInput) {
        if !self.input_enabled {
            return;
        }

        if let Some(t) = event.virtual_keycode {
            match event.state {
                ElementState::Pressed => self.current_key_state[t as u32 as usize] = true,
                ElementState::Released => self.current_key_state[t as u32 as usize] = false,
            }
        }
    }
}
