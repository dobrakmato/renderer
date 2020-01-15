use cgmath::{Matrix4, PerspectiveFov, Point3, Rad, Vector3};

pub trait Camera<T> {
    fn projection_matrix(&self) -> Matrix4<T>;
    fn view_matrix(&self) -> Matrix4<T>;
}

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
