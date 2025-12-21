# GFX Rasterizer Design (Draft)

Status: Proposal
Owner: gfx subsystem
Last updated: 2025-12-21

## Goals
- Public GFX API speaks only `RGBA8888` (`Rgba8888`) for colors.
- Rasterizer implementations own pixel format specifics (RGB565, RGBA8888, GRAY8, etc.).
- Primitives remain pixel-format agnostic; all color math is expressed in RGBA and handed to the rasterizer.
- Provide fast path ops (hspan/vspan/fill) to avoid per-pixel overhead when possible.

## Decisions (Initial Phase)
- Target backend: RGB565 first.
- Keep a raw buffer escape hatch as `unsafe_buffer_mut()` behind a feature or explicit unsafe API, not used by primitives.
- Standardize on premultiplied alpha internally within backends for speed and simpler blending math.
- Add closure-based span API to avoid temporary allocations for gradients/AA where generating colors on-the-fly is cheaper.

## Current State (Observed)
- `Rasterizer` exposes raw `buffer_mut(): &mut [u8]` and shapes compute indices assuming RGB565 (`idx = (y * w + x) * 2`).
- Blending and lerp utilities are RGB565-specific (`blend_rgb565`, `lerp_rgb565`).
- `Fill` now encodes colors as `Rgba8888`; gradients are computed in RGBA and handed to the rasterizer. Legacy RGB565 conversions have been removed from primitives; backends still convert internally to their native formats.
- All primitives (`Circle`, `Arc`, `RoundedRect`, `Line`, `Text`) read the background pixel and blend in RGB565 inside the shape.

This tightly couples primitives to one format (RGB565) and prevents backends from choosing their native formats.

## Design Principles
- Shapes generate per-pixel color and coverage in RGBA-space only.
- Rasterizer handles background read, conversion, and blending in native format.
- Provide batch (span) ops for speed; per-pixel ops exist as a fallback.
- Keep the trait usable with generics (`R: Rasterizer`) and avoid heavy dynamic dispatch.
- Maintain `mark_dirty` rectangle semantics; rasterizer tracks dirty regions.

## Proposed Interfaces

### Color Type
- Keep `Rgba8888` as the public color type. Alpha is part of the color; additional per-pixel coverage (AA) is passed as a separate `u8`.

### PixelFormat (backend-internal)
A trait implemented by backends to define conversion and blending in their native pixel type.

```rust
pub trait PixelFormat {
    type Pixel: Copy; // e.g., u16 for RGB565, u32 for RGBA8888

    // Convert from public RGBA8888 to native pixel representation
    fn to_native(c: Rgba8888) -> Self::Pixel;

    // Blend foreground over background using final alpha = coverage * fg.a / 255
    fn blend(bg: Self::Pixel, fg: Self::Pixel, coverage: u8) -> Self::Pixel;
}
```

- Implementations can choose premultiplied alpha internally; contract is that `coverage` already accounts for AA coverage only, while color alpha comes from `Rgba8888`.

### Rasterizer
Replace buffer exposure with high-level ops. Backends are free to store any native pixel format.

```rust
pub trait Rasterizer {
    type Format: PixelFormat;

    fn width(&self) -> i32;
    fn height(&self) -> i32;

    // Mark dirty region; required for partial updates
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);
# GFX Rasterizer Design

Status: Proposal (condensed)
Last updated: 2025-12-21

## Goals
- Public GFX API uses `Rgba8888` for all colors.
- Backends own pixel-format specifics (e.g., RGB565) and convert internally.
- Primitives are pixel-format agnostic; they emit RGBA + coverage.
- Provide fast ops (spans/fills) to avoid per-pixel overhead.

## Decisions
- Initial backend: RGB565.
- Unsafe escape hatch: `unsafe_buffer_mut()` allowed; primitives won’t use it.
- Premultiplied alpha inside backends.
- Closure-based span APIs to avoid temporary allocations.
- No feature flag gating for the escape hatch.

## API Surface
Color: `Rgba8888` remains the public color type.

Rasterizer trait (concept):
```rust
pub trait Rasterizer {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);

    // Universal per-pixel blend
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8);

    // Span blend with slices (solid or gradient)
    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>);

    // Span blend with closures (no temp buffers)
    fn blend_hspan_with(&mut self, x: i32, y: i32, len: i32, f: impl FnMut(usize) -> (Rgba8888, u8));
    fn blend_vspan_with(&mut self, x: i32, y: i32, len: i32, f: impl FnMut(usize) -> (Rgba8888, u8));

    // Solid fill rect
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888);

    // Unsafe raw buffer (escape hatch)
    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8];
}
```

## Composition Rules
- Separate concerns: color alpha from `Rgba8888`, coverage from geometry/AA.
- Effective alpha is $\text{eff} = \frac{coverage \cdot A_{rgba}}{255}$.
- Backends use premultiplied alpha for blending:
  $$C_{out} = C_{fg} + (1 - A_{fg}) \cdot C_{bg}$$

## Primitive Guidance
- Compute coverage via geometry (e.g., `aa_coverage`).
- Compute final `Rgba8888` for fill/gradient.
- Use `blend_pixel` for sparse/edge pixels; `blend_hspan_with` for scanlines; `fill_rect` for solid areas.
- Call `mark_dirty` once per shape with bounding box.

## Migration Plan
1. Add the new trait methods and keep existing backends working.
2. Implement `Rgb565Rasterizer` (premultiplied internally, conversions on write).
3. Refactor primitives incrementally to use `blend_*`/`fill_rect` and remove direct RGB565 blending.
4. Retire direct buffer manipulation from primitives after migration.

## Performance Notes
- Span + closure APIs avoid allocations and permit tight inner loops.
- RGB565 conversion from RGBA is cheap (bit shifts); backends can vectorize or DMA when available.
- Dirty-region tracking remains unchanged.
// Old

## Next Steps
- Adopt `blend_hspan_with` in `RoundedRect` and `Circle` for inner fills to reduce per-pixel overhead.
- Implement `blend_vspan_with` for vertical gradients to minimize function calls.
- Add a unified benchmark harness that generates permutations (shapes, fills, strokes, alpha) via arrays and loops with single `DELAY_MS` config.
- Validate premultiplied alpha math across edge cases (low alpha + high coverage) with screenshot comparisons.
- Add optional AA mode toggles to primitives to test quality/performance trade-offs.
- Provide a backend-agnostic screenshot comparison utility to detect regressions.
