# Core Rendering Pipeline - Layers and Primitives

**Focus:** Low-level rendering pipeline - no UI abstractions  
**Purpose:** Pure rendering: buffers → layers → primitives → pixels  
**Date:** 11 January 2026

---

## Table of Contents

1. [Overview](#overview)
2. [Layer System](#layer-system)
3. [Rendering Primitives](#rendering-primitives)
4. [Strip-Based Rendering](#strip-based-rendering)
5. [Display and Buffers](#display-and-buffers)
6. [Effects and Compositing](#effects-and-compositing)
7. [Complete Examples](#complete-examples)

---

## Overview

**Core Principle**: This is a **pure rendering engine** - no widgets, no events, no UI framework.

### What This Provides

- **DrawTargets**: Any surface with buffer + format (display, offscreen, texture)
- **Layers**: Rendering surfaces with coordinate mapping
- **Primitives**: fill, line, arc, triangle, image, text rendering
- **Strip rendering**: Memory-efficient partial rendering
- **Compositing**: Alpha blending, opacity, effects

### What This Does NOT Provide

- Widget system (buttons, labels, containers)
- Layout engines
- Event handling
- Input processing
- Animation framework

**This is the foundation** - a UI library can be built on top.

### What is a Layer?

```rust
pub struct Layer {
    buffer: *mut [u8],           // Raw pixel data (owned or borrowed)
    width: u32,                   // Buffer width in pixels
    height: u32,                  // Buffer height in pixels
    buf_area: Rect,               // Which screen region buffer represents
    clip_area: Rect,              // Clipping region
    partial_y_offset: i32,        // Y offset for partial/strip rendering
    opacity: Opacity,             // Layer-wide transparency (0-255)
    color_format: ColorFormat,    // RGB565, ARGB8888, etc
}
```

**Key concept: buf_area**

This tells rendering code "the buffer represents THIS region of the screen":

```rust
// Full-screen rendering
layer.buf_area = Rect::new(0, 0, 239, 239);
// buffer[0][0] = screen pixel (0, 0)

// Strip rendering (first 100 rows)
layer.buf_area = Rect::new(0, 0, 239, 99);
// buffer[0][0] = screen pixel (0, 0)
// Only 100 rows of screen in buffer

// Strip rendering (rows 100-199)
layer.buf_area = Rect::new(0, 100, 239, 199);
// buffer[0][0] = screen pixel (0, 100)  ← Buffer reused!
```

**A layer is NOT:**
- ❌ A widget (button, label, etc.)
- ❌ A container
- ❌ A UI element

**A layer IS:**
- ✅ A pixel buffer
- ✅ Metadata for coordinate mapping
- ✅ A rendering target

### The Rendering Pipeline

```
┌─────────────┐
│   Layer     │ ← Pixel buffer + metadata
│   Buffer    │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│  Drawing Primitives                 │
│  - fill()    - line()   - arc()     │
│  - border()  - image()  - text()    │
│  - triangle()          - vector()   │
└──────┬──────────────────────────────┘
       │ (writes pixels)
       ▼
┌─────────────┐
│ Pixel Buffer│ ← Modified with rendered content
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Composite  │ ← Blend multiple layers (optional)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Display   │ ← Flush to physical screen
└─────────────┘
```

---

## Layer System

### Single Layer - Direct Rendering

Simplest case: one buffer, render everything to it.

```rust
// Create display target (240×240 screen)
let mut display = DisplayTarget::<RGB565>::new(240, 240);

// Create layer from display
let mut layer = Layer::from_draw_target(&mut display, Rect::new(0, 0, 240, 240));

// Layer metadata:
// layer.buf_area = Rect { x1: 0, y1: 0, x2: 239, y2: 239 }
// layer.clip_area = Rect { x1: 0, y1: 0, x2: 239, y2: 239 }

// Draw primitives
fill_rect(&mut layer, Rect::new(0, 0, 240, 240), Color::BLACK);  // Background
fill_rect(&mut layer, Rect::new(50, 50, 100, 80), Color::BLUE);   // Rectangle
draw_line(&mut layer, Point::new(0, 0), Point::new(240, 240), Color::WHITE, 2);

// Flush to hardware
display.flush();
```

**Memory**: Full screen buffer (240×240×2 = 115 KB for RGB565)

### Multiple Layers - Compositing

For effects requiring multiple passes:

```
┌─────────────────────┐
│  Main Layer         │  Final display buffer
└──────────┬──────────┘
           │ composite with opacity
           ▼
┌─────────────────────┐
│  Effect Layer       │  Temporary buffer for shadow/blur
└─────────────────────┘
```

**Example: Shadow Effect**

```rust
// 1. Create main layer (display buffer)
let mut main_layer = Layer::from_draw_target(&mut display, Rect::new(0, 0, 240, 240));

// 2. Create temporary shadow layer
let shadow_rect = Rect::new(40, 40, 130, 120);
let mut shadow_buffer = vec![0u16; 130 * 120];
let mut shadow_layer = Layer::new(&mut shadow_buffer, 130, 120, ColorFormat::RGB565);
shadow_layer.buf_area = shadow_rect;

// 3. Render shadow shape
fill_rect(&mut shadow_layer, Rect::new(10, 10, 100, 100), Color::BLACK);

// 4. Apply blur effect
apply_blur(&mut shadow_layer, 10);

// 5. Composite shadow to main with transparency
main_layer.composite(&shadow_layer, Opacity::OPA_50);

// 6. Render main content directly
fill_rect(&mut main_layer, Rect::new(50, 50, 100, 100), Color::WHITE);
draw_border(&mut main_layer, Rect::new(50, 50, 100, 100), Color::GRAY, 1);

// 7. Flush to display
display.flush();
// shadow_layer freed when out of scope
```

**Memory**: Main (115 KB) + Shadow temp (31 KB) = **146 KB peak**

### Layer Compositing

**How layers combine:**

```rust
fn composite(target: &mut Layer, source: &Layer, opacity: Opacity) {
    for y in 0..source.height {
        for x in 0..source.width {
            // Get source pixel
            let src_pixel = source.buffer[y * source.width + x];
            
            // Calculate target position
            let target_x = source.area.x + x;
            let target_y = source.area.y + y;
            
            // Skip if outside target bounds
            if target_x >= target.width || target_y >= target.height {
                continue;
            }
            
            // Get target pixel
            let target_idx = target_y * target.width + target_x;
            let dst_pixel = target.buffer[target_idx];
            
            // Alpha blend
            let blended = blend(dst_pixel, src_pixel, opacity);
            target.buffer[target_idx] = blended;
        }
    }
}

fn blend(dst: Color, src: Color, opacity: u8) -> Color {
    Color {
        r: ((src.r as u16 * opacity as u16 + dst.r as u16 * (255 - opacity) as u16) / 255) as u8,
        g: ((src.g as u16 * opacity as u16 + dst.g as u16 * (255 - opacity) as u16) / 255) as u8,
        b: ((src.b as u16 * opacity as u16 + dst.b as u16 * (255 - opacity) as u16) / 255) as u8,
    }
}
```

### When to Use Multiple Layers

**✅ Need extra layer:**
- Blur effects (can't blur in-place)
- Box shadows (blur + composite)
- Rotations, scaling (need temporary buffer)
- Opacity on group of primitives
- Complex effects requiring pixel sampling

**✅ Single layer sufficient:**
- Simple fills and shapes
- Lines, arcs, triangles  
- Text rendering
- Images without transforms
- Borders and outlines
- Solid gradients

**Principle**: Only create additional layers when you need intermediate buffers for effects.

---

## Strip-Based Rendering

**Problem**: Display is 240×240 (115 KB), but only have 50 KB RAM for buffer.

**Solution**: Render in horizontal strips, reusing the same buffer.

### Strip Rendering Concept

```
Screen (240×240)          Buffer (240×100)
┌────────────┐            
│  Strip 1   │ (y=0-99)   ┌────────────┐
│            │            │ Buffer     │ ← Represents strip 1
├────────────┤            └────────────┘
│  Strip 2   │ (y=100-199)   ↓ Reused
│            │            ┌────────────┐
├────────────┤            │ Buffer     │ ← Represents strip 2
│  Strip 3   │ (y=200-239)└────────────┘
│            │               ↓ Reused
└────────────┘            ┌────────────┐
                          │ Buffer     │ ← Represents strip 3
                          └────────────┘
```

### Example: Rectangle and Triangle

```rust
// Drawing requests (in screen coordinates)
let rect = PrimitiveRequest {
    area: Rect::new(10, 10, 60, 60),     // Top-left
    color: Color::BLUE,
};

let triangle = PrimitiveRequest {
    area: Rect::new(180, 200, 230, 250), // Bottom-right
    color: Color::RED,
};
```

**Strip 1: Rows 0-99**

```rust
// Configure layer
layer.buf_area = Rect::new(0, 0, 239, 99);
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 0;

// Check intersections
if rect.area.intersects(&layer.buf_area) {
    // YES: rect (y=10-60) overlaps strip (y=0-99)
    render_rect(&mut layer, &rect);
    // Maps: screen_y=10 → buffer_y=10-0=10
}

if triangle.area.intersects(&layer.buf_area) {
    // NO: triangle (y=200-250) outside strip (y=0-99)
    // Skip
}

// Flush strip 1 to display
display.flush_strip(&layer.buffer, 0, 100);
```

**Strip 2: Rows 100-199**

```rust
// Reconfigure layer (same buffer!)
layer.buf_area = Rect::new(0, 100, 239, 199);
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 100;

// Check intersections
if rect.area.intersects(&layer.buf_area) {
    // NO: rect (y=10-60) outside strip (y=100-199)
}

if triangle.area.intersects(&layer.buf_area) {
    // NO: triangle (y=200-250) outside strip (y=100-199)
}

// Nothing to render, flush background
display.flush_strip(&layer.buffer, 100, 100);
```

**Strip 3: Rows 200-239**

```rust
// Reconfigure layer
layer.buf_area = Rect::new(0, 200, 239, 239);
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 200;

// Check intersections
if rect.area.intersects(&layer.buf_area) {
    // NO: rect (y=10-60) outside strip (y=200-239)
}

if triangle.area.intersects(&layer.buf_area) {
    // YES: triangle (y=200-250) overlaps strip (y=200-239)
    render_triangle(&mut layer, &triangle);
    // Maps: screen_y=200 → buffer_y=200-200=0 (top of buffer)
    //       screen_y=239 → buffer_y=239-200=39
    //       screen_y=240-250 clipped (outside strip)
}

// Flush strip 3 to display
display.flush_strip(&layer.buffer, 200, 40);
```

### Coordinate Mapping

**Core formula:**
```rust
buffer_y = screen_y - layer.buf_area.y1
```

**Example:**
```rust
// Strip 1: buf_area.y1 = 0
screen_y = 50  → buffer_y = 50 - 0 = 50

// Strip 2: buf_area.y1 = 100  
screen_y = 150 → buffer_y = 150 - 100 = 50

// Strip 3: buf_area.y1 = 200
screen_y = 220 → buffer_y = 220 - 200 = 20
```

### Complete Strip Rendering Loop

```rust
fn render_with_strips(
    primitives: &[PrimitiveRequest],
    display: &mut impl DrawTarget,
    screen_height: u32,
    strip_height: u32,
) {
    let mut buffer = vec![0u8; (240 * strip_height * 2) as usize];
    let mut layer = Layer::new(&mut buffer, 240, strip_height, ColorFormat::RGB565);
    
    let num_strips = (screen_height + strip_height - 1) / strip_height;
    
    for strip in 0..num_strips {
        let y_start = strip * strip_height;
        let y_end = ((strip + 1) * strip_height - 1).min(screen_height - 1);
        
        // Configure layer for this strip
        layer.buf_area = Rect::new(0, y_start, 239, y_end);
        layer.clip_area = layer.buf_area;
        layer.partial_y_offset = y_start;
        
        // Clear to background
        clear_buffer(&mut layer, Color::BLACK);
        
        // Render primitives that intersect
        for prim in primitives {
            if prim.area.intersects(&layer.buf_area) {
                render_primitive(&mut layer, prim);
            }
        }
        
        // Flush to display
        display.flush_strip(&layer.buffer, y_start, y_end - y_start + 1);
    }
}
```

**Memory savings:**
- Full screen: 240×240×2 = 115 KB
- Strip buffer: 240×100×2 = 48 KB
- **Savings: 58% less RAM**

**Trade-off:**
- More CPU (3 passes instead of 1)
- More flush calls
- Worth it on memory-constrained systems

See [strip_based_rendering.md](strip_based_rendering.md) for detailed walkthrough.

---

## Rendering Primitives

All primitives write directly to the layer's pixel buffer.

### Fill (Solid Rectangle)

**Direct pixel writes:**

```rust
fn draw_fill(layer: &mut Layer, rect: Rect, color: Color, opacity: Opacity) {
    for y in rect.y..(rect.y + rect.height) {
        for x in rect.x..(rect.x + rect.width) {
            let pixel_idx = (y * layer.width + x) as usize;
            let dst = layer.buffer[pixel_idx];
            layer.buffer[pixel_idx] = blend(dst, color, opacity.value());
        }
    }
}
```

**API usage:**

```rust
layer.fill(Rect::new(10, 10, 100, 50))
    .color(Color::RED)
    .opacity(Opacity::COVER)
    .draw();
```

### Gradient Fill

**Linear gradient with color interpolation:**

```rust
fn draw_gradient(layer: &mut Layer, rect: Rect, gradient: &Gradient) {
    for y in rect.y..(rect.y + rect.height) {
        // Calculate gradient position (0-255)
        let pos = ((y - rect.y) * 255 / rect.height) as u8;
        
        // Interpolate color from gradient stops
        let color = gradient.color_at(pos);
        
        // Draw horizontal line
        for x in rect.x..(rect.x + rect.width) {
            let pixel_idx = (y * layer.width + x) as usize;
            layer.buffer[pixel_idx] = color.to_pixel(layer.color_format);
        }
    }
}
```

**API usage:**

```rust
let gradient = Gradient::new()
    .vertical()
    .add_stop(Color::rgb(255, 100, 100), 0)
    .add_stop(Color::rgb(100, 100, 255), 255);

layer.fill(Rect::new(30, 30, 170, 120))
    .gradient(gradient)
    .draw();
```

### Border

**Drawing rectangle outlines:**

```rust
fn draw_border(layer: &mut Layer, rect: Rect, color: Color, width: i32, sides: BorderSide) {
    if sides.contains(BorderSide::TOP) {
        layer.draw_horizontal_line(rect.y, rect.x, rect.x + rect.width, color, width);
    }
    if sides.contains(BorderSide::BOTTOM) {
        layer.draw_horizontal_line(rect.y + rect.height, rect.x, rect.x + rect.width, color, width);
    }
    if sides.contains(BorderSide::LEFT) {
        layer.draw_vertical_line(rect.x, rect.y, rect.y + rect.height, color, width);
    }
    if sides.contains(BorderSide::RIGHT) {
        layer.draw_vertical_line(rect.x + rect.width, rect.y, rect.y + rect.height, color, width);
    }
}
```

**API usage:**

```rust
layer.border(Rect::new(20, 20, 200, 100))
    .color(Color::BLUE)
    .width(3)
    .draw();
```

### Line

**Bresenham's algorithm for straight lines:**

```rust
fn draw_line(layer: &mut Layer, p1: PointF, p2: PointF, color: Color, width: i32) {
    let dx = (p2.x - p1.x).abs();
    let dy = (p2.y - p1.y).abs();
    let sx = if p1.x < p2.x { 1 } else { -1 };
    let sy = if p1.y < p2.y { 1 } else { -1 };
    let mut err = dx - dy;
    
    let mut x = p1.x as i32;
    let mut y = p1.y as i32;
    
    loop {
        // Draw pixel with thickness
        for dy in -(width/2)..=(width/2) {
            for dx in -(width/2)..=(width/2) {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && px < layer.width as i32 && py >= 0 && py < layer.height as i32 {
                    let idx = (py * layer.width as i32 + px) as usize;
                    layer.buffer[idx] = color.to_pixel(layer.color_format);
                }
            }
        }
        
        if x == p2.x as i32 && y == p2.y as i32 { break; }
        
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x += sx; }
        if e2 < dx { err += dx; y += sy; }
    }
}
```

**API usage:**

```rust
layer.line(PointF::new(10.0, 50.0), PointF::new(290.0, 50.0))
    .color(Color::GREEN)
    .width(3)
    .draw();
```

### Arc/Circle

**Midpoint circle algorithm:**

```rust
fn draw_arc(layer: &mut Layer, center: Point, radius: u16, start_angle: f32, end_angle: f32, color: Color, width: i32) {
    let start_rad = start_angle.to_radians();
    let end_rad = end_angle.to_radians();
    
    // Sample points along arc
    let steps = (radius as f32 * (end_rad - start_rad).abs()) as i32;
    for i in 0..=steps {
        let angle = start_rad + (end_rad - start_rad) * (i as f32 / steps as f32);
        let x = center.x + (radius as f32 * angle.cos()) as i32;
        let y = center.y + (radius as f32 * angle.sin()) as i32;
        
        // Draw thick point
        for dy in -(width/2)..=(width/2) {
            for dx in -(width/2)..=(width/2) {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && px < layer.width as i32 && py >= 0 && py < layer.height as i32 {
                    let idx = (py * layer.width as i32 + px) as usize;
                    layer.buffer[idx] = color.to_pixel(layer.color_format);
                }
            }
        }
    }
}
```

**API usage:**

```rust
layer.arc(Point::new(150, 150), 80)
    .angles(0.0, 270.0)
    .color(Color::BLUE)
    .width(10)
    .draw();
```

### Triangle

**Rasterization using barycentric coordinates:**

```rust
fn draw_triangle(layer: &mut Layer, p1: PointF, p2: PointF, p3: PointF, color: Color) {
    // Find bounding box
    let min_x = p1.x.min(p2.x).min(p3.x).floor() as i32;
    let max_x = p1.x.max(p2.x).max(p3.x).ceil() as i32;
    let min_y = p1.y.min(p2.y).min(p3.y).floor() as i32;
    let max_y = p1.y.max(p2.y).max(p3.y).ceil() as i32;
    
    // Rasterize
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = PointF::new(x as f32, y as f32);
            
            // Barycentric test
            if point_in_triangle(p, p1, p2, p3) {
                if x >= 0 && x < layer.width as i32 && y >= 0 && y < layer.height as i32 {
                    let idx = (y * layer.width as i32 + x) as usize;
                    layer.buffer[idx] = color.to_pixel(layer.color_format);
                }
            }
        }
    }
}
```

**API usage:**

```rust
layer.triangle(
    PointF::new(150.0, 50.0),
    PointF::new(250.0, 200.0),
    PointF::new(50.0, 200.0)
)
.color(Color::YELLOW)
.draw();
```

### Image

**Blit with alpha blending:**

```rust
fn draw_image(layer: &mut Layer, dest_rect: Rect, image_src: &ImageSource, opacity: Opacity) {
    let image = decode_image(image_src);
    
    for y in 0..image.height.min(dest_rect.height) {
        for x in 0..image.width.min(dest_rect.width) {
            let src_pixel = image.data[y * image.width + x];
            let dest_x = dest_rect.x + x;
            let dest_y = dest_rect.y + y;
            
            if dest_x < layer.width && dest_y < layer.height {
                let dest_idx = (dest_y * layer.width + dest_x) as usize;
                let dest_pixel = layer.buffer[dest_idx];
                
                // Combine image alpha with layer opacity
                let final_opa = (src_pixel.a as u16 * opacity.value() as u16 / 255) as u8;
                let blended = blend(dest_pixel, src_pixel, final_opa);
                layer.buffer[dest_idx] = blended;
            }
        }
    }
}
```

**API usage:**

```rust
layer.image(Rect::new(50, 50, 200, 200), ImageSource::File("logo.png"))
    .opacity(Opacity::OPA_90)
    .draw();
```

### Text

**Glyph rendering with anti-aliasing:**

```rust
fn draw_text(layer: &mut Layer, rect: Rect, text: &str, font: &Font, color: Color) {
    let mut cursor_x = rect.x;
    let mut cursor_y = rect.y;
    
    for ch in text.chars() {
        let glyph = font.get_glyph(ch);
        
        // Render glyph bitmap
        for y in 0..glyph.height {
            for x in 0..glyph.width {
                let alpha = glyph.bitmap[y * glyph.width + x];
                if alpha > 0 {
                    let px = cursor_x + x;
                    let py = cursor_y + y;
                    
                    if px < layer.width && py < layer.height {
                        let idx = (py * layer.width + px) as usize;
                        let dest_pixel = layer.buffer[idx];
                        let blended = blend(dest_pixel, color, alpha);
                        layer.buffer[idx] = blended;
                    }
                }
            }
        }
        
        cursor_x += glyph.advance;
    }
}
```

**API usage:**

```rust
layer.label(Rect::new(20, 100, 260, 50), "Hello, Rust!")
    .font(&FONT_20)
    .color(Color::BLACK)
    .draw();
```

### Clipping

All drawing operations respect the layer's clip area:

```rust
fn draw_with_clipping(layer: &mut Layer, rect: Rect, color: Color) {
    // Intersect drawing area with clip area
    let visible_rect = rect.intersect(layer.clip_rect);
    
    // Only draw visible portion
    for y in visible_rect.y..(visible_rect.y + visible_rect.height) {
        for x in visible_rect.x..(visible_rect.x + visible_rect.width) {
            let idx = (y * layer.width + x) as usize;
            layer.buffer[idx] = color.to_pixel(layer.color_format);
        }
    }
}
```

---

## Display and Buffers

### Buffer Formats

```rust
pub enum ColorFormat {
    L8,           // 8-bit grayscale
    RGB565,       // 16-bit RGB (5-6-5)
    RGB888,       // 24-bit RGB
    ARGB8888,     // 32-bit with alpha
    XRGB8888,     // 32-bit without alpha
}
```

**Memory requirements:**

```rust
// RGB565: 16 bits per pixel
let buffer_size = width * height * 2;  // 240×240 = 115,200 bytes

// RGB888: 24 bits per pixel
let buffer_size = width * height * 3;  // 240×240 = 172,800 bytes

// ARGB8888: 32 bits per pixel
let buffer_size = width * height * 4;  // 240×240 = 230,400 bytes
```

### Display Flush

Transfer rendered pixels to physical display:

```rust
fn flush_to_display(display: &mut Display, layer: &Layer) {
    // Set display window
    display.set_window(layer.area);
    
    // Write pixels
    display.write_pixels(&layer.buffer);
    
    // Signal complete
    display.flush_ready();
}
```

### Double Buffering

Render to one buffer while displaying another:

```rust
struct DoubleBuffer {
    buffer_a: Vec<u16>,
    buffer_b: Vec<u16>,
    current: bool,  // false = A, true = B
}

impl DoubleBuffer {
    fn render_buffer(&mut self) -> &mut [u16] {
        if self.current {
            &mut self.buffer_a
        } else {
            &mut self.buffer_b
        }
    }
    
    fn display_buffer(&self) -> &[u16] {
        if self.current {
            &self.buffer_b
        } else {
            &self.buffer_a
        }
    }
    
    fn swap(&mut self) {
        self.current = !self.current;
    }
}

// Usage
let mut buffers = DoubleBuffer::new(240, 240);

loop {
    // Render to back buffer
    let mut layer = Layer::new(buffers.render_buffer(), 240, 240);
    render_frame(&mut layer);
    
    // Swap buffers
    buffers.swap();
    
    // Display front buffer
    display.flush(buffers.display_buffer());
}
```

---

## Effects and Compositing

### Blur Effect

Gaussian blur using separable convolution:

```rust
fn apply_blur(layer: &mut Layer, radius: i32) {
    let kernel = generate_gaussian_kernel(radius);
    let temp_buffer = layer.buffer.clone();
    
    // Horizontal pass
    for y in 0..layer.height {
        for x in 0..layer.width {
            let mut sum = Color { r: 0, g: 0, b: 0 };
            let mut weight_sum = 0.0;
            
            for k in -radius..=radius {
                let sample_x = (x as i32 + k).clamp(0, layer.width as i32 - 1) as u32;
                let weight = kernel[(k + radius) as usize];
                let pixel = temp_buffer[(y * layer.width + sample_x) as usize];
                
                sum.r += (pixel.r as f32 * weight) as u8;
                sum.g += (pixel.g as f32 * weight) as u8;
                sum.b += (pixel.b as f32 * weight) as u8;
                weight_sum += weight;
            }
            
            layer.buffer[(y * layer.width + x) as usize] = Color {
                r: (sum.r as f32 / weight_sum) as u8,
                g: (sum.g as f32 / weight_sum) as u8,
                b: (sum.b as f32 / weight_sum) as u8,
            };
        }
    }
    
    // Vertical pass (similar)
    // ...
}
```

### Box Shadow

Shadow with blur and offset:

```rust
fn draw_box_shadow(screen: &mut Layer, rect: Rect, shadow_params: ShadowParams) {
    // Create temporary layer for shadow
    let expanded = rect.expand(shadow_params.spread + shadow_params.blur_radius);
    let mut shadow_layer = Layer::new_for_area(expanded);
    
    // Draw shadow shape
    shadow_layer.fill(rect.translate(shadow_params.offset_x, shadow_params.offset_y))
        .color(shadow_params.color)
        .draw();
    
    // Apply blur
    shadow_layer.apply_blur(shadow_params.blur_radius);
    
    // Composite to screen
    screen.composite(&shadow_layer, shadow_params.opacity);
}
```

### Transform

Rotation using matrix transformation:

```rust
fn draw_rotated(screen: &mut Layer, rect: Rect, angle: f32) {
    // Create temp layer
    let mut temp_layer = Layer::new_for_area(rect);
    
    // Draw content to temp layer
    temp_layer.fill(rect).color(Color::BLUE).draw();
    
    // Apply rotation matrix
    let center = Point::new(rect.width / 2, rect.height / 2);
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    
    for y in 0..rect.height {
        for x in 0..rect.width {
            // Rotate around center
            let dx = x as f32 - center.x as f32;
            let dy = y as f32 - center.y as f32;
            let rx = (dx * cos_a - dy * sin_a + center.x as f32) as i32;
            let ry = (dx * sin_a + dy * cos_a + center.y as f32) as i32;
            
            if rx >= 0 && rx < rect.width && ry >= 0 && ry < rect.height {
                let src_idx = (ry * rect.width + rx) as usize;
                let dst_idx = (y * rect.width + x) as usize;
                temp_layer.buffer[dst_idx] = temp_layer.buffer[src_idx];
            }
        }
    }
    
    // Composite rotated result
    screen.composite(&temp_layer, Opacity::COVER);
}
```

---

## Complete Examples

### Example 1: Simple Graphics Scene

```rust
fn render_scene(layer: &mut Layer) {
    // Clear background
    layer.fill(Rect::new(0, 0, 240, 240))
        .color(Color::BLACK)
        .draw();
    
    // Draw shapes
    layer.fill(Rect::new(50, 50, 100, 80))
        .color(Color::BLUE)
        .draw();
    
    layer.arc(Point::new(180, 90), 30)
        .angles(0.0, 360.0)
        .color(Color::RED)
        .width(5)
        .draw();
    
    layer.triangle(
        PointF::new(120.0, 150.0),
        PointF::new(180.0, 220.0),
        PointF::new(60.0, 220.0)
    )
    .color(Color::GREEN)
    .draw();
    
    layer.label(Rect::new(20, 200, 200, 30), "Scene Example")
        .font(&FONT_16)
        .color(Color::WHITE)
        .draw();
}
```

### Example 2: Progress Bar with Gradient

```rust
fn draw_progress_bar(layer: &mut Layer, rect: Rect, progress: u8) {
    // Background
    layer.fill(rect)
        .color(Color::rgb(50, 50, 50))
        .draw();
    
    // Progress fill
    let fill_width = (rect.width * progress as i32) / 100;
    let progress_rect = Rect::new(rect.x, rect.y, fill_width, rect.height);
    
    let gradient = Gradient::new()
        .horizontal()
        .add_stop(Color::rgb(0, 255, 0), 0)
        .add_stop(Color::rgb(0, 150, 0), 255);
    
    layer.fill(progress_rect)
        .gradient(gradient)
        .draw();
    
    // Border
    layer.border(rect)
        .color(Color::WHITE)
        .width(1)
        .draw();
}
```

### Example 3: Card with Shadow and Content

```rust
fn draw_card(screen: &mut Layer, rect: Rect) {
    // Shadow layer
    let shadow_rect = rect.expand(15);
    let mut shadow_buffer = vec![0u16; shadow_rect.width * shadow_rect.height];
    let mut shadow_layer = Layer::new(&mut shadow_buffer, shadow_rect.width, shadow_rect.height);
    shadow_layer.area = shadow_rect;
    
    shadow_layer.fill(rect.translate(5, 5))
        .color(Color::BLACK)
        .draw();
    
    shadow_layer.apply_blur(10);
    screen.composite(&shadow_layer, Opacity::OPA_40);
    
    // Card background
    screen.fill(rect)
        .color(Color::WHITE)
        .draw();
    
    // Card border
    screen.border(rect)
        .color(Color::rgb(200, 200, 200))
        .width(1)
        .draw();
    
    // Card content
    screen.label(Rect::new(rect.x + 10, rect.y + 10, rect.width - 20, 30), "Card Title")
        .font(&FONT_18)
        .color(Color::BLACK)
        .draw();
    
    screen.line(
        PointF::new(rect.x as f32 + 10.0, rect.y as f32 + 45.0),
        PointF::new(rect.x as f32 + rect.width as f32 - 10.0, rect.y as f32 + 45.0)
    )
    .color(Color::rgb(220, 220, 220))
    .width(1)
    .draw();
}
```

### Example 4: Animation Loop

```rust
fn animation_loop(display: &mut Display) {
    let mut buffer = vec![0u16; 240 * 240];
    let mut angle = 0.0;
    
    loop {
        let mut layer = Layer::new(&mut buffer, 240, 240);
        layer.color_format = ColorFormat::RGB565;
        
        // Clear
        layer.fill(Rect::new(0, 0, 240, 240))
            .color(Color::BLACK)
            .draw();
        
        // Rotating arc
        layer.arc(Point::new(120, 120), 60)
            .angles(angle, angle + 90.0)
            .color(Color::BLUE)
            .width(15)
            .draw();
        
        // Flush to display
        display.flush(&layer.buffer);
        
        // Update animation
        angle += 5.0;
        if angle >= 360.0 {
            angle = 0.0;
        }
        
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
}
```

---

## Summary

### Core Concepts

1. **Layer** = Pixel buffer with metadata (area, clip, opacity, format)
2. **Primitives** = Functions that write pixels to layer buffers
3. **Compositing** = Blending multiple layers with transparency
4. **Effects** = Operations that process pixel data (blur, transform)

### Key Principles

- Layers are **raw rendering surfaces**, not UI widgets
- Most primitives draw **directly to one layer**
- Extra layers needed only for **effects requiring pixel sampling**
- Everything is **pixel manipulation** at the core

### Memory Management

- Screen layer: Permanent display buffer
- Effect layers: Temporary, allocated on-demand
- Typical app: 115-200 KB total for rendering

### Performance

- Direct rendering: Fast, immediate pixel writes
- Effects: Slower, require pixel sampling
- Compositing: Moderate, linear blend operations
- Hardware acceleration can optimize all operations
