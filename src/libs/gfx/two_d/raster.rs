pub trait Rasterizer {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn set_pixel(&mut self, x: i32, y: i32, color: crate::libs::gfx::two_d::types::Rgb565);
    fn blend_pixel(
        &mut self,
        x: i32,
        y: i32,
        color: crate::libs::gfx::two_d::types::Rgb565,
        alpha: u8,
    );
}
