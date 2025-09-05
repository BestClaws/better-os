// math.rs

use defmt::Format;
use micromath::F32Ext;

/// A 3D vector with common vector math operations.
///
/// This is intentionally a tuple-struct for compact field access in
/// performance-sensitive code, while keeping method-based operations for clarity.
#[derive(Clone, Copy, Debug, Format)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    /// Length squared (avoids a costly sqrt when only relative length is needed).
    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.0 * self.0 + self.1 * self.1 + self.2 * self.2
    }

    /// Vector length (magnitude).
    #[inline(always)]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize the vector to unit length. Returns a zero vector if magnitude is too small.
    #[inline(always)]
    pub fn normalize(self) -> Self {
        let mag_sq = self.length_squared();
        if mag_sq < 1e-8 {
            Vec3(0.0, 0.0, 0.0)
        } else {
            let inv_mag = 1.0 / mag_sq.sqrt();
            Vec3(self.0 * inv_mag, self.1 * inv_mag, self.2 * inv_mag)
        }
    }

    /// Compute the dot product with another vector.
    #[inline(always)]
    pub fn dot(self, other: Self) -> f32 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2
    }

    /// Compute the cross product with another vector.
    #[inline(always)]
    pub fn cross(self, other: Self) -> Self {
        Vec3(
            self.1 * other.2 - self.2 * other.1,
            self.2 * other.0 - self.0 * other.2,
            self.0 * other.1 - self.1 * other.0,
        )
    }

    /// Add two vectors.
    #[inline(always)]
    pub fn add(self, other: Self) -> Self {
        Vec3(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }

    /// Subtract another vector from this one.
    #[inline(always)]
    pub fn sub(self, other: Self) -> Self {
        Vec3(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    /// Multiply vector by a scalar.
    #[inline(always)]
    pub fn scale(self, factor: f32) -> Self {
        Vec3(self.0 * factor, self.1 * factor, self.2 * factor)
    }
}

/// A quaternion representing rotation in 3D space.
#[derive(Clone, Copy, Debug)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    /// Create a unit quaternion from an axis-angle representation.
    #[inline(always)]
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let (sin_half_angle, cos_half_angle) = (angle_rad * 0.5).sin_cos();
        let norm_axis = axis.normalize();
        Quaternion {
            w: cos_half_angle,
            x: norm_axis.0 * sin_half_angle,
            y: norm_axis.1 * sin_half_angle,
            z: norm_axis.2 * sin_half_angle,
        }
    }

    /// Multiply two quaternions (combining their rotations).
    #[inline(always)]
    pub fn mul(self, other: Quaternion) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Rotate a 3D vector using this quaternion.
    #[inline(always)]
    pub fn rotate_vector(self, v: Vec3) -> Vec3 {
        let q_vec = Quaternion { w: 0.0, x: v.0, y: v.1, z: v.2 };
        let q_conj = Quaternion { w: self.w, x: -self.x, y: -self.y, z: -self.z };
        let result = self.mul(q_vec).mul(q_conj);
        Vec3(result.x, result.y, result.z)
    }
}
