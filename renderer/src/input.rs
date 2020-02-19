use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub struct Input {
    key_state: [bool; 512],
    pub input_enabled: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            key_state: [false; 512],
            input_enabled: true,
        }
    }
}

impl Input {
    pub fn set_input_enabled(&mut self, input_enabled: bool) {
        // when we lose focus we disable all inputs
        if !input_enabled {
            self.key_state = [false; 512];
        }

        self.input_enabled = input_enabled;
    }

    pub fn is_key_down(&self, key: VirtualKeyCode) -> bool {
        self.key_state[key as u32 as usize]
    }

    pub fn is_key_up(&self, key: VirtualKeyCode) -> bool {
        !self.key_state[key as u32 as usize]
    }

    pub fn handle_event(&mut self, event: KeyboardInput) {
        if !self.input_enabled {
            return;
        }

        if let Some(t) = event.virtual_keycode {
            match event.state {
                ElementState::Pressed => self.key_state[t as u32 as usize] = true,
                ElementState::Released => self.key_state[t as u32 as usize] = false,
            }
        }
    }
}
