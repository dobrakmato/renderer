use cgmath::{vec3, InnerSpace, Matrix4, PerspectiveFov, Point3, Rad, Transform, Vector3};

pub trait Camera<T> {
    fn projection_matrix(&self) -> Matrix4<T>;
    fn view_matrix(&self) -> Matrix4<T>;
}

// todo: separate camera (render) from camera (movement/script)
// todo: use quaternion for camera rotation
pub struct PerspectiveCamera {
    pub position: Point3<f32>,
    pub forward: Vector3<f32>,
    pub up: Vector3<f32>,
    pub fov: Rad<f32>,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    #[inline]
    pub fn move_forward(&mut self, amount: f32) {
        self.position += self.forward * amount;
    }

    #[inline]
    pub fn move_backward(&mut self, amount: f32) {
        self.move_forward(-amount);
    }

    #[inline]
    pub fn move_right(&mut self, amount: f32) {
        self.move_left(-amount);
    }

    #[inline]
    pub fn move_left(&mut self, amount: f32) {
        let left = self.up.cross(self.forward);
        self.position += left * amount;
    }

    #[inline]
    pub fn move_up(&mut self, amount: f32) {
        self.position += Vector3::new(0.0, amount, 0.0);
    }

    #[inline]
    pub fn move_down(&mut self, amount: f32) {
        self.move_up(-amount);
    }

    #[inline]
    pub fn rotate(&mut self, dx: Rad<f32>, dy: Rad<f32>) {
        let rx = Matrix4::from_angle_y(dx);
        self.forward = rx.transform_vector(self.forward).normalize();

        let right = self.forward.cross(self.up).normalize();
        let old_forward = self.forward;
        let ry = Matrix4::from_axis_angle(right, dy);
        self.forward = ry.transform_vector(self.forward).normalize();
        let angle = self.forward.dot(vec3(0.0, 1.0, 0.0));
        if angle.abs() > 0.999 {
            self.forward = old_forward;
        }
    }
}

impl Camera<f32> for PerspectiveCamera {
    fn projection_matrix(&self) -> Matrix4<f32> {
        PerspectiveFov {
            fovy: self.fov,
            aspect: self.aspect_ratio,
            near: self.near,
            far: self.far,
        }
        .into()
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(self.position, self.forward, self.up)
    }
}
