# GFX Library Overhaul - Complete Analysis

**Date:** 11 January 2026  
**Goal:** Replace existing gfx library with new LVGL-inspired rendering API

---

## Executive Summary

The current gfx library (`src/libs/gfx/`) provides basic 2D rendering primitives with an embedded-graphics compatibility layer. It needs to be completely replaced with a new LVGL-inspired rendering system based on designs in `lvgl_analysis/canonical/` and `lvgl_analysis/derived/rendering/`.

**Key Changes:**
- **Old:** Direct Rasterizer trait with RGB565-specific operations
- **New:** DrawTarget trait with format-agnostic Layer-based rendering
- **Old:** Shapes implement draw methods directly on Rasterizer
- **New:** Fluent builder API for primitives with deferred execution
- **Old:** Embedded-graphics compatibility via adapter patterns
- **New:** Native rendering API without external dependencies

---

## 1. Current GFX Library Structure

### Core Files and Modules

```
src/libs/gfx/
├── mod.rs              - Main exports, utility functions (aa_coverage, blend_rgb565, etc.)
├── color.rs            - Rgba8888 color type
├── compat.rs           - Embedded-graphics adapters (SurfaceDrawTarget, RasterizerDrawTarget)
├── fill.rs             - Fill primitive
├── font.rs             - Font rendering
├── rasterizer.rs       - Rasterizer trait
├── shapes/
│   ├── mod.rs          - Shape trait
│   ├── arc.rs
│   ├── circle.rs
│   ├── line.rs
│   ├── rounded_rect.rs
│   └── text.rs
└── three_d/
    ├── mod.rs
    ├── math.rs
    ├── model.rs
    └── render.rs       - 3D rendering
```

### Key Exports from `mod.rs`

```rust
pub use compat::color::{IntoEgRgb, IntoRgba8888};
pub use compat::draw_target::RasterizerDrawTarget;
pub use compat::surface::SurfaceDrawTarget;
pub use fill::Fill;
pub use font::{font_for_size, Charset, FontSize, MonoFont, DEFAULT_CHARSETS};
pub use rasterizer::Rasterizer;
pub use rasterizer::Rgb565Rasterizer;
pub use shapes::{Arc, Circle, RoundedRect, Shape};
pub use three_d::{draw_model, parse_binary_stl, Model, Quaternion, RenderOptions, StlError, Vec3};
```

### Rasterizer Trait (Current)

```rust
pub trait Rasterizer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);
    
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8);
    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>);
    fn blend_hspan_with(&mut self, x: i32, y: i32, len: i32, f: impl FnMut(usize) -> (Rgba8888, u8));
    fn blend_vspan_with(&mut self, x: i32, y: i32, len: i32, f: impl FnMut(usize) -> (Rgba8888, u8));
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888);
}
```

### Shape Trait (Current)

```rust
pub trait Shape {
    fn draw<R: super::Rasterizer>(&self, rasterizer: &mut R);
}
```

**Problem:** Shapes are tightly coupled to Rasterizer trait. Each shape directly modifies pixels via the rasterizer during `draw()`.

---

## 2. Integration Points - Where GFX is Used

### 2.1 DrawingSurface Integration

**File:** `src/system/ui/drawing_surface/surface.rs`

**Current State:**
- `DrawingSurface` implements `Rasterizer` trait (in `raster_impl.rs`)
- Provides format-agnostic pixel operations via `PixelOps` function pointers
- Supports RGB565 and Gray4 formats
- Tracks dirty regions automatically

**Key Dependencies:**
```rust
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{blend_rgb565, rgba8888_to_gray4_and_alpha, rgba8888_to_rgb565_and_alpha};
```

**Implemented Methods:**
```rust
impl Rasterizer for DrawingSurface<'_> {
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8);
    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>);
    fn blend_hspan_with(&mut self, x: i32, y: i32, len: i32, f: impl FnMut(usize) -> (Rgba8888, u8));
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888);
    // ... etc
}
```

