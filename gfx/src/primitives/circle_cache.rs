/// LVGL-compatible circle cache for anti-aliased rounded corners
/// This implements LVGL's circ_calc_aa4 algorithm exactly
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

/// Circle cache entry storing pre-computed anti-aliasing data
pub struct CircleCache {
    radius: i32,
    cir_opa: Vec<u8>,         // Opacity values for circle edge points
    opa_start_on_y: Vec<u16>, // Starting index in cir_opa for each y
    x_start_on_y: Vec<u16>,   // Starting x coordinate for each y
}

pub struct CircleLineData {
    pub x_start: i32,
    pub opa: Vec<u8>,
}

impl CircleCache {
    /// Create a new circle cache for the given radius
    /// This matches LVGL's circ_calc_aa4 function
    pub fn new(radius: i32) -> Self {
        if radius == 0 {
            return Self {
                radius: 0,
                cir_opa: Vec::new(),
                opa_start_on_y: Vec::new(),
                x_start_on_y: Vec::new(),
            };
        }

        // Special case for radius 1
        if radius == 1 {
            return Self {
                radius: 1,
                cir_opa: vec![180],
                opa_start_on_y: vec![0, 1],
                x_start_on_y: vec![0],
            };
        }

        // Calculate circle using Bresenham algorithm with 4x upscaling
        let mut cir_x = vec![0i32; (radius as usize + 1) * 2];
        let mut cir_y = vec![0i32; (radius as usize + 1) * 2];
        let mut cir_opa_temp = vec![0u8; (radius as usize + 1) * 2];

        let mut y_8th_cnt = 0;
        let mut cp_x = radius * 4; // Upscale by 4
        let mut cp_y = 0;
        let mut tmp = 1 - radius * 4;
        let mut cir_size = 0;

        let mut x_int = [0u32; 4];
        let mut x_fract = [0u32; 4];
        x_int[0] = (cp_x >> 2) as u32;
        x_fract[0] = 0;

        // Calculate 1/8 circle
        while cp_y <= cp_x {
            // Calculate 4 points
            let mut i = 0;
            while i < 4 {
                if tmp <= 0 {
                    tmp += 2 * cp_y + 3;
                } else {
                    tmp += 2 * (cp_y - cp_x) + 5;
                    cp_x -= 1;
                }
                cp_y += 1;

                if cp_y > cp_x {
                    break;
                }

                x_int[i] = (cp_x >> 2) as u32;
                x_fract[i] = (cp_x & 0x3) as u32;
                i += 1;
            }

            if i != 4 {
                break;
            }

            // Store points based on how they downscale
            if x_int[0] == x_int[3] {
                // All on same x
                cir_x[cir_size] = x_int[0] as i32;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] =
                    ((x_fract[0] + x_fract[1] + x_fract[2] + x_fract[3]) * 16) as u8;
                cir_size += 1;
            } else if x_int[0] != x_int[1] {
                // Second on new x
                cir_x[cir_size] = x_int[0] as i32;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] = (x_fract[0] * 16) as u8;
                cir_size += 1;

                cir_x[cir_size] = x_int[0] as i32 - 1;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] =
                    ((1 * 4 + x_fract[1] + x_fract[2] + x_fract[3]) * 16) as u8;
                cir_size += 1;
            } else if x_int[0] != x_int[2] {
                // Third on new x
                cir_x[cir_size] = x_int[0] as i32;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] = ((x_fract[0] + x_fract[1]) * 16) as u8;
                cir_size += 1;

                cir_x[cir_size] = x_int[0] as i32 - 1;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] = ((2 * 4 + x_fract[2] + x_fract[3]) * 16) as u8;
                cir_size += 1;
            } else {
                // Fourth on new x
                cir_x[cir_size] = x_int[0] as i32;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] = ((x_fract[0] + x_fract[1] + x_fract[2]) * 16) as u8;
                cir_size += 1;

                cir_x[cir_size] = x_int[0] as i32 - 1;
                cir_y[cir_size] = y_8th_cnt;
                cir_opa_temp[cir_size] = ((3 * 4 + x_fract[3]) * 16) as u8;
                cir_size += 1;
            }

            y_8th_cnt += 1;
        }

        // Handle 45° point specially
        let mid = radius * 723; // radius * sqrt(2)/2 * 1024
        let mid_int = mid >> 10;
        if cir_size == 0 || cir_x[cir_size - 1] != mid_int || cir_y[cir_size - 1] != mid_int {
            let tmp_val = mid - (mid_int << 10);
            let opa = if tmp_val <= 512 {
                let t = tmp_val * tmp_val * 2;
                (t >> 16) as u8
            } else {
                let t = 1024 - tmp_val;
                let t = t * t * 2;
                15 - ((t >> 16) as u8)
            };

            cir_x[cir_size] = mid_int;
            cir_y[cir_size] = mid_int;
            cir_opa_temp[cir_size] = opa * 16;
            cir_size += 1;
        }

        // Mirror to create second octant
        let first_size = cir_size;
        for i in (0..first_size - 1).rev() {
            cir_x[cir_size] = cir_y[i];
            cir_y[cir_size] = cir_x[i];
            cir_opa_temp[cir_size] = cir_opa_temp[i];
            cir_size += 1;
        }

        // Build lookup arrays
        let mut opa_start_on_y = vec![0u16; radius as usize + 1];
        let mut x_start_on_y = vec![0u16; radius as usize + 1];
        let mut cir_opa = vec![0u8; cir_size];

        let mut y = 0;
        let mut i = 0;
        opa_start_on_y[0] = 0;

        while i < cir_size {
            opa_start_on_y[y] = i as u16;
            x_start_on_y[y] = cir_x[i] as u16;

            // Find minimum x for this y while copying opacity values
            while i < cir_size && cir_y[i] == y as i32 {
                x_start_on_y[y] = x_start_on_y[y].min(cir_x[i] as u16);
                cir_opa[i] = cir_opa_temp[i];
                i += 1;
            }
            y += 1;
        }

        // Sentinel entry required for get_line (matches LVGL)
        if y < opa_start_on_y.len() {
            opa_start_on_y[y] = cir_size as u16;
        }

        Self {
            radius,
            cir_opa,
            opa_start_on_y,
            x_start_on_y,
        }
    }

    /// Get opacity line for a given y coordinate
    /// Returns (opacity_array, x_start_position)
    pub fn get_line(&self, y: i32) -> Option<(&[u8], i32)> {
        if y < 0 || y >= self.radius {
            return None;
        }
        let y = y as usize;
        if y + 1 >= self.opa_start_on_y.len() {
            return None;
        }
        let start = self.opa_start_on_y[y] as usize;
        let end = self.opa_start_on_y[y + 1] as usize;
        let x_start = self.x_start_on_y[y] as i32;
        Some((&self.cir_opa[start..end], x_start))
    }

    /// Get circle line data as a struct (for debugging)
    pub fn get_line_data(&self, y: i32) -> Option<CircleLineData> {
        let (opa_slice, x_start) = self.get_line(y)?;
        Some(CircleLineData {
            x_start,
            opa: opa_slice.to_vec(),
        })
    }
    pub fn radius(&self) -> i32 {
        self.radius
    }
}
