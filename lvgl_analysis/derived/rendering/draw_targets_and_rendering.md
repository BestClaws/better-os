# Draw Targets and Rendering Flow

**Focus:** Pure rendering engine foundation - Rust traits and draw targets  
**Purpose:** Low-level rendering API for any target, foundation for future UI library  
**Date:** 11 January 2026

---

## Table of Contents

1. [Overview](#overview)
2. [Draw Target Trait](#draw-target-trait)
3. [Color Format System](#color-format-system)
4. [Rendering Flow](#rendering-flow)
5. [Offscreen Rendering](#offscreen-rendering)
6. [Implementation Examples](#implementation-examples)
7. [Integration with Primitives](#integration-with-primitives)

---

## Overview

### Pure Rendering Engine

This is a **low-level rendering API** - no widgets, no event handling, no UI framework.

**What it provides:**
- DrawTarget trait for any rendering surface
- Layer system for compositing and strip-based rendering
- Format-agnostic primitives (fills, lines, images, etc.)
- Memory-efficient partial rendering
- Foundation for a future UI library to build upon

**What it does NOT provide:**
- Widget system
- Layout management
- Event handling
- Input processing

### Architectural Foundation

**DrawTarget** - any surface providing buffer + color format + dimensions:
- Physical display
- Offscreen buffer  
- Texture cache
- Virtual display for testing

**Layer** - rendering surface with metadata:
- Pixel buffer (owned or borrowed from DrawTarget)
- Position in screen space (buf_area)
- Clipping region (clip_area)
- Transform matrix, opacity
- Support for strip-based partial rendering

**Rendering is format-agnostic** - primitives generate blend descriptors, targets handle pixel operations.

### Why Traits?

Rust traits enable:
- **Zero-cost abstraction**: Monomorphization eliminates virtual dispatch overhead
- **Compile-time polymorphism**: Target type known at compile time → optimizer friendly
- **Type safety**: Color format mismatches caught at compile time
- **Extensibility**: Add new target types without modifying core rendering code

### Architecture Overview

```
┌───────────────────────────────────────────────────┐
│  Drawing Primitives (Format-Independent)          │
│                                                   │
│  fill(rect, color) → BlendDescriptor              │
│  line(p1, p2, color) → BlendDescriptor            │
│  arc(center, radius, angles) → BlendDescriptor    │
│                                                   │
│  Output: Abstract blend requests                  │
└────────────────┬──────────────────────────────────┘
                 │
                 ▼
┌───────────────────────────────────────────────────┐
│  DrawTarget Trait                                 │
│                                                   │
│  fn color_format(&self) -> ColorFormat            │
│  fn dimensions(&self) -> (u32, u32)               │
│  fn buffer_mut(&mut self) -> &mut [u8]            │
│  fn blend(&mut self, desc: &BlendDescriptor)      │
└────────────────┬──────────────────────────────────┘
                 │
                 ▼
┌───────────────────────────────────────────────────┐
│  Concrete Implementations                         │
│                                                   │
│  • DisplayTarget<RGB565>                          │
│  • DisplayTarget<ARGB8888>                        │
│  • OffscreenTarget<RGB565>                        │
│  • TextureTarget<ARGB8888>                        │
└───────────────────────────────────────────────────┘
```

**Key Insight:** The primitive layer generates format-independent blend descriptors. The draw target handles format-specific pixel operations. This separation enables:
- All primitives work with all color formats automatically
- Adding a new format = implement blend logic once
- No code duplication across primitives

---

## Draw Target Trait

### Core Trait Definition

```rust
/// A surface that can be drawn to
pub trait DrawTarget {
    /// The pixel format of this target
    fn color_format(&self) -> ColorFormat;
    
    /// Dimensions of the drawable area
    fn dimensions(&self) -> (u32, u32);
    
    /// Stride in bytes (may differ from width for alignment)
    fn stride(&self) -> usize {
        let (width, _) = self.dimensions();
        width as usize * self.color_format().bytes_per_pixel()
    }
    
    /// Mutable access to the raw pixel buffer
    fn buffer_mut(&mut self) -> &mut [u8];
    
    /// Immutable access to the raw pixel buffer
    fn buffer(&self) -> &[u8];
    
    /// Execute a blend operation (fills, images, etc)
    fn blend(&mut self, desc: &BlendDescriptor);
    
    /// Clear the entire target to a color
    fn clear(&mut self, color: Color) {
        let (width, height) = self.dimensions();
        self.blend(&BlendDescriptor::fill(
            Rect::new(0, 0, width, height),
            color,
            Opacity::OPAQUE,
            None, // No mask
        ));
    }
}
```

### Blend Descriptor

Format-independent description of what to render:

```rust
pub struct BlendDescriptor {
    pub kind: BlendKind,
    pub area: Rect,
    pub opacity: Opacity,
    pub mask: Option<&'static [u8]>, // Coverage mask for soft edges
}

pub enum BlendKind {
    /// Fill area with solid color
    Fill {
        color: Color,
    },
    
    /// Blend source image
    Image {
        source: &'static [u8],
        source_format: ColorFormat,
        source_stride: usize,
        blend_mode: BlendMode,
    },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Normal,      // Standard alpha blend
    Additive,    // result = src + dst
    Multiply,    // result = src * dst
    Replace,     // result = src (no blending)
}

impl BlendDescriptor {
    pub fn fill(area: Rect, color: Color, opacity: Opacity, mask: Option<&'static [u8]>) -> Self {
        Self {
            kind: BlendKind::Fill { color },
            area,
            opacity,
            mask,
        }
    }
    
    pub fn image(
        area: Rect,
        source: &'static [u8],
        source_format: ColorFormat,
        source_stride: usize,
        opacity: Opacity,
        mask: Option<&'static [u8]>,
    ) -> Self {
        Self {
            kind: BlendKind::Image {
                source,
                source_format,
                source_stride,
                blend_mode: BlendMode::Normal,
            },
            area,
            opacity,
            mask,
        }
    }
}
```

---

## Color Format System

### Format Definition with Type Safety

```rust
/// Color format trait - compile-time format specification
pub trait ColorFormatType: 'static {
    const FORMAT: ColorFormat;
    const BYTES_PER_PIXEL: usize;
    
    /// Convert abstract Color to format-specific pixel
    fn from_color(color: Color) -> Self;
    
    /// Convert format-specific pixel to abstract Color
    fn to_color(&self) -> Color;
    
    /// Alpha blend two pixels
    fn blend(&self, backdrop: &Self, opacity: Opacity) -> Self;
}

/// Runtime color format enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorFormat {
    RGB565,
    RGB888,
    ARGB8888,
    XRGB8888,
    L8,        // Grayscale
    A8,        // Alpha only
}

impl ColorFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::RGB565 => 2,
            Self::RGB888 => 3,
            Self::ARGB8888 | Self::XRGB8888 => 4,
            Self::L8 | Self::A8 => 1,
        }
    }
}

/// RGB565 format (16-bit, 5:6:5)
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct RGB565(pub u16);

impl ColorFormatType for RGB565 {
    const FORMAT: ColorFormat = ColorFormat::RGB565;
    const BYTES_PER_PIXEL: usize = 2;
    
    fn from_color(color: Color) -> Self {
        let r = (color.r >> 3) as u16;
        let g = (color.g >> 2) as u16;
        let b = (color.b >> 3) as u16;
        RGB565((r << 11) | (g << 5) | b)
    }
    
    fn to_color(&self) -> Color {
        let r = ((self.0 >> 11) & 0x1F) as u8;
        let g = ((self.0 >> 5) & 0x3F) as u8;
        let b = (self.0 & 0x1F) as u8;
        Color {
            r: (r << 3) | (r >> 2), // Scale 5-bit to 8-bit
            g: (g << 2) | (g >> 4), // Scale 6-bit to 8-bit
            b: (b << 3) | (b >> 2), // Scale 5-bit to 8-bit
        }
    }
    
    fn blend(&self, backdrop: &Self, opacity: Opacity) -> Self {
        if opacity.is_opaque() {
            return *self;
        }
        
        let fg = self.to_color();
        let bg = backdrop.to_color();
        let alpha = opacity.0 as u16;
        let inv_alpha = 255 - alpha;
        
        let r = ((fg.r as u16 * alpha) + (bg.r as u16 * inv_alpha)) >> 8;
        let g = ((fg.g as u16 * alpha) + (bg.g as u16 * inv_alpha)) >> 8;
        let b = ((fg.b as u16 * alpha) + (bg.b as u16 * inv_alpha)) >> 8;
        
        Self::from_color(Color { r: r as u8, g: g as u8, b: b as u8 })
    }
}

/// ARGB8888 format (32-bit with alpha)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ARGB8888 {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl ColorFormatType for ARGB8888 {
    const FORMAT: ColorFormat = ColorFormat::ARGB8888;
    const BYTES_PER_PIXEL: usize = 4;
    
    fn from_color(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: 255,
        }
    }
    
    fn to_color(&self) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
    
    fn blend(&self, backdrop: &Self, opacity: Opacity) -> Self {
        if opacity.is_opaque() {
            return *self;
        }
        
        let alpha = opacity.0 as u16;
        let inv_alpha = 255 - alpha;
        
        Self {
            r: (((self.r as u16 * alpha) + (backdrop.r as u16 * inv_alpha)) >> 8) as u8,
            g: (((self.g as u16 * alpha) + (backdrop.g as u16 * inv_alpha)) >> 8) as u8,
            b: (((self.b as u16 * alpha) + (backdrop.b as u16 * inv_alpha)) >> 8) as u8,
            a: 255,
        }
    }
}
```

### Opacity System

```rust
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Opacity(pub u8);

impl Opacity {
    pub const TRANSPARENT: Self = Self(0);
    pub const OPA_0: Self = Self(0);
    pub const OPA_10: Self = Self(25);
    pub const OPA_20: Self = Self(51);
    pub const OPA_30: Self = Self(76);
    pub const OPA_40: Self = Self(102);
    pub const OPA_50: Self = Self(127);
    pub const OPA_60: Self = Self(153);
    pub const OPA_70: Self = Self(178);
    pub const OPA_80: Self = Self(204);
    pub const OPA_90: Self = Self(229);
    pub const OPA_100: Self = Self(255);
    pub const OPAQUE: Self = Self(255);
    pub const COVER: Self = Self(255);
    
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
    
    pub fn from_percent(percent: u8) -> Self {
        Self((percent.min(100) as u16 * 255 / 100) as u8)
    }
    
    pub const fn value(&self) -> u8 {
        self.0
    }
    
    pub const fn is_transparent(self) -> bool {
        self.0 <= 2
    }
    
    pub const fn is_opaque(self) -> bool {
        self.0 >= 253
    }
    
    /// Combine with mask opacity
    pub fn with_mask(self, mask_value: u8) -> Self {
        Self(((self.0 as u16 * mask_value as u16) >> 8) as u8)
    }
}

impl From<u8> for Opacity {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<f32> for Opacity {
    fn from(value: f32) -> Self {
        Self((value.clamp(0.0, 1.0) * 255.0) as u8)
    }
}
```

---

## Layer System

### What is a Layer?

A **Layer** is a rendering surface with its own pixel buffer and metadata. Layers are created from DrawTargets or standalone buffers.

```rust
pub struct Layer {
    buffer: *mut [u8],            // Pixel buffer (owned or borrowed)
    width: u32,
    height: u32,
    buf_area: Rect,               // Which screen region buffer represents
    clip_area: Rect,              // Clipping rectangle
    partial_y_offset: i32,        // Y offset for partial/strip rendering
    opacity: Opacity,             // Layer-wide transparency
    color_format: ColorFormat,    // Pixel format
}
```

**Key differences:**
- **DrawTarget** = trait for anything that can be drawn to (display, offscreen buffer, texture)
- **Layer** = concrete rendering surface with transform/opacity metadata for compositing

### Creating Layers from DrawTargets

```rust
// Full-screen layer from display target
let display = DisplayTarget::<RGB565>::new(240, 240);
let layer = Layer::from_draw_target(&mut display, Rect::new(0, 0, 240, 240));

// Partial/strip rendering layer (100 rows at a time)
let mut strip_buffer = vec![0u8; 240 * 100 * 2];  // Only 100 rows
let mut layer = Layer::new(&mut strip_buffer, 240, 100, ColorFormat::RGB565);

// Configure for first strip (rows 0-99)
layer.buf_area = Rect::new(0, 0, 239, 99);
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 0;

// Later: configure for second strip (rows 100-199)
layer.buf_area = Rect::new(0, 100, 239, 199);
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 100;
```

### Layer Compositing

Layers can be blended into parent layers with opacity:

```rust
// Create effect layer
let mut shadow_layer = Layer::new(&mut shadow_buffer, 100, 100, ColorFormat::ARGB8888);
shadow_layer.opacity = Opacity::OPA_50;

// Render to shadow layer
render_shadow(&mut shadow_layer);

// Composite into main layer
main_layer.composite(&shadow_layer, Rect::new(10, 10, 100, 100));
```

### Strip-Based Rendering

For memory-constrained systems, render in horizontal strips:

```rust
// Screen: 240×240, Buffer: 240×100 (only 100 rows)
let screen_height = 240;
let strip_height = 100;
let mut buffer = vec![0u8; 240 * strip_height * 2];

for strip_index in 0..(screen_height / strip_height) {
    let y_start = strip_index * strip_height;
    let y_end = y_start + strip_height - 1;
    
    // Update layer to represent this strip
    layer.buf_area = Rect::new(0, y_start, 239, y_end);
    layer.clip_area = layer.buf_area;
    layer.partial_y_offset = y_start;
    
    // Render all primitives that intersect this strip
    for primitive in &primitives {
        if primitive.area.intersects(&layer.buf_area) {
            primitive.render(&mut layer);
        }
    }
    
    // Flush strip to display
    display.flush_strip(&buffer, y_start, strip_height);
}
```

**Key principle:** Tasks use screen coordinates, layer.buf_area tells them where buffer maps.

```rust
// Primitive at screen position (10, 120)
let rect_area = Rect::new(10, 120, 60, 160);

// Strip 2: buf_area = {0, 100, 239, 199}
// Coordinate mapping:
buffer_y = screen_y - layer.buf_area.y1
         = 120 - 100 = 20  // Row 20 in buffer
```

See [strip_based_rendering.md](strip_based_rendering.md) for complete details.

---

## Rendering Flow

### Complete Rendering Cycle

```rust
// 1. Setup: Create display target
let mut target = DisplayTarget::<RGB565>::new(240, 240);

loop {
    // 2. Clear to background
    target.clear(Color::BLACK);
    
    // 3. Draw primitives
    draw_ui(&mut target);
    
    // 4. Send buffer to hardware
    display.flush(target.buffer());
}

fn draw_ui(target: &mut impl DrawTarget) {
    // Fill background rectangle
    target.blend(&BlendDescriptor::fill(
        Rect::new(10, 10, 100, 80),
        Color::BLUE,
        Opacity::OPAQUE,
        None,
    ));
    
    // Draw line with semi-transparency
    target.blend(&BlendDescriptor::fill(
        Rect::new(20, 20, 2, 60), // Thin vertical rectangle = line
        Color::WHITE,
        Opacity::new(180),
        None,
    ));
    
    // Blit image with mask
    target.blend(&BlendDescriptor::image(
        Rect::new(50, 50, 32, 32),
        ICON_BUFFER,
        ColorFormat::ARGB8888,
        32 * 4, // stride
        Opacity::OPAQUE,
        Some(ICON_MASK), // Soft edges
    ));
}
```

---

## Offscreen Rendering

### Offscreen Target

For caching, effects, or texture generation:

```rust
pub struct OffscreenTarget<F: ColorFormatType> {
    buffer: Box<[u8]>,
    width: u32,
    height: u32,
    _phantom: PhantomData<F>,
}

impl<F: ColorFormatType> OffscreenTarget<F> {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height) as usize * F::BYTES_PER_PIXEL;
        
        Self {
            buffer: vec![0u8; buffer_size].into_boxed_slice(),
            width,
            height,
            _phantom: PhantomData,
        }
    }
    
    /// Copy offscreen buffer to another target
    pub fn blit_to(&self, dest: &mut impl DrawTarget, dest_rect: Rect, opacity: Opacity) {
        dest.blend(&BlendDescriptor::image(
            dest_rect,
            &self.buffer,
            F::FORMAT,
            self.stride(),
            opacity,
            None,
        ));
    }
}

impl<F: ColorFormatType> DrawTarget for OffscreenTarget<F> {
    fn color_format(&self) -> ColorFormat {
        F::FORMAT
    }
    
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
    
    fn buffer(&self) -> &[u8] {
        &self.buffer
    }
    
    fn blend(&mut self, desc: &BlendDescriptor) {
        blend_generic::<F>(&mut self.buffer, self.width, self.stride(), desc);
    }
}
```

### Use Cases

#### 1. Caching Complex Graphics

```rust
// Render expensive content once
let mut cache = OffscreenTarget::<RGB565>::new(100, 100);
render_complex_scene(&mut cache);

// Reuse multiple times (just blit)
loop {
    target.clear(Color::BLACK);
    
    // Fast: just copy cached pixels
    cache.blit_to(&mut target, Rect::new(10, 10, 100, 100), Opacity::OPAQUE);
    
    target.swap();
    display.flush(target.display_buffer());
}
```

#### 2. Texture Generation

```rust
// Generate gradient texture
let mut gradient_texture = OffscreenTarget::<ARGB8888>::new(256, 1);

for x in 0..256 {
    let t = x as f32 / 255.0;
    let color = Color {
        r: (t * 255.0) as u8,
        g: 0,
        b: ((1.0 - t) * 255.0) as u8,
    };
    
    gradient_texture.blend(&BlendDescriptor::fill(
        Rect::new(x as i32, 0, 1, 1),
        color,
        Opacity::OPAQUE,
        None,
    ));
}

// Use texture for gradient fills
gradient_texture.blit_to(&mut target, Rect::new(0, 0, 240, 240), Opacity::OPAQUE);
```

#### 3. Shadow/Glow Effects

```rust
// Render widget to offscreen buffer
let mut widget_layer = OffscreenTarget::<ARGB8888>::new(100, 100);
draw_widget(&mut widget_layer);

// Draw shadow (offset + darker)
widget_layer.blit_to(
    &mut target,
    Rect::new(15, 15, 100, 100), // Offset by (5, 5)
    Opacity::new(100), // Semi-transparent shadow
);

// Draw widget on top
widget_layer.blit_to(
    &mut target,
    Rect::new(10, 10, 100, 100),
    Opacity::OPAQUE,
);
```

---

## Implementation Examples

### Blend Implementation (Format-Specific)

```rust
/// Generic blend function (monomorphized per format)
fn blend_generic<F: ColorFormatType>(
    buffer: &mut [u8],
    width: u32,
    stride: usize,
    desc: &BlendDescriptor,
) {
    let pixel_size = F::BYTES_PER_PIXEL;
    
    match &desc.kind {
        BlendKind::Fill { color } => {
            let fill_pixel = F::from_color(*color);
            
            // Fast path: opaque fill with no mask
            if desc.opacity.is_opaque() && desc.mask.is_none() {
                fill_fast::<F>(buffer, width, stride, desc.area, fill_pixel);
            } else {
                fill_blended::<F>(buffer, width, stride, desc.area, fill_pixel, desc.opacity, desc.mask);
            }
        }
        
        BlendKind::Image { source, source_format, source_stride, blend_mode } => {
            blit_image::<F>(
                buffer,
                width,
                stride,
                desc.area,
                source,
                *source_format,
                *source_stride,
                desc.opacity,
                desc.mask,
                *blend_mode,
            );
        }
    }
}

/// Fast opaque fill (no alpha, no mask)
fn fill_fast<F: ColorFormatType>(
    buffer: &mut [u8],
    width: u32,
    stride: usize,
    area: Rect,
    fill_pixel: F,
) {
    let pixel_bytes = unsafe {
        core::slice::from_raw_parts(
            &fill_pixel as *const F as *const u8,
            F::BYTES_PER_PIXEL,
        )
    };
    
    for y in area.y..(area.y + area.height as i32) {
        let row_offset = y as usize * stride + area.x as usize * F::BYTES_PER_PIXEL;
        let row = &mut buffer[row_offset..row_offset + area.width as usize * F::BYTES_PER_PIXEL];
        
        // Fill scanline
        for x in 0..area.width as usize {
            let pixel_offset = x * F::BYTES_PER_PIXEL;
            row[pixel_offset..pixel_offset + F::BYTES_PER_PIXEL]
                .copy_from_slice(pixel_bytes);
        }
    }
}

/// Alpha-blended fill (with optional mask)
fn fill_blended<F: ColorFormatType>(
    buffer: &mut [u8],
    width: u32,
    stride: usize,
    area: Rect,
    fill_pixel: F,
    opacity: Opacity,
    mask: Option<&[u8]>,
) {
    for y in area.y..(area.y + area.height as i32) {
        let row_offset = y as usize * stride;
        
        for x in area.x..(area.x + area.width as i32) {
            let pixel_offset = row_offset + x as usize * F::BYTES_PER_PIXEL;
            
            // Get current pixel
            let backdrop = unsafe {
                &*(buffer.as_ptr().add(pixel_offset) as *const F)
            };
            
            // Calculate effective opacity (combine global + mask)
            let effective_opacity = if let Some(mask_buf) = mask {
                let mask_index = (y - area.y) as usize * area.width as usize + (x - area.x) as usize;
                opacity.with_mask(mask_buf[mask_index])
            } else {
                opacity
            };
            
            // Skip fully transparent
            if effective_opacity.is_transparent() {
                continue;
            }
            
            // Blend
            let blended = fill_pixel.blend(backdrop, effective_opacity);
            
            // Write back
            unsafe {
                *(buffer.as_mut_ptr().add(pixel_offset) as *mut F) = blended;
            }
        }
    }
}
```

### Display Integration

```rust
pub struct Display {
    spi: SpiDevice,
    dc_pin: OutputPin,
    cs_pin: OutputPin,
}

impl Display {
    /// Flush buffer to physical display
    pub fn flush(&mut self, buffer: &[u8]) {
        // Set data/command pin to DATA mode
        self.dc_pin.set_high();
        self.cs_pin.set_low();
        
        // DMA transfer to display
        self.spi.write(buffer).unwrap();
        
        self.cs_pin.set_high();
    }
    
    /// Flush partial strip
    pub fn flush_strip(&mut self, buffer: &[u8], y_offset: u32, height: u32) {
        // Set display window
        self.set_window(0, y_offset, 240, y_offset + height);
        
        // Flush just this strip
        self.flush(buffer);
    }
}
```

---

## Integration with Primitives

### Drawing Primitives Use DrawTarget

All primitives work with any `DrawTarget`:

```rust
pub trait Drawable {
    fn draw(&self, target: &mut impl DrawTarget);
}

pub struct Rectangle {
    pub rect: Rect,
    pub color: Color,
    pub opacity: Opacity,
    pub radius: u32, // Corner radius (0 = sharp corners)
}

impl Drawable for Rectangle {
    fn draw(&self, target: &mut impl DrawTarget) {
        if self.radius == 0 {
            // Fast path: no rounded corners
            target.blend(&BlendDescriptor::fill(
                self.rect,
                self.color,
                self.opacity,
                None,
            ));
        } else {
            // Rounded corners: generate mask
            let mask = generate_radius_mask(self.rect, self.radius);
            target.blend(&BlendDescriptor::fill(
                self.rect,
                self.color,
                self.opacity,
                Some(&mask),
            ));
        }
    }
}

pub struct Line {
    pub p1: Point,
    pub p2: Point,
    pub color: Color,
    pub width: u32,
    pub opacity: Opacity,
}

impl Drawable for Line {
    fn draw(&self, target: &mut impl DrawTarget) {
        // Horizontal line optimization
        if self.p1.y == self.p2.y {
            let rect = Rect {
                x: self.p1.x.min(self.p2.x),
                y: self.p1.y - (self.width as i32 / 2),
                width: (self.p2.x - self.p1.x).abs() as u32,
                height: self.width,
            };
            
            target.blend(&BlendDescriptor::fill(
                rect,
                self.color,
                self.opacity,
                None,
            ));
        } else {
            // General case: rasterize line
            // ... (generates mask for anti-aliasing)
        }
    }
}
```

### Usage Example

```rust
fn main() {
    // Create display target (RGB565, 240×240)
    let mut display_target = DisplayTarget::<RGB565>::new(240, 240);
    
    // Create offscreen cache (ARGB8888 for alpha)
    let mut icon_cache = OffscreenTarget::<ARGB8888>::new(48, 48);
    
    // Render icon once
    render_icon(&mut icon_cache);
    
    // Main loop
    let mut display = Display::init();
    
    loop {
        // Clear
        display_target.clear(Color::BLACK);
        
        // Draw UI
        Rectangle {
            rect: Rect::new(10, 10, 220, 220),
            color: Color::DARK_BLUE,
            opacity: Opacity::OPAQUE,
            radius: 12,
        }.draw(&mut display_target);
        
        Line {
            p1: Point::new(30, 30),
            p2: Point::new(210, 210),
            color: Color::WHITE,
            width: 3,
            opacity: Opacity::new(200),
        }.draw(&mut display_target);
        
        // Blit cached icon
        icon_cache.blit_to(
            &mut display_target,
            Rect::new(96, 96, 48, 48),
            Opacity::OPAQUE,
        );
        
        // Swap and flush
        display_target.swap();
        display.flush(display_target.display_buffer());
        
        // Delay for frame rate
        cortex_m::asm::delay(1_000_000);
    }
}
```

---

## Summary

### Key Design Principles

1. **Trait-Based Architecture**: `DrawTarget` trait enables compile-time polymorphism
2. **Format Independence**: Primitives don't know about pixel formats
3. **Zero-Cost Abstractions**: Monomorphization eliminates runtime overhead
4. **Type Safety**: Color format mismatches caught at compile time
5. **Flexibility**: Same primitives work with displays, offscreen buffers, textures

### Integration Points

```rust
// Everything is a DrawTarget
let mut target: Box<dyn DrawTarget> = if offscreen {
    Box::new(OffscreenTarget::<RGB565>::new(240, 240))
} else {
    Box::new(DisplayTarget::<RGB565>::new(240, 240))
};

// All primitives work the same
rect.draw(&mut *target);
line.draw(&mut *target);
arc.draw(&mut *target);

// Format handling is automatic
// No primitive code knows about RGB565 vs ARGB8888
```

This architecture provides the foundation for a high-performance, memory-efficient, and type-safe rendering system in Rust.
