# Fluent Graphics API Specification

## 1. Overview
The Fluent Graphics API provides a strongly typed, builder-driven surface for configuring drawing primitives in embedded and `no_std` contexts while maintaining predictable resource usage.

## 2. Objectives
- Deliver ergonomic builders that hide low-level setup without introducing runtime overhead.
- Preserve predictability for embedded targets by favouring stack allocation and fixed-capacity buffers.
- Focus strictly on drawing semantics; higher-level widget composition remains out of scope.

## 3. Design Principles
- **Absolute coordinates only:** All primitives receive final screen coordinates. No translation helpers or matrices are applied during rendering.
- **Immediate execution:** `draw(&mut rasterizer)` applies the configured primitive directly to the target surface.
- **Shared style vocabulary:** Strokes, fills, shadows, and outlines share identical configuration paths across primitives.
- **Clip-first mindset:** Rectangular clipping via `clip(Area)` is the primary mechanism for constraining output.
- **No implicit transforms:** Matrix transforms are intentionally excluded from this revision to align with current primitive capabilities.

## 4. Supported Use Cases
- Rendering sequences of rectangles, circles, arcs, lines, triangles, labels, and vector paths with mixed styling.
- Generating sprites or off-screen assets that require repeatable draw pipelines.
- Building UI elements composed of rounded rectangles, gradients, shadows, and text with minimal setup cost.
- Driving custom vector drawing flows without exposing internal rendering details.

## 5. Fluent Builder Workflow
The builders follow a consistent three-step pattern:
1. Instantiate a primitive with `::new()`.
2. Configure geometry, styles, and optional clipping through chained setter methods.
3. Invoke `draw(&mut rasterizer)` to execute the configured primitive against the rasterizer implementation.

All setters return the builder, and every builder exposes `finish()` to seal the configuration before draw-time use.

## 6. Primitive Reference

### 6.1 Rectangle (`Rect`)
- **Required:** `area(Area)` describing the inclusive rectangle bounds.
- **Optional:** `radius` for rounded corners, `stroke`, `fill`, `outline`, `shadow`, and `clip`.
- **Behaviour:** Rendering order is shadow → outline → fill → border to preserve expected layering.

```rust
let rect = Rect::new()
    .area(Area::from_corners(Point::new(24, 24), Point::new(204, 132)))
    .radius(Radius::uniform(16))
    .stroke(
        Stroke::new()
            .width(4)
            .gradient(
                Gradient::linear()
                    .axis(Axis::Horizontal)
                    .stops([
                        GradientStop::new(0.0, Rgba8888::rgb(0x18, 0x5F, 0xA6)),
                        GradientStop::new(0.6, Rgba8888::rgb(0x46, 0xB3, 0xCE)),
                        GradientStop::new(1.0, Rgba8888::rgb(0x7A, 0xE4, 0xF2)),
                    ])
                    .finish(),
            )
            .finish(),
    )
    .fill(
        Fill::gradient()
            .axis(Axis::Vertical)
            .stops([
                GradientStop::new(0.0, Rgba8888::rgb(0xFE, 0xEC, 0x9A)),
                GradientStop::new(1.0, Rgba8888::rgb(0xFD, 0xC6, 0x42)),
            ])
            .finish(),
    )
    .outline(
        Outline::with_stroke(Stroke::solid(1, Rgba8888::rgba(0x00, 0x00, 0x00, 0x40)))
            .pad(2)
            .finish(),
    )
    .shadow(
        Shadow::new()
            .offset(Point::new(6, 8))
            .blur(12)
            .color(Rgba8888::rgba(0x12, 0x1E, 0x26, 0x50))
            .finish(),
    )
    .finish();

rect.draw(&mut rasterizer);
```

### 6.2 Circle (`Circle`)
- **Required:** `center(Point)` and `radius(i32)`.
- **Optional:** `stroke`, `fill`, `shadow`, and `clip`.
- **Behaviour:** Treats the circle as a specialised rectangle with uniform radius and honours the same layering rules.

```rust
let circle = Circle::new()
    .center(Point::new(160, 96))
    .radius(48)
    .stroke(Stroke::solid(6, Rgba8888::rgb(0x21, 0x2B, 0x36)))
    .fill(
        Fill::gradient()
            .kind(GradientKind::Radial)
            .center(Point::new(160, 96))
            .radius(64)
            .stops([
                GradientStop::new(0.0, Rgba8888::rgb(0xFF, 0xFF, 0xFF)),
                GradientStop::new(0.5, Rgba8888::rgb(0xE4, 0xFF, 0xF5)),
                GradientStop::new(1.0, Rgba8888::rgb(0x9C, 0xF6, 0xC5)),
            ])
            .finish(),
    )
    .shadow(
        Shadow::new()
            .offset(Point::new(4, 4))
            .blur(10)
            .color(Rgba8888::rgba(0x0A, 0x14, 0x1F, 0x40))
            .finish(),
    )
    .finish();

circle.draw(&mut rasterizer);
```

### 6.3 Arc (`Arc`)
- **Required:** `center(Point)`, `radius(Radius::ring(inner, outer))`, and `angles(Angles)`.
- **Optional:** `stroke`, `fill`, and `clip`.
- **Behaviour:** Produces annular segments based on degree-specified start and end angles.

