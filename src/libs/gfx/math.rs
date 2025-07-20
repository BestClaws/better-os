use micromath::F32Ext;

#[derive(Clone, Copy)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn normalize(self) -> Self {
        let mag = (self.0 * self.0 + self.1 * self.1 + self.2 * self.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, 0.0)
        } else {
            Vec3(self.0 / mag, self.1 / mag, self.2 / mag)
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2
    }
}

#[derive(Clone, Copy)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let (sin_a, cos_a) = (angle_rad / 2.0).sin_cos();
        let mag = (axis.0 * axis.0 + axis.1 * axis.1 + axis.2 * axis.2).sqrt();
        let (x, y, z) = if mag < 0.0001 {
            (0.0, 0.0, 0.0)
        } else {
            (axis.0 / mag * sin_a, axis.1 / mag * sin_a, axis.2 / mag * sin_a)
        };
        Quaternion { w: cos_a, x, y, z }
    }

    pub fn mul(self, other: Quaternion) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    pub fn rotate_vector(self, v: Vec3) -> Vec3 {
        let q_vec = Quaternion { w: 0.0, x: v.0, y: v.1, z: v.2 };
        let q_conj = Quaternion { w: self.w, x: -self.x, y: -self.y, z: -self.z };
        let result = self.mul(q_vec).mul(q_conj);
        Vec3(result.x, result.y, result.z)
    }
}