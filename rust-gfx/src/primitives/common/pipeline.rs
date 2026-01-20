use crate::types::Area;
use crate::Rasterizer;

/// Minimal helper that provides consistent access patterns to the
/// rasterizer while a primitive renders. Dirty tracking previously
/// handled here has been removed, so the helper now simply exposes the
/// mutable rasterizer handle and keeps the call sites unchanged.
pub struct PrimitivePipeline<'a, R>
where
    R: Rasterizer,
{
    rast: &'a mut R,
}

impl<'a, R> PrimitivePipeline<'a, R>
where
    R: Rasterizer,
{
    /// Begin a new primitive pipeline for the given rasterizer.
    pub fn new(rast: &'a mut R) -> Self {
        Self { rast }
    }

    /// Gain mutable access to the underlying rasterizer.
    pub fn raster_mut(&mut self) -> &mut R {
        self.rast
    }

    /// No-op placeholder for the previous dirty-area accumulation.
    pub fn include(&mut self, area: &Area) {
        let _ = area;
    }

    /// No-op placeholder for optional dirty-area accumulation.
    pub fn include_opt(&mut self, area: Option<Area>) {
        let _ = area;
    }

    /// Finishing now simply drops the helper without side effects.
    pub fn finish(self) {}
}
