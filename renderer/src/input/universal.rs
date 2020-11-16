use core::lerp;
use std::collections::HashMap;
use winit::event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode};

pub const MOUSE_X: &str = "Mouse X";
pub const MOUSE_Y: &str = "Mouse Y";

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Binding {
    KeyboardButton(VirtualKeyCode),
    MouseMovementX,
    MouseMovementY,
}

/// Axis represents an analog like input controller that
/// ranges usually between `-1.0` and `1.0`. As the input might
/// be from a digital device (eg. a keyboard), it has support
/// for configurable smoothing parameter to imitate analog controller.
pub struct Axis {
    /// Specifies speed of smoothing (`0.0` to `1.0`).
    pub smoothing: f32,
    pub dead_zone: f32,
    pub value: f32,
    pub raw_value: f32,
}

impl Axis {
    pub fn new() -> Self {
        Self {
            smoothing: 0.75,
            dead_zone: 0.05,
            value: 0.0,
            raw_value: 0.0,
        }
    }

    fn accept_value(&mut self, value: f32) {
        self.raw_value = value;
        self.apply_smoothing();
    }

    fn apply_smoothing(&mut self) {
        self.value = lerp(self.value, self.raw_value, 1.0 - self.smoothing)
    }

    #[inline]
    fn value(&self) -> f32 {
        if self.value.abs() < self.dead_zone {
            return 0.0;
        }

        self.value
    }
}

/// A digital input controller that is either pressed
/// or released.
pub struct Button {
    pub down: bool,
    pub was_pressed: bool,
    pub was_released: bool,
}

impl Button {
    pub fn new() -> Button {
        Self {
            down: false,
            was_pressed: false,
            was_released: false,
        }
    }

