# Rust Rendering API - Core Primitives

**Focus:** Pure rendering API - 2D graphics primitives  
**Purpose:** Low-level drawing operations, foundation for UI library  
**Date:** 11 January 2026

---

## Overview

This is a **pure rendering API** providing low-level 2D graphics operations.

**What this API provides:**
- Core types (Point, Rect, Color, Opacity)
- DrawTarget trait for any rendering surface
- Layer abstraction with coordinate mapping
- Drawing primitives (rectangles, lines, arcs, triangles, text, images)
- Gradient fills
- Alpha blending and compositing

**What this API does NOT provide:**
- Widget system
- Event handling
- Layout management  
- Input processing
- Animation framework

**This is the foundation** - draw pixels, not UI elements. A UI library builds on top.

---

## Table of Contents

1. [Core Types](#core-types)
2. [Color System](#color-system)
3. [Gradients](#gradients)
4. [Rectangles](#rectangles)
5. [Lines](#lines)
6. [Arcs and Circles](#arcs-and-circles)
7. [Triangles](#triangles)
8. [Text](#text)
9. [Vector Graphics](#vector-graphics)
10. [Usage Examples](#usage-examples)

---

## Core Types

### DrawTarget Trait

**DrawTarget** is the display equivalent - anything that provides a buffer, color format, and dimensions.

```rust
/// A surface that can be drawn to (display, offscreen buffer, texture)
pub trait DrawTarget {
    /// Get the color format of this target
    fn color_format(&self) -> ColorFormat;
    
    /// Get width and height
    fn dimensions(&self) -> (u32, u32);
    
    /// Get stride in bytes (may differ from width due to alignment)
    fn stride(&self) -> usize {
        let (width, _) = self.dimensions();
        width as usize * self.color_format().bytes_per_pixel()
    }
    
    /// Mutable access to pixel buffer
    fn buffer_mut(&mut self) -> &mut [u8];
    
    /// Immutable access to pixel buffer
    fn buffer(&self) -> &[u8];
    
    /// Execute blend operation
    fn blend(&mut self, desc: &BlendDescriptor);
    
    /// Clear to color
    fn clear(&mut self, color: Color);
}
```

**Implementations:**
- `DisplayTarget<F: ColorFormatType>` - Physical display
- `OffscreenTarget<F: ColorFormatType>` - Memory buffer
- `TextureTarget<F: ColorFormatType>` - Cached texture

### Drawing Layer

**Layer** is a rendering surface with pixel buffer and coordinate mapping.

```rust
pub struct Layer {
    buffer: *mut [u8],            // Pixel buffer (owned or borrowed)
    width: u32,                    // Buffer width in pixels
    height: u32,                   // Buffer height in pixels
    buf_area: Rect,                // Which screen region buffer represents
    clip_area: Rect,               // Clipping rectangle  
    partial_y_offset: i32,         // Y offset for strip rendering
    opacity: Opacity,              // Layer-wide transparency
    color_format: ColorFormat,     // Pixel format
}

impl Layer {
    /// Create layer from standalone buffer
    pub fn new(buffer: &mut [u8], width: u32, height: u32, format: ColorFormat) -> Self;
    
    /// Create layer from any DrawTarget
    pub fn from_draw_target(target: &mut impl DrawTarget, area: Rect) -> Self;
    
    /// Clear layer to color
    pub fn clear(&mut self, color: Color);
    
    /// Composite another layer into this one
    pub fn composite(&mut self, source: &Layer, dest_area: Rect);
    
    /// Configure for strip rendering
    pub fn set_strip(&mut self, screen_y_start: i32, screen_y_end: i32) {
        self.buf_area = Rect::new(0, screen_y_start, self.width as i32 - 1, screen_y_end);
        self.clip_area = self.buf_area;
        self.partial_y_offset = screen_y_start;
    }
}
```

**Key concept: buf_area**

Tells primitives which screen region the buffer represents:

```rust
// Full screen
layer.buf_area = Rect::new(0, 0, 239, 239);
// buffer[y][x] = screen pixel (x, y)

// Strip rendering (rows 100-199)
layer.buf_area = Rect::new(0, 100, 239, 199);
// buffer[0][x] = screen pixel (x, 100)
// buffer[99][x] = screen pixel (x, 199)
```

### Geometric Types

```rust
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self;
}

impl PointF {
    pub const fn new(x: f32, y: f32) -> Self;
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self;
    pub fn contains(&self, point: Point) -> bool;
    pub fn intersects(&self, other: &Rect) -> bool;
}
```

---

## Color System

### Color Types

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorAlpha {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: u16,  // 0-360
    pub s: u8,   // 0-100
    pub v: u8,   // 0-100
}
```

### Color Construction

```rust
impl Color {
    // Basic constructors
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self;
    pub const fn from_hex(hex: u32) -> Self;
    pub fn from_hsv(h: u16, s: u8, v: u8) -> Self;
    
    // Common colors
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const ORANGE: Color = Color::rgb(255, 165, 0);
    pub const PURPLE: Color = Color::rgb(128, 0, 128);
    
    // Transformations
    pub fn with_alpha(self, alpha: u8) -> ColorAlpha;
    pub fn darken(self, amount: u8) -> Self;
    pub fn lighten(self, amount: u8) -> Self;
    pub fn mix(self, other: Color, ratio: u8) -> Self;
    pub fn to_hsv(self) -> Hsv;
}
```

### Opacity

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Opacity(u8);

impl Opacity {
    pub const TRANSPARENT: Opacity = Opacity(0);
    pub const OPA_0: Opacity = Opacity(0);
    pub const OPA_10: Opacity = Opacity(25);
    pub const OPA_20: Opacity = Opacity(51);
    pub const OPA_30: Opacity = Opacity(76);
    pub const OPA_40: Opacity = Opacity(102);
    pub const OPA_50: Opacity = Opacity(127);
    pub const OPA_60: Opacity = Opacity(153);
    pub const OPA_70: Opacity = Opacity(178);
    pub const OPA_80: Opacity = Opacity(204);
    pub const OPA_90: Opacity = Opacity(229);
    pub const OPA_100: Opacity = Opacity(255);
    pub const OPAQUE: Opacity = Opacity(255);
    pub const COVER: Opacity = Opacity(255);
    
    pub const fn new(value: u8) -> Self;
    pub fn from_percent(percent: u8) -> Self; // 0-100
    pub fn value(&self) -> u8;
}

impl From<u8> for Opacity {
    fn from(value: u8) -> Self;
}

impl From<f32> for Opacity {
    fn from(value: f32) -> Self; // 0.0-1.0
}
```

### Blend Modes

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Normal,        // Standard alpha blending
    Additive,      // Add color channels
    Subtractive,   // Subtract foreground from background
    Multiply,      // Multiply colors
    Difference,    // Absolute difference
}
```

---

## Gradients

### Gradient Types

```rust
#[derive(Debug, Clone, Copy)]
pub enum GradientDirection {
    Vertical,    // Top to bottom
    Horizontal,  // Left to right
    Linear,      // Custom angle via start/end points
    Radial,      // From center outward
    Conical,     // Sweeping angle
}

#[derive(Debug, Clone, Copy)]
pub enum GradientSpread {
    Pad,      // Clamp to edge colors
    Repeat,   // Repeat pattern
    Reflect,  // Mirror pattern
}
```

### Gradient Stop

```rust
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    pub color: Color,
    pub opacity: Opacity,
    pub position: u8,  // 0-255 along gradient
}

impl GradientStop {
    pub fn new(color: Color, position: u8) -> Self;
    pub fn with_opacity(mut self, opacity: Opacity) -> Self;
}
```

### Gradient Builder

```rust
pub struct Gradient {
    stops: [GradientStop; 8],
    stop_count: usize,
    direction: GradientDirection,
    spread: GradientSpread,
}

impl Gradient {
    pub fn new() -> Self;
    
    // Direction
    pub fn vertical(mut self) -> Self;
    pub fn horizontal(mut self) -> Self;
    pub fn linear(mut self, start: Point, end: Point) -> Self;
    pub fn radial(mut self, center: Point, radius: i32) -> Self;
    pub fn conical(mut self, center: Point, start_angle: i16, end_angle: i16) -> Self;
    
    // Stops
    pub fn add_stop(mut self, color: Color, position: u8) -> Self;
    pub fn add_stop_with_opacity(mut self, color: Color, position: u8, opacity: Opacity) -> Self;
    
    // Spread
    pub fn spread_pad(mut self) -> Self;
    pub fn spread_repeat(mut self) -> Self;
    pub fn spread_reflect(mut self) -> Self;
    
    // Convenience
    pub fn two_color_vertical(top: Color, bottom: Color) -> Self;
    pub fn two_color_horizontal(left: Color, right: Color) -> Self;
}
```

---

## Rectangles

### Fill (Solid Rectangle)

```rust
pub struct Fill<'a> {
    layer: &'a mut Canvas,
    rect: Rect,
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
    radius: i32,
}

impl<'a> Fill<'a> {
    pub fn new(layer: &'a mut Canvas, rect: Rect) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn gradient(mut self, gradient: Gradient) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn circle(mut self) -> Self;
    
    pub fn draw(self);
}

// Extension on Layer
impl Layer {
    pub fn fill(&mut self, rect: Rect) -> Fill;
}
```

### Border

```rust
#[derive(Debug, Clone, Copy)]
pub struct BorderSides {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl BorderSides {
    pub const NONE: Self = BorderSides { top: false, bottom: false, left: false, right: false };
    pub const ALL: Self = BorderSides { top: true, bottom: true, left: true, right: true };
    pub const TOP: Self = BorderSides { top: true, bottom: false, left: false, right: false };
    pub const BOTTOM: Self = BorderSides { top: false, bottom: true, left: false, right: false };
    pub const LEFT: Self = BorderSides { top: false, bottom: false, left: true, right: false };
    pub const RIGHT: Self = BorderSides { top: false, bottom: false, left: false, right: true };
}

pub struct Border<'a> {
    layer: &'a mut Canvas,
    rect: Rect,
    color: Color,
    width: i32,
    opacity: Opacity,
    radius: i32,
    sides: BorderSides,
}

impl<'a> Border<'a> {
    pub fn new(layer: &'a mut Canvas, rect: Rect) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn sides(mut self, sides: BorderSides) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn border(&mut self, rect: Rect) -> Border;
}
```

### Shadow

```rust
pub struct Shadow<'a> {
    layer: &'a mut Canvas,
    rect: Rect,
    color: Color,
    blur_radius: i32,
    spread: i32,
    offset: Point,
    opacity: Opacity,
}

impl<'a> Shadow<'a> {
    pub fn new(layer: &'a mut Canvas, rect: Rect) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn blur_radius(mut self, radius: i32) -> Self;
    pub fn spread(mut self, spread: i32) -> Self;
    pub fn offset(mut self, x: i32, y: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn shadow(&mut self, rect: Rect) -> Shadow;
}
```

---

## Lines

```rust
pub struct Line<'a> {
    layer: &'a mut Canvas,
    start: PointF,
    end: PointF,
    color: Color,
    width: i32,
    opacity: Opacity,
    dash_width: i32,
    dash_gap: i32,
    round_caps: bool,
}

impl<'a> Line<'a> {
    pub fn new(layer: &'a mut Canvas, start: PointF, end: PointF) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn dashed(mut self, dash_width: i32, gap: i32) -> Self;
    pub fn round_caps(mut self) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn line(&mut self, start: PointF, end: PointF) -> Line;
}
```

---

## Arcs and Circles

### Arc

```rust
pub struct Arc<'a> {
    layer: &'a mut Canvas,
    center: Point,
    radius: u16,
    start_angle: f32,  // degrees, 0° = 3 o'clock
    end_angle: f32,
    width: i32,
    color: Color,
    opacity: Opacity,
    round_ends: bool,
}

impl<'a> Arc<'a> {
    pub fn new(layer: &'a mut Canvas, center: Point, radius: u16) -> Self;
    
    pub fn angles(mut self, start: f32, end: f32) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn color(mut self, color: Color) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn round_ends(mut self) -> Self;
    
    // Convenience
    pub fn full_circle(mut self) -> Self;  // 0° to 360°
    pub fn progress(mut self, percent: u8) -> Self;  // 0-100%
    
    pub fn draw(self);
}

impl Layer {
    pub fn arc(&mut self, center: Point, radius: u16) -> Arc;
}
```

### Circle (Filled)

```rust
pub struct Circle<'a> {
    layer: &'a mut Canvas,
    center: Point,
    radius: u16,
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
}

impl<'a> Circle<'a> {
    pub fn new(layer: &'a mut Canvas, center: Point, radius: u16) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn gradient(mut self, gradient: Gradient) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn circle(&mut self, center: Point, radius: u16) -> Circle;
}
```

---

## Triangles

```rust
pub struct Triangle<'a> {
    layer: &'a mut Canvas,
    p1: PointF,
    p2: PointF,
    p3: PointF,
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
}

impl<'a> Triangle<'a> {
    pub fn new(layer: &'a mut Canvas, p1: PointF, p2: PointF, p3: PointF) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn gradient(mut self, gradient: Gradient) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn triangle(&mut self, p1: PointF, p2: PointF, p3: PointF) -> Triangle;
}
```

---

## Text

### Font

```rust
pub struct Font {
    // Font implementation details
}

impl Font {
    pub fn measure_text(&self, text: &str) -> (u32, u32);  // (width, height)
    pub fn line_height(&self) -> u32;
}
```

### Text Alignment

```rust
#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum TextDecoration {
    None,
    Underline,
    Strikethrough,
}
```

### Text Drawing

```rust
pub struct Text<'a> {
    layer: &'a mut Canvas,
    rect: Rect,
    text: &'static str,
    font: &'static Font,
    color: Color,
    opacity: Opacity,
    align: TextAlign,
    line_spacing: i32,
    letter_spacing: i32,
    decoration: TextDecoration,
}

impl<'a> Text<'a> {
    pub fn new(layer: &'a mut Canvas, rect: Rect, text: &'static str) -> Self;
    
    pub fn font(mut self, font: &'static Font) -> Self;
    pub fn color(mut self, color: Color) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn align(mut self, align: TextAlign) -> Self;
    pub fn line_spacing(mut self, spacing: i32) -> Self;
    pub fn letter_spacing(mut self, spacing: i32) -> Self;
    pub fn decoration(mut self, decoration: TextDecoration) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn text(&mut self, rect: Rect, text: &'static str) -> Text;
}
```

---

## Vector Graphics

### Path Operations

```rust
#[derive(Debug, Clone, Copy)]
pub enum PathOp {
    MoveTo,
    LineTo,
    QuadraticTo,
    CubicTo,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, Copy)]
pub enum StrokeCap {
    Butt,
    Square,
    Round,
}

#[derive(Debug, Clone, Copy)]
pub enum StrokeJoin {
    Miter,
    Bevel,
    Round,
}
```

### Path Builder

```rust
pub struct Path {
    ops: Vec<PathOp>,
    points: Vec<PointF>,
}

impl Path {
    pub fn new() -> Self;
    
    // Basic operations
    pub fn move_to(mut self, x: f32, y: f32) -> Self;
    pub fn line_to(mut self, x: f32, y: f32) -> Self;
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self;
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self;
    pub fn close(mut self) -> Self;
    
    // Convenience shapes
    pub fn rect(x: f32, y: f32, width: f32, height: f32) -> Self;
    pub fn circle(cx: f32, cy: f32, radius: f32) -> Self;
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self;
    pub fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Self;
    pub fn arc_path(cx: f32, cy: f32, radius: f32, start: f32, end: f32) -> Self;
}
```

### Vector Drawing

```rust
pub struct VectorPath<'a> {
    layer: &'a mut Canvas,
    path: Path,
    
    // Fill
    fill_color: Option<Color>,
    fill_gradient: Option<Gradient>,
    fill_rule: FillRule,
    fill_opacity: Opacity,
    
    // Stroke
    stroke_color: Option<Color>,
    stroke_width: f32,
    stroke_cap: StrokeCap,
    stroke_join: StrokeJoin,
    stroke_opacity: Opacity,
}

impl<'a> VectorPath<'a> {
    pub fn new(layer: &'a mut Canvas, path: Path) -> Self;
    
    // Fill
    pub fn fill(mut self, color: Color) -> Self;
    pub fn fill_gradient(mut self, gradient: Gradient) -> Self;
    pub fn fill_rule(mut self, rule: FillRule) -> Self;
    pub fn fill_opacity(mut self, opacity: Opacity) -> Self;
    
    // Stroke
    pub fn stroke(mut self, color: Color) -> Self;
    pub fn stroke_width(mut self, width: f32) -> Self;
    pub fn stroke_cap(mut self, cap: StrokeCap) -> Self;
    pub fn stroke_join(mut self, join: StrokeJoin) -> Self;
    pub fn stroke_opacity(mut self, opacity: Opacity) -> Self;
    
    pub fn draw(self);
}

impl Layer {
    pub fn vector(&mut self, path: Path) -> VectorPath;
}
```

---

## Usage Examples

### Example 1: Simple Rectangle

```rust
fn draw_rect(layer: &mut Layer) {
    canvas.fill(Rect::new(10, 10, 100, 50))
        .color(Color::RED)
        .opacity(Opacity::OPAQUE)
        .draw();
}
```

### Example 2: Rounded Rectangle with Border

```rust
fn draw_rounded_rect(layer: &mut Layer) {
    let rect = Rect::new(20, 20, 200, 100);
    
    // Background
    canvas.fill(rect)
        .color(Color::WHITE)
        .radius(10)
        .draw();
    
    // Border
    canvas.border(rect)
        .color(Color::BLUE)
        .width(3)
        .radius(10)
        .draw();
}
```

### Example 3: Gradient Fill

```rust
fn draw_gradient(layer: &mut Layer) {
    let gradient = Gradient::new()
        .vertical()
        .add_stop(Color::rgb(255, 100, 100), 0)
        .add_stop(Color::rgb(100, 100, 255), 255);
    
    canvas.fill(Rect::new(30, 30, 200, 150))
        .gradient(gradient)
        .radius(15)
        .draw();
}
```

### Example 4: Card with Shadow

```rust
fn draw_card(layer: &mut Layer) {
    let rect = Rect::new(50, 50, 200, 150);
    
    // Shadow
    canvas.shadow(rect)
        .color(Color::BLACK)
        .blur_radius(10)
        .offset(5, 5)
        .opacity(Opacity::OPA_50)
        .draw();
    
    // Card background
    let gradient = Gradient::two_color_vertical(
        Color::rgb(240, 240, 255),
        Color::rgb(200, 200, 240)
    );
    
    canvas.fill(rect)
        .gradient(gradient)
        .radius(12)
        .draw();
    
    // Border
    canvas.border(rect)
        .color(Color::rgb(100, 100, 200))
        .width(2)
        .radius(12)
        .draw();
}
```

### Example 5: Dashed Line

```rust
fn draw_dashed_line(layer: &mut Layer) {
    canvas.line(PointF::new(10.0, 50.0), PointF::new(290.0, 50.0))
        .color(Color::GREEN)
        .width(3)
        .dashed(10, 5)
        .round_caps()
        .draw();
}
```

### Example 6: Progress Arc

```rust
fn draw_progress(layer: &mut Layer, progress: u8) {
    let center = Point::new(150, 150);
    
    // Background arc
    canvas.arc(center, 80)
        .angles(0.0, 360.0)
        .color(Color::GRAY)
        .width(15)
        .opacity(Opacity::OPA_30)
        .draw();
    
    // Progress arc
    let progress_angle = (progress as f32 / 100.0) * 360.0;
    canvas.arc(center, 80)
        .angles(0.0, progress_angle)
        .color(Color::GREEN)
        .width(15)
        .round_ends()
        .draw();
}
```

### Example 7: Triangle with Gradient

```rust
fn draw_triangle(layer: &mut Layer) {
    let gradient = Gradient::new()
        .linear(Point::new(100, 50), Point::new(200, 200))
        .add_stop(Color::RED, 0)
        .add_stop(Color::YELLOW, 127)
        .add_stop(Color::GREEN, 255);
    
    canvas.triangle(
        PointF::new(150.0, 50.0),
        PointF::new(250.0, 200.0),
        PointF::new(50.0, 200.0)
    )
    .gradient(gradient)
    .draw();
}
```

### Example 8: Text with Styling

```rust
fn draw_text(layer: &mut Layer, font: &'static Font) {
    canvas.text(Rect::new(20, 100, 260, 150), "Hello, Rust!")
        .font(font)
        .color(Color::BLACK)
        .align(TextAlign::Center)
        .letter_spacing(2)
        .decoration(TextDecoration::Underline)
        .draw();
}
```

### Example 9: Vector Star

```rust
fn draw_star(layer: &mut Layer) {
    let path = Path::new()
        .move_to(150.0, 50.0)
        .line_to(180.0, 130.0)
        .line_to(260.0, 130.0)
        .line_to(200.0, 180.0)
        .line_to(220.0, 260.0)
        .line_to(150.0, 210.0)
        .line_to(80.0, 260.0)
        .line_to(100.0, 180.0)
        .line_to(40.0, 130.0)
        .line_to(120.0, 130.0)
        .close();
    
    canvas.vector(path)
        .fill(Color::YELLOW)
        .stroke(Color::ORANGE)
        .stroke_width(3.0)
        .draw();
}
```

### Example 10: Radial Gradient Circle

```rust
fn draw_radial_circle(layer: &mut Layer) {
    let center = Point::new(150, 150);
    
    let gradient = Gradient::new()
        .radial(center, 80)
        .add_stop(Color::WHITE, 0)
        .add_stop(Color::BLUE, 127)
        .add_stop(Color::rgb(0, 0, 100), 255);
    
    canvas.circle(center, 80)
        .gradient(gradient)
        .draw();
}
```

### Example 11: Complex Button

```rust
fn draw_button(layer: &mut Layer, rect: Rect, text: &'static str, font: &'static Font, pressed: bool) {
    let color = if pressed {
        Color::rgb(80, 120, 200)
    } else {
        Color::rgb(100, 150, 255)
    };
    
    let gradient = Gradient::new()
        .vertical()
        .add_stop(color.lighten(20), 0)
        .add_stop(color, 255);
    
    // Shadow (only when not pressed)
    if !pressed {
        canvas.shadow(rect)
            .color(Color::BLACK)
            .blur_radius(8)
            .offset(0, 2)
            .opacity(Opacity::OPA_30)
            .draw();
    }
    
    // Background
    canvas.fill(rect)
        .gradient(gradient)
        .radius(8)
        .draw();
    
    // Border
    canvas.border(rect)
        .color(Color::rgb(50, 80, 150))
        .width(2)
        .radius(8)
        .draw();
    
    // Text
    canvas.text(rect, text)
        .font(font)
        .color(Color::WHITE)
        .align(TextAlign::Center)
        .draw();
}
```

### Example 12: Animated Progress Bar

```rust
fn draw_progress_bar(layer: &mut Layer, rect: Rect, progress: u8) {
    // Background
    canvas.fill(rect)
        .color(Color::rgb(220, 220, 220))
        .radius(5)
        .draw();
    
    // Progress fill
    let fill_width = (rect.width as f32 * progress as f32 / 100.0) as u32;
    let progress_rect = Rect::new(rect.x, rect.y, fill_width, rect.height);
    
    let gradient = Gradient::two_color_horizontal(
        Color::GREEN,
        Color::rgb(0, 200, 0)
    );
    
    canvas.fill(progress_rect)
        .gradient(gradient)
        .radius(5)
        .draw();
    
    // Border
    canvas.border(rect)
        .color(Color::BLACK)
        .width(1)
        .radius(5)
        .draw();
}
```

### Example 13: Vector Path Shapes

```rust
fn draw_shapes(layer: &mut Layer) {
    // Rounded rectangle path
    let rounded = Path::rounded_rect(10.0, 10.0, 100.0, 80.0, 15.0);
    canvas.vector(rounded)
        .fill(Color::CYAN)
        .stroke(Color::BLUE)
        .stroke_width(2.0)
        .draw();
    
    // Ellipse path
    let ellipse = Path::ellipse(200.0, 50.0, 60.0, 40.0);
    canvas.vector(ellipse)
        .fill(Color::MAGENTA)
        .stroke(Color::PURPLE)
        .stroke_width(2.0)
        .draw();
    
    // Arc path
    let arc = Path::arc_path(150.0, 200.0, 50.0, 0.0, 270.0);
    canvas.vector(arc)
        .stroke(Color::ORANGE)
        .stroke_width(5.0)
        .stroke_cap(StrokeCap::Round)
        .draw();
}
```

---

## Summary

### Core Primitives

| Primitive | Description | Key Features |
|-----------|-------------|--------------|
| **Rectangle** | Fill, border, shadow | Solid color, gradients, rounded corners |
| **Line** | Straight lines | Dashed, variable width, rounded caps |
| **Arc** | Circular arcs | Angles, width, rounded ends |
| **Circle** | Filled circles | Solid color, gradients |
| **Triangle** | Three-point polygons | Solid color, gradients |
| **Text** | Text rendering | Fonts, alignment, decoration |
| **Vector** | Path-based shapes | Fill, stroke, complex shapes |

### Common Properties

All primitives support:
- **Color** - RGB or HSV
- **Opacity** - 0-255 transparency
- **Gradients** - Linear, radial, conical
- **Blending** - Multiple blend modes

### Drawing Pattern

All operations follow the builder pattern:
```rust
canvas.primitive(params)
    .property1(value1)
    .property2(value2)
    .draw();
```

This provides:
- ✅ Type safety
- ✅ Chainable configuration
- ✅ Clear intent
- ✅ Compile-time validation
- ✅ Zero-cost abstractions

