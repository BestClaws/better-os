use defmt::debug;
use embassy_time::Instant;

use super::state::UICompositor;
use crate::libs::gfx::color::Rgba8888;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::system::ui::windowing::{WindowHandle, WindowManager};
use crate::util::math::primitives::Rect;

use super::super::blitter::SurfaceBlitter;
use super::super::region::extract_region_buffer;
use super::super::strategy::{determine_update_strategy, UpdateStrategy};

impl UICompositor {
    /// Process pending redraw requests for the focused window and present changes on the display.
    pub async fn process_redraws(&mut self, wm: &mut WindowManager) {
        if self.pending_redraws.is_empty() {
            return;
        }
        if self.transition_in_progress() {
            return;
        }

        let render_start = Instant::now();
        let (width, height, pixel_format) = match self.display_service.as_ref() {
            Some(display) => (display.width(), display.height(), display.pixel_format()),
            None => return,
        };
        let frame_area = width * height;

        let mut dirty_regions = heapless::Vec::<Rect, 8>::new();
        let mut active_window = None;
        if let Some((current, _prev, _next)) = self.current_prev_next() {
            dirty_regions = self.collect_dirty_regions(wm, current);
            if !dirty_regions.is_empty() {
                active_window = Some(current);
            }
        }

        if let Some(handle) = active_window {
            let scratch = self.scratch.acquire(width, height, pixel_format);
            let mut composite_surface = DrawingSurface::new_unattached(width, height, pixel_format);
            composite_surface.attach_buffer(scratch);
            composite_surface.clear(Rgba8888::rgba(0, 0, 0, 255));

            Self::blit_window_regions(wm, &mut composite_surface, handle, &dirty_regions);

            if let Some(display) = self.display_service.as_ref() {
                match determine_update_strategy(frame_area, &dirty_regions) {
                    UpdateStrategy::FullScreen => {
                        display.draw_full(composite_surface.buffer()).await;
                    }
                    UpdateStrategy::Partial(regions) => {
                        for region in regions.iter() {
                            let region_buffer = extract_region_buffer(
                                composite_surface.buffer(),
                                region,
                                width,
                                height,
                                composite_surface.bytes_per_pixel(),
                            );
                            display.draw_region(&region_buffer, *region).await;
                        }
                    }
                }
            }

            self.clear_window_dirty_regions(wm, handle);
        }

        self.pending_redraws.clear();
        debug!(
            "Frame rendered in {} μs",
            render_start.elapsed().as_micros()
        );
    }

    fn collect_dirty_regions(
        &mut self,
        wm: &mut WindowManager,
        handle: WindowHandle,
    ) -> heapless::Vec<Rect, 8> {
        let mut out = heapless::Vec::new();
        let _ = wm.with_surface(handle, |surface| {
            for region in surface.dirty_regions() {
                let _ = out.push(*region);
            }
        });
        out
    }

    fn clear_window_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        let _ = wm.with_surface(handle, |surface| {
            surface.flush();
        });
    }

    fn blit_window_regions(
        wm: &mut WindowManager,
        output_surface: &mut DrawingSurface<'_>,
        source: WindowHandle,
        regions: &[Rect],
    ) {
        if regions.is_empty() {
            return;
        }

        let _ = wm.with_surface(source, |surface| {
            for region in regions {
                SurfaceBlitter::copy_region(output_surface, surface, *region, 0, 0);
            }
        });
    }
}
