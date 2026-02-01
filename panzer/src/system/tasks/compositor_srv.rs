use alloc::boxed::Box;
use alloc::vec::Vec;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::services::display::{Display, DisplayService};

use gfx::rasterizer::RasterTarget;
use gfx::colors::Color;

// Swash font library for text rendering
use swash::{FontRef, scale::ScaleContext, scale::Render, scale::Source, GlyphId};
use swash::scale::image::Content;
use alloc::collections::BTreeMap;

/// Embedded pixel font
const FONT_DATA_PIXEL: &[u8] = include_bytes!("../../assets/pixel.ttf");

/// Target cadence for the compositor loop (~60 FPS).
const MIN_FRAME_TIME_MS: u64 = 16;

/// Cached glyph data
struct CachedGlyph {
    /// Pre-rendered glyph image data (alpha mask)
    data: Vec<u8>,
    /// Width of the glyph image
    width: usize,
    /// Height of the glyph image
    height: usize,
    /// Left bearing (offset from cursor)
    left: i32,
    /// Top bearing (offset from baseline)
    top: i32,
    /// Horizontal advance (how much to move cursor)
    advance: i32,
}

/// RGB565 rasterizer that wraps a framebuffer
struct Rgb565Rasterizer<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
}

impl<'a> Rgb565Rasterizer<'a> {
    fn new(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self { buffer, width, height }
    }
}

impl<'a> RasterTarget for Rgb565Rasterizer<'a> {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        let rgb565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let high = (rgb565 >> 8) as u8;
        let low = (rgb565 & 0xFF) as u8;
        
        for x in x_start..x_start.saturating_add(length) {
            if x < self.width && y < self.height {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        let fg_565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let fg_r = (fg_565 >> 11) & 0x1F;
        let fg_g = (fg_565 >> 5) & 0x3F;
        let fg_b = fg_565 & 0x1F;
        
        for (i, &alpha) in coverage.iter().enumerate() {
            let x = x_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.buffer[idx] = (fg_565 >> 8) as u8;
                    self.buffer[idx + 1] = (fg_565 & 0xFF) as u8;
                } else {
                    // Alpha blend
                    let bg_high = self.buffer[idx];
                    let bg_low = self.buffer[idx + 1];
                    let bg_565 = ((bg_high as u16) << 8) | (bg_low as u16);
                    
                    let bg_r = (bg_565 >> 11) & 0x1F;
                    let bg_g = (bg_565 >> 5) & 0x3F;
                    let bg_b = bg_565 & 0x1F;
                    
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    
                    let out_r = ((fg_r * alpha_norm + bg_r * inv_alpha) / 255) & 0x1F;
                    let out_g = ((fg_g * alpha_norm + bg_g * inv_alpha) / 255) & 0x3F;
                    let out_b = ((fg_b * alpha_norm + bg_b * inv_alpha) / 255) & 0x1F;
                    
                    let out_565 = (out_r << 11) | (out_g << 5) | out_b;
                    self.buffer[idx] = (out_565 >> 8) as u8;
                    self.buffer[idx + 1] = (out_565 & 0xFF) as u8;
                }
            }
        }
    }

    fn blend_color_hspan(&mut self, _y: u16, _x_start: u16, _colors: &[Color], _coverage: &[u8]) {
        unimplemented!()
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        let rgb565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let high = (rgb565 >> 8) as u8;
        let low = (rgb565 & 0xFF) as u8;
        
        for y in y_start..y_start.saturating_add(length) {
            if x < self.width && y < self.height {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }
    }

    fn blend_solid_vspan(&mut self, _x: u16, _y_start: u16, _color: Color, _coverage: &[u8]) {
        unimplemented!()
    }

    fn blend_color_vspan(&mut self, _x: u16, _y_start: u16, _colors: &[Color], _coverage: &[u8]) {
        unimplemented!()
    }
}

/// Convert RGB888 to RGB565
fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}

/// Glyph cache for fast text rendering
struct GlyphCache {
    glyphs: BTreeMap<char, CachedGlyph>,
    font_size: f32,
    baseline: i32,
}

impl GlyphCache {
    /// Create a new glyph cache and pre-render common characters
    fn new(font: &FontRef, scale_context: &mut ScaleContext, font_size: f32, chars: &str) -> Self {
        let mut glyphs = BTreeMap::new();
        let mut scaler = scale_context.builder(*font)
            .size(font_size)
            .hint(true)
            .build();
        
        // Get font metrics to calculate baseline
        let metrics = font.metrics(&[]).scale(font_size);
        let baseline = metrics.ascent as i32;
        
        let charmap = font.charmap();
        
        for ch in chars.chars() {
            let glyph_id = charmap.map(ch);
            
            if let Some(img) = Render::new(&[Source::Outline]).render(&mut scaler, glyph_id) {
                if let Content::Mask = img.content {
                    let cached = CachedGlyph {
                        data: img.data,
                        width: img.placement.width as usize,
                        height: img.placement.height as usize,
                        left: img.placement.left,
                        top: img.placement.top,
                        advance: img.placement.width as i32 + 2,
                    };
                    
                    glyphs.insert(ch, cached);
                }
            }
        }
        
        Self { glyphs, font_size, baseline }
    }
    
