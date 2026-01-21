# Fluent API Documentation Gaps

The reference at `gfx fluent api.md` has drifted from the actual fluent builders. For quick repair, address these mismatches:

- **Constructor names:** Update sections 6 and 7 to use the real builders (`Rect::new()`, `StrokeBuilder::new()`, `GradientBuilder::linear()`, `FillBuilder::gradient()`, `OutlineBuilder::with_stroke()`, `ShadowBuilder::new()`, `StrokePlan::solid()`, `FillPlan::solid()`). The current prose and examples reference non-existent helpers like `Stroke::new()` and `Gradient::linear()`.
- **Rectangle layering:** The doc states “shadow → outline → fill → border,” but the engine draws shadow → fill → border → outline (`rust-gfx/src/primitives/rectangle/mod.rs`). Correct the ordering text.
- **Arc fill support:** `ArcPrimitive::draw()` ignores its `fill` slot (TODO remains). Remove or qualify the conic gradient example until fill rendering exists.
- **Label opacity example:** `LabelOpacity::new` takes a raw `u8`. Replace `LabelOpacity::new(Opa::new(200))` with `LabelOpacity::new(200)`.

Verifying these fixes and re-running sprite comparisons will keep the spec aligned with the shipped code.
