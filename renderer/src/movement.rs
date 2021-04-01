//! Various input handling movement controllers.

use crate::camera::PerspectiveCamera;
use crate::input::Input;
use cgmath::Rad;

/// Provides simple FPS-like free movement controller for camera.
pub struct FpsMovement;

impl FpsMovement {
    pub fn update(camera: &mut PerspectiveCamera, input: &Input) {
        let speed = if input.universal.is_button_down("Sprint") {
            4.0 * 0.005
        } else {
            4.0 * 0.00125
        };

        camera.move_right(speed * input.universal.axis("MoveRight"));
        camera.move_forward(speed * input.universal.axis("MoveForward"));
        camera.move_up(speed * input.universal.axis("MoveUp"));

        camera.rotate(
            Rad(input.universal.axis_raw("Mouse X") * 0.001),
            Rad(input.universal.axis_raw("Mouse Y") * 0.001),
        )
    }
}