```rust
let arc = Arc::new()
    .center(Point::new(160, 120))
    .radius(Radius::ring(36, 72))
    .angles(Angles::new(Angle::from_degrees(40), Angle::from_degrees(220)))
    .stroke(Stroke::solid(8, Rgba8888::rgb(0x1C, 0x75, 0xBC)))
    .fill(
        Fill::gradient()
            .kind(GradientKind::Conic)
            .center(Point::new(160, 120))
            .start_angle(Angle::from_degrees(40))
            .stops([
                GradientStop::new(0.0, Rgba8888::rgb(0xFC, 0xD0, 0x3F)),
                GradientStop::new(1.0, Rgba8888::rgb(0xF9, 0x6D, 0x00)),
            ])
            .finish(),
    )
    .finish();

arc.draw(&mut rasterizer);
```

### 6.4 Line (`Line`)
- **Required:** `vertices(Vertices::new(p1, p2))`.
- **Optional:** `stroke`, `caps`, `dash`, and `clip`.
- **Behaviour:** Supports axis-aligned and diagonal lines with consistent stroke, dash, and cap semantics.

```rust
let line = Line::new()
    .vertices(Vertices::new(Point::new(28, 220), Point::new(212, 52)))
    .stroke(
        Stroke::new()
            .width(3)
            .gradient(
                Gradient::linear()
                    .axis(Axis::Horizontal)
                    .stops([
                        GradientStop::new(0.0, Rgba8888::rgb(0x4A, 0x90, 0xE2)),
                        GradientStop::new(1.0, Rgba8888::rgb(0x50, 0xE3, 0xC2)),
                    ])
                    .finish(),
            )
            .finish(),
    )
    .caps(LineCaps::round())
    .dash(DashPattern::new(&[DashUnit::pixels(8), DashUnit::pixels(4)]))
    .clip(Area::from_corners(Point::new(40, 112), Point::new(200, 244)))
    .finish();

line.draw(&mut rasterizer);
```

### 6.5 Triangle (`Triangle`)
- **Required:** `vertices(Vertices::new3(a, b, c))`.
- **Optional:** `stroke`, `fill`, and `clip`.
- **Behaviour:** Provides filled and stroked triangles with precise edge handling consistent with other primitives.

```rust
let triangle = Triangle::new()
    .vertices(
        Vertices::new3(
            Point::new(64, 32),
            Point::new(208, 96),
            Point::new(72, 168),
        ),
    )
    .stroke(Stroke::solid(2, Rgba8888::rgb(0x11, 0x28, 0x2F)))
    .fill(Fill::solid(Rgba8888::rgb(0x3B, 0x8F, 0xC0)))
    .finish();

triangle.draw(&mut rasterizer);
```

### 6.6 Label (`Label`)
- **Required:** `origin(Point)` and `content(LabelContentPlan)`.
- **Optional:** `alignment`, `decor`, `spacing`, `opacity`, `color`, and `clip`.
- **Behaviour:** Configures text layout, decoration, and positioning through a fluent interface.

```rust
let label = Label::new()
    .origin(Point::new(142, 310))
    .content(LabelContentPlan::text(FontHandle::named("Inter-Medium", 24), "Step Counter"))
    .alignment(LabelAlignment::centered())
    .decor(LabelDecor::underline())
    .spacing(LabelSpacing::new(2, 4))
    .opacity(LabelOpacity::new(Opa::new(200)))
    .color(Rgba8888::rgba(0x22, 0x2E, 0x38, 0xFF))
    .finish();

label.draw(&mut rasterizer);
```

### 6.7 Vector (`Vector`)
- **Required:** `path(PathPlan)` and `winding(WindingPlan)`.
- **Optional:** `stroke`, `fill`, and `clip`.
- **Behaviour:** Renders vector paths with the configured winding rule, fill, and stroke options. No matrix transforms are applied.

```rust
let vector = Vector::new()
    .path(PathPlan::from_svg(include_str!("icons/compass.svg")))
    .winding(WindingPlan::non_zero())
    .stroke(Stroke::solid(2, Rgba8888::rgb(0x22, 0x22, 0x22)))
    .fill(
        Fill::gradient()
            .kind(GradientKind::Radial)
            .center(Point::new(96, 96))
            .radius(140)
            .stops([
                GradientStop::new(0.0, Rgba8888::rgb(0xFF, 0xF3, 0xD7)),
                GradientStop::new(1.0, Rgba8888::rgb(0xF4, 0x8C, 0x06)),
            ])
            .finish(),
    )
    .finish();

vector.draw(&mut rasterizer);
```

## 7. Style Vocabulary
- **Stroke:** Captures width, color, opacity, cap, join, dash pattern, and optional gradients. Reused across primitives, including outlines.
- **Fill:** Supports solid colors and gradient variants through `GradientKind` (linear, radial, conic) with arbitrary stop tables.
- **Shadow:** Provides drop-shadow configuration with offset, blur radius, and color.
- **Outline:** Wraps stroke-like configuration with additional padding controls.
- Each style exposes a fluent builder (`Stroke::new()…finish()`, `Fill::gradient()…finish()`) to ensure consistent usage.

## 8. Clipping and Constraints
- `clip(Area)` bounds rendering to a rectangular region; pixels outside the area are discarded.
- Builders reject invalid geometry (negative sizes, inverted corners) at configuration time.
- Transforms remain out of scope; all calculations assume axis-aligned screen space coordinates.

## 9. Error Handling
- Builders validate input eagerly and surface configuration issues as early as possible.
- Rasterizer integrations must document any unsupported features so callers can adapt earlier in the pipeline.

## 10. Non-Goals
- No support for general affine transforms or arbitrary clipping paths.
- No widget orchestration, animation scheduling, or state management.
- No dedicated memory-management strategy beyond what the builders already require.

## 11. Future Considerations
- Optional matrix transform support once primitive pipelines accept transformed geometry.
- Additional clipping primitives (rounded rect, arbitrary masks) if mask infrastructure evolves.
- Integration helpers for higher-level UI frameworks while keeping the core API focused on drawing.



