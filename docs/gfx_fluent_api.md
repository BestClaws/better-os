# Fluent Graphics API Specification (gfx crate)

## 1. Overview
The Fluent Graphics API defines the builder surface that will sit on top of the `gfx` crate. It focuses on creating ready-to-draw primitives that wrap the strongly typed data structures already available in the renderer, such as [`Rectangle<'a>`](gfx/src/primitives/rectangle/mod.rs#L16-L38), [`FillStyle`](gfx/src/primitives/features/fill.rs#L1-L7), [`StrokeStyle<'a, GS>`](gfx/src/primitives/features/stroke.rs#L4-L19), and the geometry helpers provided by `zeno` (for example [`Bounds`](gfx/src/primitives/rectangle/draw.rs#L1002-L1018) and `Point`).

## 2. Objectives
- Provide an ergonomic builder layer that produces the existing low-level structs without cloning or heap churn.
- Keep every step compatible with `no_std` plus `alloc`, matching the current crate configuration.
- Make builder output deterministic so that it can be serialized for sprite generation and fed directly into `RasterTarget` implementations.

## 3. Core Types
- Geometry comes from `zeno`, primarily [`Bounds`](gfx/src/primitives/rectangle/draw.rs#L1002-L1018) and `Point` for absolute positions.
- Visual state uses [`FillStyle<GS>`](gfx/src/primitives/features/fill.rs#L1-L7) and [`StrokeStyle<'a, GS>`](gfx/src/primitives/features/stroke.rs#L4-L19); gradients are stored via [`Gradient<GS>`](gfx/src/primitives/features/gradient.rs#L1-L18).
- Colors are expressed through [`Color`](gfx/src/colors.rs#L1-L32) helpers like `Color::rgba(r, g, b, a)`.
- Clipping is represented as extra `Bounds` passed alongside each primitive (see [`Rectangle::clip`](gfx/src/primitives/rectangle/mod.rs#L21-L24)).
- Rasterization targets implement [`RasterTarget`](gfx/src/rasterizer.rs#L1-L102) and receive the fully resolved primitives.

Gradients use const generics to fix the maximum stop count at compile time. For example, the rectangle implementation currently expects up to three stops for fills (`FillStyle<3>`) and two stops for per-edge strokes (`StrokeStyle<'a, 2>`).

## 4. Design Principles
- Absolute coordinates only. Builders receive final `Bounds`/`Point` values and pass them through untouched.
- Immediate execution. Calling `draw(&mut dyn RasterTarget)` on the finished primitive triggers rasterization directly.
- Shared vocabulary. Stroke and fill builders feed the same `StrokeStyle`/`FillStyle` structs into every primitive that consumes them.
- Clip-first workflow. Each primitive can optionally accept a `Bounds` clip; it is intersected with the canvas inside the renderer.
- Deterministic allocation. Builders pre-size the temporary buffers they need and rely on [`temp_allocation_peak_bytes`](gfx/src/primitives/rectangle/mod.rs#L10-L14) to track worst-case usage.

## 5. Supported Use Cases
- Rendering rounded rectangles with solid or gradient fills and per-edge borders.
- Preparing sprite atlas content offline by instantiating the builder graph and recording the resolved primitives.
- Driving embedded UIs where memory footprints are predictable and only the `alloc` crate is available.

Additional primitives (circles, arcs, vectors, text) remain in the roadmap but will be specified separately once their backing types land in the crate.

## 6. Fluent Workflow
All fluent types follow the same pattern:

1. Start from `::new()` (Rectangle) or an inherent constructor on the target type (for example `FillStyle::vertical_gradient`). Every entry point returns a meaningful default ready for drawing.
  - `Rectangle::new()` returns an empty rectangle with `FillStyle::transparent()` applied, all edges disabled, and the full-surface clip so callers override only what they need.
2. Apply configuration methods that mutate and return the same value, keeping chaining inexpensive.
3. Extract the final value either by calling `build()` on builders or by using the returned style instance directly.

Implementations avoid allocation unless the underlying primitive requires heap-backed buffers (for example gradient lookup tables), and those allocations stay bounded by static capacities.

## 7. Primitive Reference

### 7.1 Rectangle (`Rectangle<'a>`)
- **Required:**
  - `bounds(Bounds)` describing the axis-aligned outer rectangle.
- **Optional:**
  - `corner_radii([CornerRadius; 4])` with helpers for uniform radii.
  - `fill(FillStyle<3>)` returned by fluent style constructors.
  - Per-edge borders through `edge(Edge::Top, StrokeStyle<'a, 2>)` etc.
  - `clip(Bounds)` to override the default full-surface clip.
- **Behaviour:**
  - Layer order: fill first, then borders. If a border edge reports zero width it is skipped entirely.
  - Inner corner radii are renormalised automatically to preserve monotonic radii as seen in [compute_inner_radii](gfx/src/primitives/rectangle/draw.rs#L944-L990).

#### Example
```rust
use gfx::colors::Color;
use gfx::primitives::{CornerRadius, Rectangle};
use gfx::primitives::features::fill::FillStyle;
use gfx::primitives::features::stroke::StrokeStyle;
use zeno::{Bounds, Point, Stroke};

let top_fill = FillStyle::vertical_gradient([
  (Color::rgba(0x18, 0x5F, 0xA6, 0xFF), 0),
  (Color::rgba(0x46, 0xB3, 0xCE, 0xFF), 153),
  (Color::rgba(0x7A, 0xE4, 0xF2, 0xFF), 255),
]);

let mut base_stroke = Stroke::default();
base_stroke.width = 4.0;

let top_edge = StrokeStyle::from_stroke(base_stroke)
  .horizontal_gradient([
    (Color::rgba(0x4A, 0x90, 0xE2, 0xFF), 0),
    (Color::rgba(0x50, 0xE3, 0xC2, 0xFF), 255),
  ])
  .finalize();

let rect = Rectangle::new()
  .bounds(Bounds::new(Point::new(24.0, 24.0), Point::new(204.0, 132.0)))
  .corner_radii(CornerRadius::new(16.0, 16.0))
  .fill(top_fill)
  .edge(Edge::Top, top_edge)
  .clip(Bounds::new(Point::new(16.0, 16.0), Point::new(216.0, 144.0)))
  .build();

rect.draw(&mut rasterizer);
```

### 7.2 Future Primitive Families
Subsequent builder specifications will mirror the rectangle design but target new primitive structs once they exist in the crate:
- Circular and ring geometries once we expose `zeno::Arc` helpers and store radii directly.
- Stroke-only primitives (lines, paths) that wrap `zeno::PathBuilder` usage.
- Text and vector content once the font and SVG pipelines in the repository are stabilised.

Each new primitive will favour the same style builders (`FillStyle`, `StrokeStyle`) to avoid duplicating configuration paths.

## 8. Style APIs

### 8.1 FillStyle
- `FillStyle::transparent()` produces a default no-op fill used by builders out of the box.
- `FillStyle::solid(color)` returns a solid fill.
- `FillStyle::horizontal_gradient(stops)` and `FillStyle::vertical_gradient(stops)` capture linear gradients along each axis.
- `.with_opacity(u8)` returns a new fill where every stop alpha is scaled prior to sampling.

These constructors validate stop count at compile time through const generics and keep the resulting enum ready for reuse.

- `StrokeStyle::transparent()` yields the default edge style (zero width, full transparency) used when no edge configuration is supplied.
- `StrokeStyle::from_stroke(zeno::Stroke<'a>)` seeds a stroke with the desired width and cap/join state.
- `.solid(color)` replaces the color payload with a single tone and returns `self` for chaining.
- `.horizontal_gradient(stops)` or `.vertical_gradient(stops)` mirror the fill helpers.
- `.finalize()` yields a `StrokeStyle<'a, GS>` ready to attach to edges.

The fluent methods mutate in place but always return `Self` to allow chaining on a single value.

## 9. Style Vocabulary
- **FillStyle<GS>:** Either `Solid(Color)` or `Gradient(Gradient<GS>)`. Gradient stops are sampled on a 0–255 domain (see [`GradientTable`](gfx/src/primitives/rectangle/draw.rs#L211-L230)).
- **StrokeStyle<'a, GS>:** Couples a `StrokeColor<GS>` with a borrowed `zeno::Stroke<'a>` describing width, joins, caps, and dash pattern. Stroke widths are clamped to non-negative values before rendering ([edge classification](gfx/src/primitives/rectangle/draw.rs#L33-L84)).
- **CornerRadius (rectangle-only):** Stored per corner with independent horizontal/vertical radii on [`Rectangle<'a>`](gfx/src/primitives/rectangle/mod.rs#L16-L38). Normalization ensures radii never exceed the available space.
- **Clip Bounds:** Stored as `Bounds` and intersected with the canvas and primitive coverage in [`shape_clip`](gfx/src/primitives/rectangle/draw.rs#L1018-L1057).

## 10. Clipping and Constraints
- Invalid geometry (empty bounds, negative widths) is rejected during `build()` with descriptive errors.
- Clip rectangles default to the primitive bounds but can shrink rendering further.
- Gradients honour the orientation specified at construction time and never allocate more than 256 cached samples per axis chunk.

## 11. Error Handling
- Builder setters return `Result<Self, BuildError>` when validation fails (for example mismatched stop counts or out-of-range clip bounds).
- Drawing surfaces must advertise unsupported features (like per-edge gradients) via capability flags so builders can short-circuit before allocation.

## 12. Future Considerations
- Add higher-level presets (e.g., system card, focus ring) that output ready-to-use `Rectangle<'static>` instances backed by static stop tables.
- Extend the style APIs with reusable gradient palettes for common UI recipes.
- Expand the primitive set with text, vector paths, and mask-aware shapes once those engines share the same underlying geometry types.