**What Needs to Change:**
1. Remove `Rasterizer` trait implementation
2. Add `DrawTarget` trait implementation
3. Keep existing `PixelOps` system (it's already format-agnostic)
4. Adapt to Layer-based rendering model

### 2.2 AppContext Integration

**File:** `src/system/app/app_context.rs`

**Current State:**
```rust
pub async fn draw(&self, f: impl FnOnce(&mut DrawingSurface) + Send) {
    let mut wm = self.window_manager.lock().await;
    let _ = wm.with_surface(self.handle, f);
}
```

**Apps receive:** `&mut DrawingSurface`  
**Apps expect:** To use it as a `Rasterizer` for drawing shapes

**What Needs to Change:**
1. Keep the same signature (still passes `&mut DrawingSurface`)
2. Apps will need to create `Layer` from surface
3. Apps will use new fluent API instead of Shape trait

### 2.3 Embedded-Graphics Compatibility Layer

**File:** `src/libs/gfx/compat.rs`

**Current Components:**
- `SurfaceDrawTarget` - Wraps `DrawingSurface` as embedded-graphics `DrawTarget`
- `RasterizerDrawTarget` - Wraps any `Rasterizer` as embedded-graphics `DrawTarget`
- Color conversion traits: `IntoEgRgb`, `IntoRgba8888`

**Usage:** Used by font rendering (`embedded_graphics::text::Text`)

**What Needs to Change:**
1. **Option A:** Keep compatibility layer, adapt to new DrawTarget trait
2. **Option B:** Remove embedded-graphics dependency, implement native text rendering
3. **Recommendation:** Keep for Phase 1 (font rendering), remove in Phase 2

### 2.4 App Usage Patterns

**Files Using GFX:**
- `src/apps/gfx_bench.rs`
- `src/apps/watch_app.rs`
- `src/apps/rect.rs`
- `src/apps/arrow.rs`
- `src/apps/text_demo.rs`
- `src/apps/discord.rs`
- `src/apps/bluetooth_scanner.rs`
- `src/apps/gray_test.rs`

**Current Pattern:**
```rust
ctx.draw(|surface: &mut DrawingSurface| {
    // Create shape
    let circle = Circle::new(Point { x: 120, y: 120 }, 50)
        .color(Rgba8888::rgb(255, 0, 0));
    
    // Draw it directly
    circle.draw(surface);
    
    // Or use SurfaceDrawTarget for embedded-graphics
    let mut target = SurfaceDrawTarget::new(surface);
    Text::new("Hello", Point::new(10, 10), style).draw(&mut target).unwrap();
});
```

**New Pattern (Target):**
```rust
ctx.draw(|surface: &mut DrawingSurface| {
    // Create layer from surface
    let mut layer = Layer::from_draw_target(surface);
    
    // Use fluent API
    layer.circle(Point::new(120, 120), 50)
        .color(Color::RED)
        .draw();
    
    // Text rendering (if we keep embedded-graphics for now)
    let mut target = SurfaceDrawTarget::new(surface);
    Text::new("Hello", Point::new(10, 10), style).draw(&mut target).unwrap();
});
```

---

## 3. New GFX System Design

### 3.1 Core Architecture (from derived/simplified_rendering_api.md)

**DrawTarget Trait:**
```rust
pub trait DrawTarget {
    fn color_format(&self) -> ColorFormat;
    fn dimensions(&self) -> (u32, u32);
    fn stride(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];
    fn buffer(&self) -> &[u8];
    fn blend(&mut self, desc: &BlendDescriptor);
    fn clear(&mut self, color: Color);
}
```

**Layer:**
```rust
pub struct Layer {
    buffer: *mut [u8],
    width: u32,
    height: u32,
    buf_area: Rect,           // Which screen region buffer represents
    clip_area: Rect,          // Clipping rectangle
    partial_y_offset: i32,    // Y offset for strip rendering
    opacity: Opacity,
    color_format: ColorFormat,
}

impl Layer {
    pub fn new(buffer: &mut [u8], width: u32, height: u32, format: ColorFormat) -> Self;
    pub fn from_draw_target(target: &mut impl DrawTarget, area: Rect) -> Self;
    pub fn clear(&mut self, color: Color);
    pub fn composite(&mut self, source: &Layer, dest_area: Rect);
}
```

**Key Insight:** Layer bridges between logical screen coordinates and buffer storage.

### 3.2 Fluent Builder API

**Rectangles:**
```rust
layer.fill(Rect::new(10, 10, 100, 50))
    .color(Color::BLUE)
    .opacity(Opacity::OPA_50)
    .draw();

layer.border(Rect::new(10, 10, 100, 50))
    .color(Color::WHITE)
    .width(2)
    .radius(5)
    .draw();
```

**Circles:**
```rust
layer.circle(Point::new(120, 120), 50)
    .color(Color::RED)
    .draw();
```

**Lines:**
```rust
layer.line(PointF::new(0.0, 0.0), PointF::new(100.0, 100.0))
    .color(Color::WHITE)
    .width(2)
    .draw();
```

**Arcs:**
```rust
layer.arc(Point::new(120, 120), 60)
    .angles(0.0, 180.0)
    .width(3)
    .color(Color::CYAN)
    .draw();
```

### 3.3 Gradient Support (from canonical/rust_rendering_api_spec.md)

```rust
let gradient = Gradient::new()
    .vertical()
    .add_stop(Color::RED, 0)
    .add_stop(Color::BLUE, 255);

layer.fill(rect)
    .gradient(gradient)
    .draw();
```

### 3.4 Color System

**Core Types:**
```rust
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct ColorAlpha {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Opacity(u8);

pub enum ColorFormat {
    L8,
    A8,
    Rgb565,
    Rgb888,
    Argb8888,
    Xrgb8888,
}
```

**Current:** Uses `Rgba8888` (different naming convention)  
**New:** Separate `Color` (RGB) and `ColorAlpha` (RGBA), plus `Opacity` type

---

## 4. Migration Strategy

### Phase 1: Foundation Layer (DrawTarget + Layer)

**Goal:** Implement core abstractions without breaking existing code

**Steps:**
1. Create new module structure under `src/libs/gfx/`
2. Implement core types:
   - `Color`, `ColorAlpha`, `Opacity` (replace `Rgba8888`)
   - `ColorFormat` enum
   - `Point`, `PointF`, `Rect` (coordinate with `util::math::primitives`)
3. Implement `DrawTarget` trait
4. Implement `Layer` struct with coordinate mapping
5. Keep old code intact for now

**Files to Create:**
```
src/libs/gfx/
├── core/
│   ├── mod.rs
│   ├── color.rs          - Color, ColorAlpha, Opacity, ColorFormat
│   ├── geometry.rs       - Point, PointF, Rect
│   └── blend.rs          - BlendMode, BlendDescriptor
├── draw_target.rs        - DrawTarget trait
└── layer.rs              - Layer implementation
```

### Phase 2: DrawTarget Implementation for DrawingSurface

**Goal:** Make DrawingSurface implement DrawTarget

**Steps:**
1. Implement `DrawTarget` for `DrawingSurface`
2. Keep `Rasterizer` implementation temporarily (for backward compat)
3. Update internal pixel operations to be compatible with both

**File:** `src/system/ui/drawing_surface/draw_target_impl.rs`

```rust
impl DrawTarget for DrawingSurface<'_> {
    fn color_format(&self) -> ColorFormat {
        match self.pixel_format() {
            PixelFormat::Rgb565 => ColorFormat::Rgb565,
            PixelFormat::Gray4 => ColorFormat::L8, // approximate
            _ => ColorFormat::Rgb565,
        }
    }
    
    fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    
    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
    
    fn buffer(&self) -> &[u8] {
        self.buffer()
    }
    
    fn blend(&mut self, desc: &BlendDescriptor) {
        // Implementation using existing PixelOps
    }
    
    fn clear(&mut self, color: Color) {
        let rgba = Rgba8888::rgba(color.r, color.g, color.b, 255);
        self.clear(rgba);
    }
}
```

### Phase 3: Primitive Implementations

**Goal:** Implement all drawing primitives with fluent API

**Files to Create:**
```
src/libs/gfx/primitives/
├── mod.rs
├── rectangle.rs      - Fill, Border, Shadow
├── line.rs           - Line
├── arc.rs            - Arc, Circle
├── triangle.rs       - Triangle
├── text.rs           - Text (if removing embedded-graphics)
└── gradient.rs       - Gradient, GradientStop
```

**Implementation Pattern:**
```rust
pub struct Fill<'a> {
    layer: &'a mut Layer,
    rect: Rect,
    color: Color,
    gradient: Option<Gradient>,
    opacity: Opacity,
    radius: i32,
}

impl<'a> Fill<'a> {
    pub fn new(layer: &'a mut Layer, rect: Rect) -> Self {
        Self {
            layer,
            rect,
            color: Color::WHITE,
            gradient: None,
            opacity: Opacity::OPAQUE,
            radius: 0,
        }
    }
    
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    
    pub fn draw(self) {
        // Implementation: rasterize and blend into layer
    }
}

impl Layer {
    pub fn fill(&mut self, rect: Rect) -> Fill {
        Fill::new(self, rect)
    }
}
```

### Phase 4: Update Apps

**Goal:** Migrate all apps to new API

**For each app:**
1. Remove old imports
2. Add new imports
3. Create Layer from DrawingSurface
4. Convert Shape::new().draw() to layer.shape().draw()
5. Test rendering output

**Example Migration:**

**Before:**
```rust
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Circle, RoundedRect};

ctx.draw(|surface: &mut DrawingSurface| {
    let circle = Circle::new(Point { x: 120, y: 120 }, 50)
        .color(Rgba8888::rgb(255, 0, 0));
    circle.draw(surface);
});
```

**After:**
```rust
use crate::libs::gfx::{Color, Layer, Point};

ctx.draw(|surface: &mut DrawingSurface| {
    let mut layer = Layer::from_draw_target(surface);
    layer.circle(Point::new(120, 120), 50)
        .color(Color::RED)
        .draw();
});
```

### Phase 5: Delete Old Code

**Goal:** Remove deprecated gfx library

**Files to Delete:**
```
src/libs/gfx/
├── rasterizer.rs         ❌ DELETE
├── shapes/               ❌ DELETE entire directory
│   ├── arc.rs
│   ├── circle.rs
│   ├── line.rs
│   ├── rounded_rect.rs
│   └── text.rs
├── fill.rs               ❌ DELETE (replaced by primitives/rectangle.rs)
└── compat.rs             ❌ DELETE or KEEP (decision needed for fonts)
```

**Keep:**
```
src/libs/gfx/
├── three_d/              ✅ KEEP (3D rendering separate concern)
└── font.rs               ✅ KEEP (may need adaptation)
```

---

## 5. Non-GFX Component Changes Required

### 5.1 DrawingSurface Changes

**File:** `src/system/ui/drawing_surface/surface.rs`

**Required Changes:**

1. **Add DrawTarget implementation**
   - New file: `draw_target_impl.rs`
   - Implement all DrawTarget methods
   - Reuse existing PixelOps for format-specific operations

2. **Keep backward compatibility temporarily**
   - Keep Rasterizer implementation during migration
   - Deprecate after all apps migrated

3. **Color type mapping**
   - Add conversion utilities between `Color`/`ColorAlpha` and `Rgba8888`
   - Eventually replace `Rgba8888` with new color types

**Estimated Impact:** Medium (new trait impl, but existing pixel ops reusable)

### 5.2 AppContext Changes

**File:** `src/system/app/app_context.rs`

**Required Changes:**
- **None!** The `draw()` method signature stays the same
- Apps still receive `&mut DrawingSurface`
- Apps are responsible for creating Layer internally

**Estimated Impact:** None

### 5.3 Compositor/Window Manager Changes

**Files:**
- `src/system/ui/compositor/core/render.rs`
- `src/system/ui/compositor/core/transitions.rs`

**Current Usage:**
```rust
use crate::libs::gfx::color::Rgba8888;
```

**Required Changes:**
- Update imports to use new Color types
- Update any direct color manipulation to use new API
- Should be minimal - these mostly pass colors through

**Estimated Impact:** Low (simple import changes)

### 5.4 Utility Math Types Coordination

**File:** `src/util/math/primitives.rs`

**Current State:**
- Has its own `Point`, `Rect`, `Size` types
- Used throughout the system

**Required Changes:**
- **Option A:** Use existing types, add conversions in gfx
- **Option B:** Replace with gfx types system-wide
- **Recommendation:** Option A (less disruption)

**Implementation:**
```rust
// In gfx module
impl From<util::math::primitives::Point> for gfx::Point {
    fn from(p: util::math::primitives::Point) -> Self {
        gfx::Point::new(p.x, p.y)
    }
}
```

**Estimated Impact:** Low (add conversion utilities)

---

## 6. Implementation Priority Order

### Priority 1: Core Foundation (Can implement in parallel)

1. **Core types** (`core/color.rs`, `core/geometry.rs`)
2. **DrawTarget trait** (`draw_target.rs`)
3. **Layer implementation** (`layer.rs`)
4. **DrawingSurface::DrawTarget impl** (`drawing_surface/draw_target_impl.rs`)

### Priority 2: Basic Primitives (Need Priority 1)

5. **Rectangle primitives** (`primitives/rectangle.rs`)
   - Fill (solid rect)
   - Border
6. **Circle primitive** (`primitives/arc.rs`)
7. **Line primitive** (`primitives/line.rs`)

### Priority 3: Apps Migration (Need Priority 2)

8. **Migrate simple apps** (rect, gray_test, gfx_bench)
9. **Migrate complex apps** (watch_app, discord, bluetooth_scanner)
10. **Migrate 3D apps** (arrow) - may need special handling

### Priority 4: Advanced Features

11. **Gradient support** (`primitives/gradient.rs`)
12. **Arc primitive** (for watch_app)
13. **Shadow effects**
14. **Text rendering** (native implementation or keep compat)

### Priority 5: Cleanup

15. **Delete old Shape trait and implementations**
16. **Delete old Rasterizer trait**
17. **Remove embedded-graphics dependency** (if decided)
18. **Update documentation**

---

## 7. Risk Assessment

### High Risk Areas

1. **Breaking app rendering**
   - Mitigation: Keep both APIs working during migration
   - Test: Visual comparison screenshots before/after

2. **Performance regression**
   - Risk: New abstraction layers might be slower
   - Mitigation: Use inline functions, benchmark critical paths
   - Test: Run gfx_bench before/after

3. **Coordinate system confusion**
   - Risk: Layer vs screen vs buffer coordinates
   - Mitigation: Clear documentation, assertions
   - Test: Edge cases (clipping, partial rendering)

### Medium Risk Areas

4. **Font rendering breakage**
   - If removing embedded-graphics, need native text
   - Mitigation: Phase this separately, keep compat initially

5. **3D rendering integration**
   - Current 3D code directly manipulates pixels
   - Mitigation: Keep three_d module separate, adapt minimal interface

### Low Risk Areas

6. **Color type changes**
   - Simple rename/restructure
   - Mitigation: Conversion utilities

7. **App updates**
   - Mechanical transformation
   - Mitigation: Update one app at a time, test each

---

## 8. Testing Strategy

### Unit Tests

**Core Types:**
- Color construction and conversions
- Opacity calculations
- Rect intersection/clipping
- Coordinate transformations

**Layer:**
- Coordinate mapping (buf_area, screen coordinates)
- Strip rendering mode
- Clipping

**Primitives:**
- Rectangle at various positions/sizes
- Circles with different radii
- Lines at different angles
- Edge cases (clipping, zero-size, negative coords)

### Integration Tests

**DrawingSurface:**
- DrawTarget implementation correctness
- Format conversion (RGB565, Gray4)
- Dirty region tracking
- Blend operations

### Visual Tests

**Apps:**
- Screenshot each app before migration
- Screenshot each app after migration
- Pixel-by-pixel comparison (or manual inspection)

**Test Apps:**
```rust
// test_primitives.rs
ctx.draw(|surface| {
    let mut layer = Layer::from_draw_target(surface);
    
    // Grid of test primitives
    layer.fill(Rect::new(0, 0, 50, 50)).color(Color::RED).draw();
    layer.circle(Point::new(75, 25), 20).color(Color::BLUE).draw();
    layer.line(PointF::new(100.0, 0.0), PointF::new(150.0, 50.0))
        .color(Color::GREEN).width(2).draw();
});
```

### Performance Tests

**Benchmarks:**
- Fill large rectangle
- Draw many small circles
- Complex scene (watch face)
- Compare old vs new implementation

---

## 9. Decision Points

### Decision 1: Embedded-Graphics Compatibility

**Options:**
- **A:** Keep compatibility layer for font rendering
- **B:** Implement native text rendering
- **C:** Keep for Phase 1, remove in Phase 2

**Recommendation:** Option C
- **Rationale:** Font rendering is complex, don't block main migration
- **Timeline:** Phase 1 (keep), Phase 2 (remove once native text ready)

### Decision 2: Coordinate Type Unification

**Options:**
- **A:** Use existing `util::math::primitives` types
- **B:** Use new gfx types everywhere
- **C:** Have both with conversion utilities

**Recommendation:** Option C
- **Rationale:** Minimize disruption to non-gfx code
- **Implementation:** Add `From` trait implementations

### Decision 3: Three_d Module Integration

**Options:**
- **A:** Leave three_d completely separate
- **B:** Port three_d to use DrawTarget
- **C:** Remove three_d (if not used)

**Recommendation:** Option A for initial migration
- **Rationale:** 3D rendering is specialized, minimize scope
- **Future:** Can integrate later if needed

### Decision 4: Gradients in Phase 1?

**Options:**
- **A:** Include gradient support in Phase 1
- **B:** Defer to Phase 2 (after basic primitives working)

**Recommendation:** Option B
- **Rationale:** No current apps use gradients
- **Benefit:** Faster initial migration

---

## 10. File-by-File Change Summary

### Files to Create (New GFX API)

```
src/libs/gfx/
├── core/
│   ├── mod.rs              [NEW] - Core type exports
│   ├── color.rs            [NEW] - Color, ColorAlpha, Opacity, ColorFormat
│   ├── geometry.rs         [NEW] - Point, PointF, Rect
│   └── blend.rs            [NEW] - BlendMode, BlendDescriptor
├── draw_target.rs          [NEW] - DrawTarget trait definition
├── layer.rs                [NEW] - Layer implementation
└── primitives/
    ├── mod.rs              [NEW] - Primitive exports
    ├── rectangle.rs        [NEW] - Fill, Border, Shadow
    ├── line.rs             [NEW] - Line
    ├── arc.rs              [NEW] - Arc, Circle
    ├── triangle.rs         [NEW] - Triangle
    └── gradient.rs         [NEW] - Gradient, GradientStop (Phase 2)
```

### Files to Modify (Glue Layer)

```
src/system/ui/drawing_surface/
├── mod.rs                  [MODIFY] - Add draw_target export
├── surface.rs              [MODIFY] - Add ColorFormat mapping
├── draw_target_impl.rs     [NEW] - DrawTarget implementation
└── raster_impl.rs          [KEEP] - Temporary backward compat
```

### Files to Delete (Old GFX)

```
src/libs/gfx/
├── rasterizer.rs           [DELETE in Phase 5]
├── fill.rs                 [DELETE in Phase 5]
├── shapes/                 [DELETE in Phase 5]
│   ├── mod.rs
│   ├── arc.rs
│   ├── circle.rs
│   ├── line.rs
│   ├── rounded_rect.rs
│   └── text.rs
└── compat.rs               [DECISION NEEDED - fonts]
```

### Files to Update (Apps)

```
src/apps/
├── gfx_bench.rs            [UPDATE] - Use new API
├── watch_app.rs            [UPDATE] - Use new API
├── rect.rs                 [UPDATE] - Use new API
├── arrow.rs                [UPDATE] - Use new API (special: 3D)
├── text_demo.rs            [UPDATE] - Use new API
├── discord.rs              [UPDATE] - Use new API
├── bluetooth_scanner.rs    [UPDATE] - Use new API
└── gray_test.rs            [UPDATE] - Use new API
```

### Files to Update (Minimal Changes)

```
src/system/ui/compositor/core/
├── render.rs               [UPDATE] - Import changes only
└── transitions.rs          [UPDATE] - Import changes only

src/system/app/
└── app_context.rs          [NO CHANGE]
```

---

## 11. Next Steps - Implementation Plan

### Step 1: Create Core Foundation (TODAY)

1. Create directory structure
2. Implement Color, ColorAlpha, Opacity
3. Implement Point, PointF, Rect (or use existing)
4. Define DrawTarget trait
5. Implement Layer struct (basic version)

**Deliverable:** Core types compile and have basic tests

### Step 2: DrawingSurface Integration (DAY 2)

1. Implement DrawTarget for DrawingSurface
2. Add Layer::from_draw_target() helper
3. Test basic layer creation
4. Keep Rasterizer impl working

**Deliverable:** Can create Layer from DrawingSurface

### Step 3: First Primitive - Fill Rectangle (DAY 2-3)

1. Implement Fill struct with fluent API
2. Implement Fill::draw() rasterization
3. Add Layer::fill() helper
4. Test with simple app

**Deliverable:** Can draw filled rectangles with new API

### Step 4: More Primitives (DAY 3-4)

1. Implement Circle
2. Implement Line
3. Implement Border

**Deliverable:** Basic shape set working

### Step 5: Migrate First App (DAY 4)

1. Choose simple app (gray_test or rect)
2. Update to use new API
3. Compare visual output
4. Document pattern

**Deliverable:** One app fully migrated and working

### Step 6: Migrate Remaining Apps (DAY 5-6)

1. Update each app systematically
2. Test each one
3. Fix any issues

**Deliverable:** All apps using new API

### Step 7: Cleanup (DAY 7)

1. Delete old Shape trait files
2. Delete old Rasterizer trait
3. Remove backward compat code
4. Update documentation

**Deliverable:** Clean codebase with new GFX system

---

## 12. References

### Design Documents

- `lvgl_analysis/canonical/rust_rendering_api_spec.md` - Complete API specification
- `lvgl_analysis/canonical/lvgl_rendering_pipeline.md` - LVGL architecture
- `lvgl_analysis/canonical/lvgl_rasterization_and_blending.md` - Rendering algorithms
- `lvgl_analysis/derived/rendering/simplified_rendering_api.md` - Simplified API
- `lvgl_analysis/derived/rendering/draw_targets_and_rendering.md` - DrawTarget design
- `lvgl_analysis/derived/rendering/simplified_rendering_flow.md` - Flow documentation

### LVGL Reference Code

- `references/lvgl/src/` - Original LVGL implementation for algorithms
- Prefer LVGL C code over markdown for actual implementation details
- Use canonical docs for "what", LVGL code for "how"

### Existing Implementation

- `src/libs/gfx/` - Current implementation to be replaced
- `src/system/ui/drawing_surface/` - Integration point
- `src/apps/` - Usage examples and test cases

---

## Appendix A: API Comparison Examples

### Example 1: Drawing a Circle

**Current API:**
```rust
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::Circle;
use crate::libs::gfx::rasterizer::Rasterizer;

let circle = Circle::new(Point { x: 120, y: 120 }, 50)
    .color(Rgba8888::rgb(255, 0, 0));
circle.draw(surface);
```

**New API:**
```rust
use crate::libs::gfx::{Color, Layer, Point};

let mut layer = Layer::from_draw_target(surface);
layer.circle(Point::new(120, 120), 50)
    .color(Color::RED)
    .draw();
```

### Example 2: Drawing a Rounded Rectangle

**Current API:**
```rust
use crate::libs::gfx::{RoundedRect, Shape};

let rect = RoundedRect::new(
    Point { x: 10, y: 10 },
    Point { x: 110, y: 60 },
    10,
    Rgba8888::rgba(0, 0, 255, 200)
);
rect.draw(surface);
```

**New API:**
```rust
layer.fill(Rect::new(10, 10, 100, 50))
    .color(Color::BLUE)
    .radius(10)
    .opacity(Opacity::from_percent(78))
    .draw();
```

### Example 3: Drawing with SurfaceDrawTarget (Embedded-Graphics)

**Current API:**
```rust
use crate::libs::gfx::SurfaceDrawTarget;
use embedded_graphics::text::Text;

let mut target = SurfaceDrawTarget::new(surface);
Text::new("Hello", Point::new(10, 10), style)
    .draw(&mut target)
    .unwrap();
```

**New API (Phase 1 - Keep Compat):**
```rust
// Same as before - no change needed
let mut target = SurfaceDrawTarget::new(surface);
Text::new("Hello", Point::new(10, 10), style)
    .draw(&mut target)
    .unwrap();
```

**New API (Phase 2 - Native Text):**
```rust
layer.text("Hello", Point::new(10, 10))
    .font(Font::MONO_12)
    .color(Color::WHITE)
    .draw();
```

---

## Appendix B: Performance Considerations

### Current Performance Characteristics

**Strengths:**
- Direct pixel manipulation in Rasterizer trait
- Function pointers in PixelOps avoid virtual dispatch
- Inline blend operations

**Weaknesses:**
- No batching or deferred rendering
- Every shape immediately rasterizes
- No GPU acceleration path

### New API Performance

**Improvements:**
- Deferred execution allows optimization
- BlendDescriptor enables GPU backend later
- Layer abstraction supports strip rendering

**Potential Issues:**
- Builder pattern adds small overhead (but should inline)
- Extra indirection through DrawTarget trait
- Mitigation: Monomorphization, inline hints

**Benchmark Plan:**
1. Measure current gfx_bench baseline
2. Implement new primitives
3. Compare performance
4. Profile and optimize hot paths

---

## Appendix C: Open Questions

### Question 1: Font Rendering Strategy

**Context:** Current system uses embedded-graphics for text rendering

**Options:**
1. Keep embedded-graphics dependency
2. Implement native bitmap font renderer
3. Add FreeType/TrueType support

**Dependencies:** Affects compat.rs deletion timeline

### Question 2: Gradient Implementation Priority

**Context:** No current apps use gradients

**Question:** Should gradients be in Phase 1 or deferred?

**Recommendation:** Defer to Phase 2

### Question 3: 3D Rendering Integration

**Context:** arrow.rs uses 3D rendering (three_d module)

**Question:** How to integrate with new DrawTarget?

**Options:**
1. Keep separate (3D renders to buffer, then composite)
2. Make three_d use DrawTarget
3. Remove 3D support

**Recommendation:** Keep separate for Phase 1

### Question 4: Anti-Aliasing Strategy

**Context:** Current system has aa_coverage() utility

**Question:** How to handle AA in new primitives?

**Reference:** LVGL has sophisticated AA (see lvgl_rasterization_and_blending.md)

**Recommendation:** Implement LVGL-style AA per primitive

---

## Document Version

**Version:** 1.0  
**Date:** 11 January 2026  
**Author:** Analysis for GFX Library Overhaul  
**Status:** Ready for Implementation

---

**Next Action:** Begin Step 1 - Create Core Foundation