    /// Draw text using RasterTarget
    fn draw_text<T: RasterTarget>(
        &self,
        target: &mut T,
        text: &str,
        x: i32,
        y: i32,
        color: Color,
    ) {
        let mut cursor_x = x;
        
        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                // Draw glyph with baseline alignment using RasterTarget
                for gy in 0..glyph.height {
                    let py = y - glyph.top + gy as i32;
                    if py < 0 || py >= target.height() as i32 {
                        continue;
                    }
                    
                    let px_start = cursor_x + glyph.left;
                    if px_start >= target.width() as i32 {
                        continue;
                    }
                    
                    // Extract coverage for this row
                    let row_start = gy * glyph.width;
                    let coverage = &glyph.data[row_start..row_start + glyph.width];
                    
                    // Clip to visible range
                    let clip_start = if px_start < 0 { -px_start as usize } else { 0 };
                    let visible_width = (glyph.width - clip_start).min((target.width() as i32 - (px_start + clip_start as i32)) as usize);
                    
                    if visible_width > 0 {
                        let x_coord = (px_start + clip_start as i32) as u16;
                        target.blend_solid_hspan(
                            py as u16,
                            x_coord,
                            color,
                            &coverage[clip_start..clip_start + visible_width]
                        );
                    }
                }
                cursor_x += glyph.advance;
            } else if ch == ' ' {
                cursor_x += (self.font_size * 0.3) as i32;
            }
        }
    }
}

/// Compositor service task - handles UI rendering, animations, and display upkeep.
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!("Starting compositor service");

    // Negotiate display capabilities first (before hardware init)
    let display_facade: Display = DisplayService::new(display).initialize().await;
    let resolution = display_facade.resolution();
    let negotiated_pixel_format = display_facade.pixel_format();

    // Initialize display hardware with negotiated settings
    let mut display_lock = display.lock().await;
    display_lock.init().await;
    display_lock.set_brightness(0xFF).await;
    info!("Display initialized: brightness=100%");
    drop(display_lock);

    let width = resolution.logical.width as usize;
    let height = resolution.logical.height as usize;
    let pixel_count = width * height;
    let mut frame_buffer = alloc::vec![0u8; pixel_count * 2]; // RGB565 = 2 bytes per pixel

    // Initialize swash font rendering
    let font_pixel = FontRef::from_index(FONT_DATA_PIXEL, 0).expect("Failed to load pixel font");
    let mut scale_context = ScaleContext::new();
    info!("Swash font library initialized: Pixel font");

    // Build glyph cache
    let cache_start = Instant::now();
    let cache = GlyphCache::new(
        &font_pixel, 
        &mut scale_context, 
        8.0,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 !?.:,'-Frame\u{00b5}s"
    );
    let cache_time = cache_start.elapsed().as_millis();
    info!("Glyph cache built in {} ms ({} glyphs)", 
        cache_time, cache.glyphs.len());

    // Red background (RGB565 format)
    let bg_color = (0xF8, 0x00); // Pure red: 0xF800
    let mut frame_counter = 0u32;

    info!("Starting font rendering demo");
    
    loop {
        let frame_start = Instant::now();
        let render_start = Instant::now();
        frame_counter += 1;
        
        // Fill with solid red background
        for chunk in frame_buffer.chunks_exact_mut(2) {
            chunk[0] = bg_color.0;
            chunk[1] = bg_color.1;
        }
        
        let render_time = render_start.elapsed().as_micros();
        
        // Create rasterizer for text drawing
        let mut rasterizer = Rgb565Rasterizer::new(&mut frame_buffer, width as u16, height as u16);
        let white = Color::rgba(255, 255, 255, 255);
        
        // Draw text from glyph cache
        let text_start = Instant::now();
        
        cache.draw_text(&mut rasterizer, "THE QUICK BROWN FOX JUMPS OVER", 10, 15 + cache.baseline, white);
        cache.draw_text(&mut rasterizer, "THE LAZY DOG. PACK MY BOX WITH", 10, 30 + cache.baseline, white);
        cache.draw_text(&mut rasterizer, "FIVE DOZEN LIQUOR JUGS. HOW", 10, 45 + cache.baseline, white);
        cache.draw_text(&mut rasterizer, "VEXINGLY QUICK DAFT ZEBRAS JUMP.", 10, 60 + cache.baseline, white);
        
        // Draw dynamic frame counter
        use core::fmt::Write;
        let mut frame_text_buf = heapless::String::<32>::new();
        let _ = write!(frame_text_buf, "Frame: {}", frame_counter);
        cache.draw_text(&mut rasterizer, &frame_text_buf, 10, 80 + cache.baseline, white);
        
        let text_time = text_start.elapsed().as_micros();
        
        info!("Render: {} \u{00b5}s | Text: {} \u{00b5}s", render_time, text_time);
        
        // Draw the frame
        let mut display_lock = display.lock().await;
        display_lock.draw(&frame_buffer).await;
        drop(display_lock);
        
        // Maintain target frame rate
        let frame_time = frame_start.elapsed();
        if frame_time.as_millis() < MIN_FRAME_TIME_MS {
            Timer::after(Duration::from_millis(MIN_FRAME_TIME_MS - frame_time.as_millis())).await;
        }
    }
}
