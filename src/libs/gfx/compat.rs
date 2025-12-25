/// Color conversion helpers between the better-os color model and embedded-graphics colors.
pub mod color {
    use crate::libs::gfx::color::Rgba8888;
    use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

    /// Extension trait converting an `Rgba8888` into the 24-bit RGB color used by embedded-graphics.
    pub trait IntoEgRgb {
        fn into_rgb888(self) -> Rgb888;
    }

    impl IntoEgRgb for Rgba8888 {
        #[inline]
        fn into_rgb888(self) -> Rgb888 {
            let raw = self.to_u32();
            let r = ((raw >> 24) & 0xFF) as u8;
            let g = ((raw >> 16) & 0xFF) as u8;
            let b = ((raw >> 8) & 0xFF) as u8;
            Rgb888::new(r, g, b)
        }
    }

    /// Extension trait converting an embedded-graphics RGB color into the better-os RGBA model.
    pub trait IntoRgba8888 {
        fn into_rgba8888(self, alpha: u8) -> Rgba8888;
    }

    impl IntoRgba8888 for Rgb888 {
        #[inline]
        fn into_rgba8888(self, alpha: u8) -> Rgba8888 {
            Rgba8888::rgba(self.r(), self.g(), self.b(), alpha)
        }
    }
}

pub mod draw_target {
    use super::color::IntoRgba8888;
    use crate::libs::gfx::color::Rgba8888;
    use crate::libs::gfx::rasterizer::Rasterizer;
    use core::convert::Infallible;
    use embedded_graphics::geometry::{OriginDimensions, Size};
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::prelude::DrawTarget;
    use embedded_graphics::{geometry::Point, Pixel};

    /// Adapter implementing `DrawTarget` on top of the project rasterizer.
    pub struct RasterizerDrawTarget<'a, R: Rasterizer> {
        rasterizer: &'a mut R,
        alpha: u8,
        width: i32,
        height: i32,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    }

    impl<'a, R: Rasterizer> RasterizerDrawTarget<'a, R> {
        pub fn new(rasterizer: &'a mut R, alpha: u8) -> Self {
            let width = rasterizer.width() as i32;
            let height = rasterizer.height() as i32;
            Self {
                rasterizer,
                alpha,
                width,
                height,
                min_x: i32::MAX,
                min_y: i32::MAX,
                max_x: i32::MIN,
                max_y: i32::MIN,
            }
        }

        fn record_dirty(&mut self, x: i32, y: i32) {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        }

        pub fn finish(&mut self) {
            if self.min_x <= self.max_x && self.min_y <= self.max_y {
                let min_x = self.min_x.max(0);
                let min_y = self.min_y.max(0);
                let max_x = self.max_x.min(self.width - 1);
                let max_y = self.max_y.min(self.height - 1);
                if min_x <= max_x && min_y <= max_y {
                    self.rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
                }
            }
        }
    }

    impl<R: Rasterizer> DrawTarget for RasterizerDrawTarget<'_, R> {
        type Color = Rgb888;
        type Error = Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(Point { x, y }, color) in pixels {
                if x < 0 || y < 0 || x >= self.width || y >= self.height {
                    continue;
                }
                let rgba = color.into_rgba8888(self.alpha);
                self.rasterizer.blend_pixel(x, y, rgba, 255);
                self.record_dirty(x, y);
            }
            Ok(())
        }
    }

    impl<R: Rasterizer> OriginDimensions for RasterizerDrawTarget<'_, R> {
        fn size(&self) -> Size {
            Size::new(self.width as u32, self.height as u32)
        }
    }
}

pub mod surface {
    use super::color::IntoRgba8888;
    use crate::system::ui::drawing_surface::surface::DrawingSurface;
    use crate::util::math::primitives::{Point as UiPoint, Rect as UiRect};
    use core::convert::Infallible;
    use embedded_graphics::geometry::{OriginDimensions, Point, Size};
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::prelude::DrawTarget;
    use embedded_graphics::Pixel;

    /// Adapter exposing `DrawingSurface` as an embedded-graphics `DrawTarget`.
    pub struct SurfaceDrawTarget<'a, 'b> {
        surface: &'a mut DrawingSurface<'b>,
        alpha: u8,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    }

    impl<'a, 'b> SurfaceDrawTarget<'a, 'b> {
        pub fn new(surface: &'a mut DrawingSurface<'b>) -> Self {
            Self::with_alpha(surface, 255)
        }

        pub fn with_alpha(surface: &'a mut DrawingSurface<'b>, alpha: u8) -> Self {
            Self {
                surface,
                alpha,
                min_x: i32::MAX,
                min_y: i32::MAX,
                max_x: i32::MIN,
                max_y: i32::MIN,
            }
        }

        fn record_dirty(&mut self, x: i32, y: i32) {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        }

        fn finish(&mut self) {
            if self.min_x <= self.max_x && self.min_y <= self.max_y {
                let min_x = self.min_x.max(0);
                let min_y = self.min_y.max(0);
                let max_x = self.max_x.min(self.surface.width() as i32 - 1);
                let max_y = self.max_y.min(self.surface.height() as i32 - 1);
                if min_x <= max_x && min_y <= max_y {
                    let rect = UiRect::with_corners(
                        UiPoint::new(min_x, min_y),
                        UiPoint::new(max_x, max_y),
                    );
                    self.surface.mark_dirty(rect);
                }
            }
        }
    }

    impl Drop for SurfaceDrawTarget<'_, '_> {
        fn drop(&mut self) {
            self.finish();
        }
    }

    impl DrawTarget for SurfaceDrawTarget<'_, '_> {
        type Color = Rgb888;
        type Error = Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(Point { x, y }, color) in pixels {
                if x < 0
                    || y < 0
                    || x >= self.surface.width() as i32
                    || y >= self.surface.height() as i32
                {
                    continue;
                }
                let rgba = color.into_rgba8888(self.alpha);
                self.surface.set_pixel_internal(x, y, rgba);
                self.record_dirty(x, y);
            }
            Ok(())
        }
    }

    impl OriginDimensions for SurfaceDrawTarget<'_, '_> {
        fn size(&self) -> Size {
            Size::new(self.surface.width(), self.surface.height())
        }
    }
}
