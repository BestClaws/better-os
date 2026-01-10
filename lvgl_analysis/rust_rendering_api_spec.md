# Rust Fluent-Style Rendering API Specification
## Based on LVGL Drawing Primitives

**Version:** 1.0  
**Date:** 10 January 2026  
**Source:** LVGL v9.x Drawing API Analysis

---

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Color System](#color-system)
3. [Opacity and Blending](#opacity-and-blending)
4. [Gradient System](#gradient-system)
5. [Rectangle Primitives](#rectangle-primitives)
6. [Line Primitives](#line-primitives)
7. [Arc Primitives](#arc-primitives)
8. [Triangle Primitives](#triangle-primitives)
9. [Image Primitives](#image-primitives)
10. [Text/Label Primitives](#textlabel-primitives)
11. [Vector Graphics](#vector-graphics)
12. [Effects](#effects)
13. [Complete Usage Examples](#complete-usage-examples)

---

## Core Concepts

### Drawing Layer

The base abstraction for all drawing operations. All primitives are drawn to a layer.

```rust
pub struct Layer<'a> {
    buffer: &'a mut DrawBuffer,
    // Internal state
}

impl<'a> Layer<'a> {
    pub fn new(buffer: &'a mut DrawBuffer) -> Self;
    pub fn clear(&mut self) -> &mut Self;
}
```

### Point and Area Types

```rust
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct PointPrecise {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self;
}

impl PointPrecise {
    pub fn new(x: f32, y: f32) -> Self;
}

impl Area {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self;
    pub fn from_points(p1: Point, p2: Point) -> Self;
    pub fn width(&self) -> i32;
    pub fn height(&self) -> i32;
}
```

---

## Color System

### Color Types

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct Color32 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorHsv {
    pub hue: u16,        // 0-360
    pub saturation: u8,  // 0-100
    pub value: u8,       // 0-100
}

#[derive(Debug, Clone, Copy)]
pub enum ColorFormat {
    L8,              // 8-bit grayscale
    A8,              // 8-bit alpha only
    Rgb565,          // 16-bit RGB
    Rgb888,          // 24-bit RGB
    Argb8888,        // 32-bit ARGB
    Xrgb8888,        // 32-bit RGB (no alpha)
}
```

### Color Builder (Fluent API)

```rust
impl Color {
    // Constructors
    pub fn new(red: u8, green: u8, blue: u8) -> Self;
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self;
    pub fn from_hex(hex: u32) -> Self;
    pub fn from_hsv(h: u16, s: u8, v: u8) -> Self;
    
    // Color constants
    pub const BLACK: Color;
    pub const WHITE: Color;
    pub const RED: Color;
    pub const GREEN: Color;
    pub const BLUE: Color;
    pub const YELLOW: Color;
    pub const CYAN: Color;
    pub const MAGENTA: Color;
    pub const GRAY: Color;
    pub const ORANGE: Color;
    pub const PURPLE: Color;
    
    // Transformations (fluent)
    pub fn with_alpha(self, alpha: u8) -> Color32;
    pub fn darken(self, amount: u8) -> Self;
    pub fn lighten(self, amount: u8) -> Self;
    pub fn mix(self, other: Color, ratio: u8) -> Self;
    pub fn to_hsv(self) -> ColorHsv;
    pub fn to_grayscale(self) -> Self;
}
```

---

## Opacity and Blending

### Opacity (Opa) Type

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
    pub const COVER: Opacity = Opacity(255);
    
    pub fn new(value: u8) -> Self;
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

## Gradient System

### Gradient Types and Builders

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientDirection {
    None,      // No gradient
    Vertical,  // Top to bottom
    Horizontal,// Left to right
    Linear,    // Custom angle
    Radial,    // From center outward
    Conical,   // Sweeping angle
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientExtend {
    Pad,      // Clamp to edge colors
    Repeat,   // Repeat pattern
    Reflect,  // Mirror pattern
}

#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    pub color: Color,
    pub opacity: Opacity,
    pub position: u8,  // 0-255
}

impl GradientStop {
    pub fn new(color: Color, position: u8) -> Self;
    pub fn at(color: Color, position: u8) -> Self;
    pub fn with_opacity(mut self, opacity: Opacity) -> Self;
}

// Main Gradient Descriptor
pub struct Gradient {
    stops: [GradientStop; 8],  // Max stops
    stop_count: u8,
    direction: GradientDirection,
    extend: GradientExtend,
    params: GradientParams,
}

#[derive(Debug, Clone, Copy)]
pub enum GradientParams {
    Linear {
        start: Point,
        end: Point,
    },
    Radial {
        focal: Point,
        focal_extent: Point,
        end: Point,
        end_extent: Point,
    },
    Conical {
        center: Point,
        start_angle: i16,  // 0-3600 (tenths of degree)
        end_angle: i16,
    },
}

// Fluent API for Gradient
impl Gradient {
    pub fn new() -> Self;
    
    // Direction setters
    pub fn vertical(mut self) -> Self;
    pub fn horizontal(mut self) -> Self;
    pub fn linear(mut self, start: Point, end: Point) -> Self;
    pub fn radial(mut self, center: Point, radius: i32) -> Self;
    pub fn radial_advanced(
        mut self,
        focal: Point,
        focal_extent: Point,
        end: Point,
        end_extent: Point
    ) -> Self;
    pub fn conical(mut self, center: Point, start_angle: i16, end_angle: i16) -> Self;
    
    // Stop management
    pub fn add_stop(mut self, color: Color, position: u8) -> Self;
    pub fn add_stop_with_opacity(mut self, color: Color, position: u8, opacity: Opacity) -> Self;
    pub fn stops(mut self, stops: &[GradientStop]) -> Self;
    
    // Extend behavior
    pub fn extend_pad(mut self) -> Self;
    pub fn extend_repeat(mut self) -> Self;
    pub fn extend_reflect(mut self) -> Self;
    
    // Helper macros for positions
    pub const LEFT: u8 = 0;
    pub const RIGHT: u8 = 255;
    pub const TOP: u8 = 0;
    pub const BOTTOM: u8 = 255;
    pub const CENTER: u8 = 127;
}

// Convenient constructors
impl Gradient {
    pub fn two_color_vertical(top: Color, bottom: Color) -> Self;
    pub fn two_color_horizontal(left: Color, right: Color) -> Self;
    pub fn multi_stop(colors: &[(Color, u8)]) -> Self;
}
```

---

## Rectangle Primitives

### Fill (Solid Rectangle)

```rust
pub struct Fill<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
    radius: i32,
}

impl<'a> Fill<'a> {
    // Start drawing a fill
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    // Fluent setters
    pub fn color(mut self, color: Color) -> Self;
    pub fn gradient(mut self, gradient: Gradient) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn circle(mut self) -> Self;  // radius = RADIUS_CIRCLE
    
    // Execute the draw
    pub fn draw(self);
}

// Usage extension on Layer
impl<'a> Layer<'a> {
    pub fn fill(&mut self, area: Area) -> Fill;
}
```

### Border

```rust
#[derive(Debug, Clone, Copy)]
pub enum BorderSide {
    None    = 0x00,
    Bottom  = 0x01,
    Top     = 0x02,
    Left    = 0x04,
    Right   = 0x08,
    Full    = 0x0F,
    Internal = 0x10,
}

pub struct Border<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    color: Color,
    width: i32,
    opacity: Opacity,
    radius: i32,
    side: BorderSide,
}

impl<'a> Border<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    // Fluent setters
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn side(mut self, side: BorderSide) -> Self;
    pub fn sides(mut self, sides: &[BorderSide]) -> Self;
    
    // Convenience
    pub fn top(mut self) -> Self;
    pub fn bottom(mut self) -> Self;
    pub fn left(mut self) -> Self;
    pub fn right(mut self) -> Self;
    pub fn full(mut self) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn border(&mut self, area: Area) -> Border;
}
```

### Box Shadow

```rust
pub struct BoxShadow<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    color: Color,
    width: i32,        // Blur radius
    spread: i32,       // Expand/contract shadow
    offset_x: i32,
    offset_y: i32,
    opacity: Opacity,
    radius: i32,
    bg_cover: bool,
}

impl<'a> BoxShadow<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn spread(mut self, spread: i32) -> Self;
    pub fn offset(mut self, x: i32, y: i32) -> Self;
    pub fn offset_x(mut self, x: i32) -> Self;
    pub fn offset_y(mut self, y: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn bg_cover(mut self, cover: bool) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn box_shadow(&mut self, area: Area) -> BoxShadow;
}
```

### Complete Rectangle (All features combined)

```rust
pub struct Rect<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    
    // Background
    bg_color: Color,
    bg_gradient: Option<Gradient>,
    bg_opacity: Opacity,
    bg_image: Option<ImageSource>,
    
    // Border
    border_color: Color,
    border_width: i32,
    border_opacity: Opacity,
    border_side: BorderSide,
    
    // Shadow
    shadow_color: Color,
    shadow_width: i32,
    shadow_offset_x: i32,
    shadow_offset_y: i32,
    shadow_spread: i32,
    shadow_opacity: Opacity,
    
    // Outline
    outline_color: Color,
    outline_width: i32,
    outline_pad: i32,
    outline_opacity: Opacity,
    
    // Common
    radius: i32,
}

impl<'a> Rect<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    // Background
    pub fn bg_color(mut self, color: Color) -> Self;
    pub fn bg_gradient(mut self, gradient: Gradient) -> Self;
    pub fn bg_opacity(mut self, opacity: Opacity) -> Self;
    pub fn bg_image(mut self, src: ImageSource) -> Self;
    
    // Border
    pub fn border_color(mut self, color: Color) -> Self;
    pub fn border_width(mut self, width: i32) -> Self;
    pub fn border_opacity(mut self, opacity: Opacity) -> Self;
    pub fn border_side(mut self, side: BorderSide) -> Self;
    
    // Shadow
    pub fn shadow_color(mut self, color: Color) -> Self;
    pub fn shadow_width(mut self, width: i32) -> Self;
    pub fn shadow_offset(mut self, x: i32, y: i32) -> Self;
    pub fn shadow_spread(mut self, spread: i32) -> Self;
    pub fn shadow_opacity(mut self, opacity: Opacity) -> Self;
    
    // Outline
    pub fn outline_color(mut self, color: Color) -> Self;
    pub fn outline_width(mut self, width: i32) -> Self;
    pub fn outline_pad(mut self, pad: i32) -> Self;
    pub fn outline_opacity(mut self, opacity: Opacity) -> Self;
    
    // Radius
    pub fn radius(mut self, radius: i32) -> Self;
    pub fn circle(mut self) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn rect(&mut self, area: Area) -> Rect;
}
```

---

## Line Primitives

```rust
pub struct Line<'a> {
    layer: &'a mut Layer<'a>,
    p1: PointPrecise,
    p2: PointPrecise,
    color: Color,
    width: i32,
    opacity: Opacity,
    dash_width: i32,
    dash_gap: i32,
    round_start: bool,
    round_end: bool,
}

impl<'a> Line<'a> {
    pub fn new(layer: &'a mut Layer<'a>, p1: PointPrecise, p2: PointPrecise) -> Self;
    pub fn from_to(layer: &'a mut Layer<'a>, x1: f32, y1: f32, x2: f32, y2: f32) -> Self;
    
    // Fluent setters
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn dashed(mut self, dash_width: i32, gap: i32) -> Self;
    pub fn dash_width(mut self, width: i32) -> Self;
    pub fn dash_gap(mut self, gap: i32) -> Self;
    pub fn round_start(mut self) -> Self;
    pub fn round_end(mut self) -> Self;
    pub fn rounded(mut self) -> Self;  // Both ends
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn line(&mut self, p1: PointPrecise, p2: PointPrecise) -> Line;
    pub fn line_from_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Line;
}
```

---

## Arc Primitives

```rust
pub struct Arc<'a> {
    layer: &'a mut Layer<'a>,
    center: Point,
    radius: u16,
    start_angle: f32,  // degrees, 0 = 3 o'clock
    end_angle: f32,
    color: Color,
    width: i32,
    opacity: Opacity,
    rounded: bool,
    image_src: Option<ImageSource>,
}

impl<'a> Arc<'a> {
    pub fn new(layer: &'a mut Layer<'a>, center: Point, radius: u16) -> Self;
    
    // Fluent setters
    pub fn angles(mut self, start: f32, end: f32) -> Self;
    pub fn start_angle(mut self, angle: f32) -> Self;
    pub fn end_angle(mut self, angle: f32) -> Self;
    pub fn color(mut self, color: Color) -> Self;
    pub fn width(mut self, width: i32) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn rounded(mut self) -> Self;
    pub fn image_src(mut self, src: ImageSource) -> Self;
    
    // Convenience constructors
    pub fn circle_segment(mut self, start: f32, sweep: f32) -> Self;
    pub fn progress_ring(mut self, progress: u8) -> Self;  // 0-100
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn arc(&mut self, center: Point, radius: u16) -> Arc;
}
```

---

## Triangle Primitives

```rust
pub struct Triangle<'a> {
    layer: &'a mut Layer<'a>,
    points: [PointPrecise; 3],
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
}

impl<'a> Triangle<'a> {
    pub fn new(layer: &'a mut Layer<'a>, p1: PointPrecise, p2: PointPrecise, p3: PointPrecise) -> Self;
    
    // Fluent setters
    pub fn color(mut self, color: Color) -> Self;
    pub fn gradient(mut self, gradient: Gradient) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn points(mut self, p1: PointPrecise, p2: PointPrecise, p3: PointPrecise) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn triangle(&mut self, p1: PointPrecise, p2: PointPrecise, p3: PointPrecise) -> Triangle;
}
```

---

## Image Primitives

```rust
#[derive(Debug, Clone)]
pub enum ImageSource {
    Buffer(&'static [u8]),
    File(&'static str),
    Descriptor(&'static ImageDescriptor),
}

pub struct Image<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    src: ImageSource,
    
    // Transformations
    rotation: i32,      // 0.1 degree units
    scale_x: i32,       // 256 = no zoom
    scale_y: i32,
    skew_x: i32,
    skew_y: i32,
    pivot: Point,
    
    // Effects
    recolor: Color,
    recolor_opacity: Opacity,
    opacity: Opacity,
    blend_mode: BlendMode,
    
    // Clipping
    clip_radius: i32,
    
    // Tiling
    tile: bool,
    
    // Masking
    bitmap_mask: Option<&'static ImageDescriptor>,
    
    antialias: bool,
}

impl<'a> Image<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area, src: ImageSource) -> Self;
    
    // Transform
    pub fn rotation(mut self, degrees: f32) -> Self;
    pub fn scale(mut self, scale: f32) -> Self;
    pub fn scale_xy(mut self, scale_x: f32, scale_y: f32) -> Self;
    pub fn skew(mut self, skew_x: f32, skew_y: f32) -> Self;
    pub fn pivot(mut self, pivot: Point) -> Self;
    
    // Effects
    pub fn recolor(mut self, color: Color) -> Self;
    pub fn recolor_opacity(mut self, opacity: Opacity) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    pub fn blend_mode(mut self, mode: BlendMode) -> Self;
    
    // Clipping
    pub fn clip_radius(mut self, radius: i32) -> Self;
    pub fn clip_circle(mut self) -> Self;
    
    // Tiling
    pub fn tile(mut self) -> Self;
    
    // Masking
    pub fn bitmap_mask(mut self, mask: &'static ImageDescriptor) -> Self;
    
    // Quality
    pub fn antialias(mut self, enable: bool) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn image(&mut self, area: Area, src: ImageSource) -> Image;
}
```

---

## Text/Label Primitives

```rust
#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum TextDecoration {
    None          = 0x00,
    Underline     = 0x01,
    Strikethrough = 0x02,
}

#[derive(Debug, Clone, Copy)]
pub enum BaseDirection {
    LeftToRight,
    RightToLeft,
    Auto,
}

pub struct Label<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    text: &'static str,
    font: &'static Font,
    color: Color,
    opacity: Opacity,
    
    // Layout
    align: TextAlign,
    line_space: i32,
    letter_space: i32,
    offset_x: i32,
    offset_y: i32,
    
    // Rotation
    rotation: i32,  // 0.1 degree units
    
    // Selection
    sel_start: u32,
    sel_end: u32,
    sel_color: Color,
    sel_bg_color: Color,
    
    // Text properties
    decoration: TextDecoration,
    bidi_dir: BaseDirection,
    
    // Outline
    outline_opacity: Opacity,
}

impl<'a> Label<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area, text: &'static str) -> Self;
    
    // Basic
    pub fn font(mut self, font: &'static Font) -> Self;
    pub fn color(mut self, color: Color) -> Self;
    pub fn opacity(mut self, opacity: Opacity) -> Self;
    
    // Layout
    pub fn align(mut self, align: TextAlign) -> Self;
    pub fn align_left(mut self) -> Self;
    pub fn align_center(mut self) -> Self;
    pub fn align_right(mut self) -> Self;
    pub fn line_space(mut self, space: i32) -> Self;
    pub fn letter_space(mut self, space: i32) -> Self;
    pub fn offset(mut self, x: i32, y: i32) -> Self;
    
    // Transform
    pub fn rotation(mut self, degrees: f32) -> Self;
    
    // Selection
    pub fn selection(mut self, start: u32, end: u32) -> Self;
    pub fn sel_color(mut self, color: Color) -> Self;
    pub fn sel_bg_color(mut self, color: Color) -> Self;
    
    // Decoration
    pub fn underline(mut self) -> Self;
    pub fn strikethrough(mut self) -> Self;
    pub fn decoration(mut self, decor: TextDecoration) -> Self;
    
    // Direction
    pub fn bidi_dir(mut self, dir: BaseDirection) -> Self;
    pub fn rtl(mut self) -> Self;
    pub fn ltr(mut self) -> Self;
    
    // Outline
    pub fn outline_opacity(mut self, opacity: Opacity) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn label(&mut self, area: Area, text: &'static str) -> Label;
}
```

---

## Vector Graphics

### Path Building

```rust
#[derive(Debug, Clone, Copy)]
pub enum PathOp {
    MoveTo,
    LineTo,
    QuadTo,
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

pub struct VectorPath {
    ops: Vec<PathOp>,
    points: Vec<PointPrecise>,
}

impl VectorPath {
    pub fn new() -> Self;
    
    // Path commands (fluent)
    pub fn move_to(mut self, x: f32, y: f32) -> Self;
    pub fn line_to(mut self, x: f32, y: f32) -> Self;
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self;
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self;
    pub fn close(mut self) -> Self;
    
    // Convenience shapes
    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Self;
    pub fn circle(cx: f32, cy: f32, radius: f32) -> Self;
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self;
    pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Self;
}
```

### Vector Drawing

```rust
#[derive(Debug, Clone, Copy)]
pub enum VectorGradientStyle {
    Linear,
    Radial,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorGradientSpread {
    Pad,
    Repeat,
    Reflect,
}

pub struct VectorDraw<'a> {
    layer: &'a mut Layer<'a>,
    path: VectorPath,
    
    // Fill
    fill_color: Option<Color>,
    fill_gradient: Option<VectorGradient>,
    fill_rule: FillRule,
    fill_opacity: Opacity,
    
    // Stroke
    stroke_color: Option<Color>,
    stroke_width: f32,
    stroke_cap: StrokeCap,
    stroke_join: StrokeJoin,
    stroke_opacity: Opacity,
    stroke_miter_limit: f32,
    
    // Transform
    transform: Option<Matrix>,
    
    // Blend
    blend_mode: VectorBlendMode,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorBlendMode {
    SrcOver,
    SrcIn,
    DstOver,
    DstIn,
    Screen,
    Multiply,
    None,
    Additive,
    Subtractive,
}

impl<'a> VectorDraw<'a> {
    pub fn new(layer: &'a mut Layer<'a>, path: VectorPath) -> Self;
    
    // Fill
    pub fn fill_color(mut self, color: Color) -> Self;
    pub fn fill_gradient(mut self, gradient: VectorGradient) -> Self;
    pub fn fill_rule(mut self, rule: FillRule) -> Self;
    pub fn fill_opacity(mut self, opacity: Opacity) -> Self;
    
    // Stroke
    pub fn stroke_color(mut self, color: Color) -> Self;
    pub fn stroke_width(mut self, width: f32) -> Self;
    pub fn stroke_cap(mut self, cap: StrokeCap) -> Self;
    pub fn stroke_join(mut self, join: StrokeJoin) -> Self;
    pub fn stroke_opacity(mut self, opacity: Opacity) -> Self;
    pub fn stroke_miter_limit(mut self, limit: f32) -> Self;
    
    // Convenience
    pub fn stroked(mut self, color: Color, width: f32) -> Self;
    pub fn filled(mut self, color: Color) -> Self;
    
    // Transform
    pub fn transform(mut self, matrix: Matrix) -> Self;
    pub fn translate(mut self, x: f32, y: f32) -> Self;
    pub fn rotate(mut self, degrees: f32) -> Self;
    pub fn scale(mut self, sx: f32, sy: f32) -> Self;
    
    // Blend
    pub fn blend_mode(mut self, mode: VectorBlendMode) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn vector(&mut self, path: VectorPath) -> VectorDraw;
}
```

---

## Effects

### Blur

```rust
#[derive(Debug, Clone, Copy)]
pub enum BlurQuality {
    Auto,
    Speed,
    Precision,
}

pub struct Blur<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    blur_radius: i32,
    corner_radius: i32,
    quality: BlurQuality,
}

impl<'a> Blur<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    pub fn blur_radius(mut self, radius: i32) -> Self;
    pub fn corner_radius(mut self, radius: i32) -> Self;
    pub fn quality(mut self, quality: BlurQuality) -> Self;
    pub fn fast(mut self) -> Self;
    pub fn precise(mut self) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn blur(&mut self, area: Area) -> Blur;
}
```

### Mask

```rust
pub struct MaskRect<'a> {
    layer: &'a mut Layer<'a>,
    area: Area,
    radius: i32,
}

impl<'a> MaskRect<'a> {
    pub fn new(layer: &'a mut Layer<'a>, area: Area) -> Self;
    
    pub fn radius(mut self, radius: i32) -> Self;
    
    pub fn draw(self);
}

impl<'a> Layer<'a> {
    pub fn mask_rect(&mut self, area: Area) -> MaskRect;
}
```

---

## Complete Usage Examples

### Example 1: Simple Colored Rectangle

```rust
fn draw_simple_rect(layer: &mut Layer) {
    layer.fill(Area::new(10, 10, 100, 100))
        .color(Color::RED)
        .opacity(Opacity::COVER)
        .draw();
}
```

### Example 2: Rounded Rectangle with Border

```rust
fn draw_rounded_rect_with_border(layer: &mut Layer) {
    let area = Area::new(20, 20, 220, 120);
    
    // Fill
    layer.fill(area)
        .color(Color::WHITE)
        .radius(10)
        .draw();
    
    // Border
    layer.border(area)
        .color(Color::BLUE)
        .width(3)
        .radius(10)
        .draw();
}
```

### Example 3: Gradient Rectangle

```rust
fn draw_gradient_rect(layer: &mut Layer) {
    let gradient = Gradient::new()
        .vertical()
        .add_stop(Color::rgb(255, 100, 100), 0)
        .add_stop(Color::rgb(100, 100, 255), 255)
        .extend_pad();
    
    layer.fill(Area::new(30, 30, 200, 150))
        .gradient(gradient)
        .radius(15)
        .draw();
}
```

### Example 4: Complex Rectangle with Shadow

```rust
fn draw_card(layer: &mut Layer) {
    let area = Area::new(50, 50, 250, 200);
    
    // Shadow
    layer.box_shadow(area)
        .color(Color::BLACK)
        .width(10)
        .offset(5, 5)
        .opacity(Opacity::OPA_50)
        .radius(12)
        .draw();
    
    // Background with gradient
    let gradient = Gradient::two_color_vertical(
        Color::rgb(240, 240, 255),
        Color::rgb(200, 200, 240)
    );
    
    layer.fill(area)
        .gradient(gradient)
        .radius(12)
        .draw();
    
    // Border
    layer.border(area)
        .color(Color::rgb(100, 100, 200))
        .width(2)
        .radius(12)
        .draw();
}
```

### Example 5: Dashed Line

```rust
fn draw_dashed_line(layer: &mut Layer) {
    layer.line_from_to(10.0, 50.0, 290.0, 50.0)
        .color(Color::GREEN)
        .width(3)
        .dashed(10, 5)
        .rounded()
        .draw();
}
```

### Example 6: Progress Arc

```rust
fn draw_progress_arc(layer: &mut Layer, progress: u8) {
    let center = Point::new(150, 150);
    
    // Background arc
    layer.arc(center, 80)
        .angles(0.0, 360.0)
        .color(Color::GRAY)
        .width(15)
        .opacity(Opacity::OPA_30)
        .draw();
    
    // Progress arc
    let progress_angle = (progress as f32 / 100.0) * 360.0;
    layer.arc(center, 80)
        .angles(0.0, progress_angle)
        .color(Color::GREEN)
        .width(15)
        .rounded()
        .draw();
}
```

### Example 7: Triangle with Gradient

```rust
fn draw_triangle_with_gradient(layer: &mut Layer) {
    let gradient = Gradient::new()
        .linear(Point::new(100, 50), Point::new(200, 200))
        .add_stop(Color::RED, 0)
        .add_stop(Color::YELLOW, 127)
        .add_stop(Color::GREEN, 255);
    
    layer.triangle(
        PointPrecise::new(150.0, 50.0),
        PointPrecise::new(250.0, 200.0),
        PointPrecise::new(50.0, 200.0)
    )
    .gradient(gradient)
    .opacity(Opacity::COVER)
    .draw();
}
```

### Example 8: Rotated Image

```rust
fn draw_rotated_image(layer: &mut Layer) {
    let src = ImageSource::File("image.png");
    
    layer.image(Area::new(50, 50, 250, 250), src)
        .rotation(45.0)
        .pivot(Point::new(150, 150))
        .opacity(Opacity::OPA_90)
        .antialias(true)
        .draw();
}
```

### Example 9: Styled Text

```rust
fn draw_styled_text(layer: &mut Layer, font: &'static Font) {
    layer.label(Area::new(20, 100, 280, 150), "Hello, Rust!")
        .font(font)
        .color(Color::BLACK)
        .align_center()
        .letter_space(2)
        .underline()
        .draw();
}
```

### Example 10: Vector Path

```rust
fn draw_vector_star(layer: &mut Layer) {
    let path = VectorPath::new()
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
    
    layer.vector(path)
        .filled(Color::YELLOW)
        .stroked(Color::ORANGE, 3.0)
        .draw();
}
```

### Example 11: Radial Gradient Circle

```rust
fn draw_radial_gradient(layer: &mut Layer) {
    let center = Point::new(150, 150);
    
    let gradient = Gradient::new()
        .radial(center, 80)
        .add_stop(Color::WHITE, 0)
        .add_stop(Color::BLUE, 127)
        .add_stop(Color::rgb(0, 0, 100), 255);
    
    layer.fill(Area::new(70, 70, 230, 230))
        .gradient(gradient)
        .circle()
        .draw();
}
```

### Example 12: Blurred Background

```rust
fn draw_blurred_panel(layer: &mut Layer) {
    // Draw background content first
    layer.fill(Area::new(0, 0, 300, 300))
        .color(Color::rgb(200, 100, 100))
        .draw();
    
    // Apply blur to a region
    layer.blur(Area::new(50, 50, 250, 250))
        .blur_radius(10)
        .corner_radius(15)
        .quality(BlurQuality::Precision)
        .draw();
    
    // Draw panel on top
    layer.fill(Area::new(50, 50, 250, 250))
        .color(Color::WHITE)
        .opacity(Opacity::OPA_60)
        .radius(15)
        .draw();
}
```

### Example 13: Complex UI Component

```rust
fn draw_button(layer: &mut Layer, area: Area, text: &'static str, font: &'static Font, pressed: bool) {
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
        layer.box_shadow(area)
            .color(Color::BLACK)
            .width(8)
            .offset(0, 2)
            .opacity(Opacity::OPA_30)
            .radius(8)
            .draw();
    }
    
    // Background
    layer.fill(area)
        .gradient(gradient)
        .radius(8)
        .draw();
    
    // Border
    layer.border(area)
        .color(Color::rgb(50, 80, 150))
        .width(2)
        .radius(8)
        .draw();
    
    // Text
    layer.label(area, text)
        .font(font)
        .color(Color::WHITE)
        .align_center()
        .draw();
}
```

---

## Summary

This specification defines a complete, fluent-style Rust API for rendering primitives based on LVGL. Key features:

1. **Fluent Builder Pattern**: All drawing operations use chainable method calls
2. **Type Safety**: Strong typing for colors, opacity, blend modes, etc.
3. **Comprehensive Coverage**: All LVGL primitives (rectangles, lines, arcs, triangles, images, text, vectors)
4. **Advanced Features**: Gradients, shadows, blurs, masks, transformations
5. **Ergonomic API**: Convenient shortcuts and sensible defaults
6. **Zero-cost Abstractions**: Designed to compile to efficient code

The API is designed to be:
- **Intuitive**: Method names clearly indicate their purpose
- **Composable**: Complex graphics built from simple primitives
- **Flexible**: Support for both simple and advanced use cases
- **Safe**: Rust's type system prevents common errors