    fn accept_state(&mut self, down: bool) {
        self.was_pressed = !self.down & down;
        self.was_released = self.down & !down;
        self.down = down;
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum Mapping {
    Axis(&'static str, f32),
    Button(&'static str),
}

/// Universal abstract input device that supports multiple
/// concrete input devices (such as mouse and keyboard) and has
/// support for configurable mapping (keybindings) of individual
/// physical devices to this.
pub struct Universal {
    /// All existing axes.
    axes: HashMap<&'static str, Axis>,
    /// All existing buttons.
    buttons: HashMap<&'static str, Button>,

    bindings: HashMap<Binding, Vec<Mapping>>,
    input_enabled: bool,
}

impl Universal {
    /// Returns whether is the keyboard input currently responding
    /// to incoming keyboard input events.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.input_enabled
    }

    /// Enables or disables the handling of `winit` keyboard events.
    pub fn set_enabled(&mut self, input_enabled: bool) {
        // when we lose focus we trigger release all pressed keys to prevent bugs
        if !input_enabled {
            self.reset_all_inputs();
        }

        self.input_enabled = input_enabled;
    }

    pub fn reset_all_inputs(&mut self) {
        for axis in self.axes.values_mut() {
            axis.value = 0.0;
            axis.raw_value = 0.0;
        }

        for button in self.buttons.values_mut() {
            button.accept_state(false);
        }
    }

    pub fn axis(&self, name: &'static str) -> f32 {
        self.axes[name].value()
    }

    pub fn axis_raw(&self, name: &'static str) -> f32 {
        self.axes[name].raw_value
    }

    pub fn is_button_down(&self, name: &'static str) -> bool {
        self.buttons[name].down
    }

    pub fn is_button_up(&self, name: &'static str) -> bool {
        !self.buttons[name].down
    }

    pub fn was_pressed(&self, name: &'static str) -> bool {
        self.buttons[name].was_pressed
    }

    pub fn was_released(&self, name: &'static str) -> bool {
        self.buttons[name].was_released
    }

    pub fn handle_event(&mut self, input_event: &DeviceEvent) {
        if !self.input_enabled {
            return;
        }

        match input_event {
            DeviceEvent::MouseMotion { delta } => self.accept_mouse_movement(*delta),
            DeviceEvent::Key(k) => self.accept_keyboard_input(*k),
            _ => {}
        }
    }

    fn accept_keyboard_input(&mut self, k: KeyboardInput) {
        let binding = Binding::KeyboardButton(k.virtual_keycode.unwrap());

        // get list of mappings that are bound to this binding
        if let Some(mappings) = self.bindings.get(&binding) {
            // we iterate over mappings and try to send input
            // to all of them by matching on the mapping type
            // then acquiring mutable reference from the internal
            // hashmap and finally by calling the accept_value
            // method on the axis/button.
            for mapping in mappings {
                match mapping {
                    Mapping::Axis(axis_id, weight) => {
                        if let Some(axis) = self.axes.get_mut(axis_id) {
                            let value = weight
                                * if k.state == ElementState::Pressed {
                                    1.0
                                } else {
                                    0.0
                                };

                            axis.accept_value(value)
                        }
                    }
                    Mapping::Button(button_id) => {
                        if let Some(button) = self.buttons.get_mut(button_id) {
                            button.accept_state(k.state == ElementState::Pressed)
                        }
                    }
                }
            }
        }
    }

    fn accept_mouse_movement(&mut self, delta: (f64, f64)) {
        macro_rules! mouse_movement {
            ($binding: expr, $val: expr) => {
                if let Some(mappings) = self.bindings.get(&$binding) {
                    for mapping in mappings {
                        match mapping {
                            Mapping::Axis(a, _) => {
                                if let Some(axis) = self.axes.get_mut(a) {
                                    axis.accept_value($val as f32)
                                }
                            }
                            Mapping::Button(b) => {
                                if let Some(button) = self.buttons.get_mut(b) {
                                    button.accept_state($val > 0.0)
                                }
                            }
                        }
                    }
                }
            };
        }

        mouse_movement!(Binding::MouseMovementX, delta.0);
        mouse_movement!(Binding::MouseMovementY, delta.1);
    }

    /// Should be called once per frame to maintain internal state.
    pub fn frame_finished(&mut self) {
        for axis in self.axes.values_mut() {
            axis.apply_smoothing();
        }

        if let Some(t) = self.axes.get_mut(MOUSE_X) {
            t.raw_value = 0.0;
        }

        if let Some(t) = self.axes.get_mut(MOUSE_Y) {
            t.raw_value = 0.0;
        }
    }
}

/// Implements a default key maps that uses keyboard and mouse.
impl Default for Universal {
    fn default() -> Self {
        let axes = ["MoveForward", "MoveRight", "MoveUp", MOUSE_X, MOUSE_Y];
        let buttons = ["Sprint"];

        Universal {
            axes: axes.iter().map(|c| (*c, Axis::new())).collect(),
            buttons: buttons.iter().map(|c| (*c, Button::new())).collect(),
            bindings: vec![
                (
                    Binding::KeyboardButton(VirtualKeyCode::W),
                    vec![Mapping::Axis("MoveForward", 1.0)],
                ),
                (
                    Binding::KeyboardButton(VirtualKeyCode::S),
                    vec![Mapping::Axis("MoveForward", -1.0)],
                ),
                (
                    Binding::KeyboardButton(VirtualKeyCode::D),
                    vec![Mapping::Axis("MoveRight", 1.0)],
                ),
                (
                    Binding::KeyboardButton(VirtualKeyCode::A),
                    vec![Mapping::Axis("MoveRight", -1.0)],
                ),
                (
                    Binding::KeyboardButton(VirtualKeyCode::Space),
                    vec![Mapping::Axis("MoveUp", 1.0)],
                ),
                (
                    Binding::KeyboardButton(VirtualKeyCode::LControl),
                    vec![Mapping::Axis("MoveUp", -1.0)],
                ),
                (Binding::MouseMovementX, vec![Mapping::Axis("Mouse X", 1.0)]),
                (Binding::MouseMovementY, vec![Mapping::Axis("Mouse Y", 1.0)]),
                (
                    Binding::KeyboardButton(VirtualKeyCode::LShift),
                    vec![Mapping::Button("Sprint")],
                ),
            ]
            .into_iter()
            .collect(),
            input_enabled: true,
        }
    }
}
