// file: src/rasterizer.rs

pub trait Rasterizer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);
}