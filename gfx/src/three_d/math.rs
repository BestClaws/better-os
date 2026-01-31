use core::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use micromath::F32Ext;

const EPSILON: f32 = 1.0e-6;

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub fn saturate(self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl AddAssign for Vec2 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl SubAssign for Vec2 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline(always)]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline(always)]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline(always)]
    pub fn normalize(self) -> Self {
        let mag_sq = self.length_squared();
        if mag_sq < EPSILON {
            Self::default()
        } else {
            let inv = 1.0 / mag_sq.sqrt();
            Self::new(self.x * inv, self.y * inv, self.z * inv)
        }
    }

    #[inline(always)]
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    #[inline(always)]
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        self + (rhs - self).scale(t)
    }
}

impl Add for Vec3 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl SubAssign for Vec3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    #[inline(always)]
    pub const fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { w, x, y, z }
    }

    #[inline(always)]
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    #[inline(always)]
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let (s, c) = (angle_rad * 0.5).sin_cos();
        let n = axis.normalize();
        Self::new(c, n.x * s, n.y * s, n.z * s)
    }

    #[inline(always)]
    pub fn normalize(self) -> Self {
        let mag_sq = self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z;
        if mag_sq < EPSILON {
            Self::identity()
        } else {
            let inv = 1.0 / mag_sq.sqrt();
            Self::new(self.w * inv, self.x * inv, self.y * inv, self.z * inv)
        }
    }

    #[inline(always)]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    #[inline(always)]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Quaternion::new(0.0, v.x, v.y, v.z);
        let qc = Quaternion::new(self.w, -self.x, -self.y, -self.z);
        let res = self.mul(qv).mul(qc);
        Vec3::new(res.x, res.y, res.z)
    }

    #[inline(always)]
    pub fn to_mat4(self) -> Mat4 {
        let q = self.normalize();
        let xx = q.x * q.x;
        let yy = q.y * q.y;
        let zz = q.z * q.z;
        let xy = q.x * q.y;
        let xz = q.x * q.z;
        let yz = q.y * q.z;
        let wx = q.w * q.x;
        let wy = q.w * q.y;
        let wz = q.w * q.z;

        Mat4 {
            m: [
                [1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy), 0.0],
                [2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx), 0.0],
                [2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    #[inline(always)]
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline(always)]
    pub fn from_translation(t: Vec3) -> Self {
        let mut mat = Self::identity();
        mat.m[0][3] = t.x;
        mat.m[1][3] = t.y;
        mat.m[2][3] = t.z;
        mat
    }

    #[inline(always)]
    pub fn from_scale(s: Vec3) -> Self {
        Self {
            m: [
                [s.x, 0.0, 0.0, 0.0],
                [0.0, s.y, 0.0, 0.0],
                [0.0, 0.0, s.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline(always)]
    pub fn from_quaternion(q: Quaternion) -> Self {
        q.to_mat4()
    }

    #[inline(always)]
    pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        let nf = 1.0 / (near - far);
        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, 2.0 * far * near * nf],
                [0.0, 0.0, -1.0, 0.0],
            ],
        }
    }

    #[inline(always)]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Self {
            m: [
                [s.x, s.y, s.z, -s.dot(eye)],
                [u.x, u.y, u.z, -u.dot(eye)],
                [-f.x, -f.y, -f.z, f.dot(eye)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline(always)]
    pub fn mul_mat4(&self, rhs: &Self) -> Self {
        let mut out = [[0.0f32; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                out[r][c] = self.m[r][0] * rhs.m[0][c]
                    + self.m[r][1] * rhs.m[1][c]
                    + self.m[r][2] * rhs.m[2][c]
                    + self.m[r][3] * rhs.m[3][c];
            }
        }
        Self { m: out }
    }

    #[inline(always)]
    pub fn mul_vec4(&self, v: Vec4) -> Vec4 {
        Vec4 {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z + self.m[0][3] * v.w,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z + self.m[1][3] * v.w,
            z: self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z + self.m[2][3] * v.w,
            w: self.m[3][0] * v.x + self.m[3][1] * v.y + self.m[3][2] * v.z + self.m[3][3] * v.w,
        }
    }

    #[inline(always)]
    pub fn transform_point(&self, v: Vec3) -> Vec3 {
        let res = self.mul_vec4(Vec4::new(v.x, v.y, v.z, 1.0));
        if res.w.abs() > EPSILON {
            Vec3::new(res.x / res.w, res.y / res.w, res.z / res.w)
        } else {
            Vec3::new(res.x, res.y, res.z)
        }
    }

    #[inline(always)]
    pub fn transform_direction(&self, v: Vec3) -> Vec3 {
        let res = self.mul_vec4(Vec4::new(v.x, v.y, v.z, 0.0));
        Vec3::new(res.x, res.y, res.z)
    }
}

impl Default for Mat4 {
    #[inline(always)]
    fn default() -> Self {
        Self::identity()
    }
}
