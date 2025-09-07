use defmt::Format;
use micromath::F32Ext;

#[derive(Copy, Clone, Debug, Format)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }

    pub fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }

    pub fn dot(self, rhs: Vec3) -> f32 {
        self.0 * rhs.0 + self.1 * rhs.1 + self.2 * rhs.2
    }

    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3(
            self.1 * rhs.2 - self.2 * rhs.1,
            self.2 * rhs.0 - self.0 * rhs.2,
            self.0 * rhs.1 - self.1 * rhs.0,
        )
    }

    pub fn scale(self, s: f32) -> Vec3 {
        Vec3(self.0 * s, self.1 * s, self.2 * s)
    }

    pub fn normalize(self) -> Vec3 {
        let mag = (self.0 * self.0 + self.1 * self.1 + self.2 * self.2).sqrt();
        if mag > 0.0 {
            self.scale(1.0 / mag)
        } else {
            self
        }
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.0 * self.0 + self.1 * self.1 + self.2 * self.2
    }
}


/// A quaternion representing rotation
#[derive(Copy, Clone, Debug, Format)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}


impl Quaternion {
    /// Rotates a vector by this quaternion
    pub fn rotate_vector(self, v: Vec3) -> Vec3 {
        let u = Vec3(self.x, self.y, self.z);
        let s = self.w;

        let uv = u.cross(v);
        let uuv = u.cross(uv);

        v.add(uv.scale(2.0 * s)).add(uuv.scale(2.0))
    }

    pub fn magnitude(&self) -> f32 {
        libm::sqrt((self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z) as f64)
            as f32
    }

    /// Normalizes the quaternion to have magnitude 1.
    ///
    /// Normalization is important because:
    /// 1. Only unit quaternions (magnitude = 1) represent pure rotations
    /// 2. Prevents scaling effects from accumulating during calculations
    /// 3. Maintains numerical stability in orientation tracking
    ///
    /// The process divides each component by the quaternion's magnitude.
    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        Self {
            w: self.w / m,
            x: self.x / m,
            y: self.y / m,
            z: self.z / m,
        }
    }

    /// Returns the identity quaternion (no rotation).
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

}

