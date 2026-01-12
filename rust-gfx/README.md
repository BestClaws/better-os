# rust-gfx

A pure Rust, no_std graphics library for embedded systems with RGB565 framebuffer support. Optimized for ESP32 and other resource-constrained devices, but can also run on desktop for testing.

## Features

- **no_std compatible** - Works on embedded systems without standard library
- **RGB565 rasterizer** - Efficient 16-bit color rendering
- **Anti-aliased primitives** - Smooth circles, arcs, rounded rectangles, and lines
- **Flexible fill styles** - Solid colors, linear gradients (H/V), and radial gradients
- **Configurable strokes** - Variable width stroke support with anti-aliasing
- **3D rendering** - Basic 3D model rendering with STL support
- **Platform agnostic** - Trait-based design allows custom rasterizer backends

## Shapes

### Circle
```rust
use rust_gfx::{Circle, Rgba8888, Rgb565Rasterizer, Shape};

let mut rasterizer = Rgb565Rasterizer::new(240, 240);

// Filled circle
Circle::new(120, 120, 50)
    .fill_solid(Rgba8888::rgb(255, 0, 0))
    .draw(&mut rasterizer);

// Stroked circle
Circle::new(120, 120, 50)
    .stroke(3, Rgba8888::rgb(0, 255, 0))
    .draw(&mut rasterizer);

// Radial gradient fill
Circle::new(120, 120, 50)
    .fill_radial(Rgba8888::rgb(255, 255, 255), Rgba8888::rgb(0, 0, 255))
    .draw(&mut rasterizer);
```

### Rounded Rectangle
```rust
use rust_gfx::{RoundedRect, Rgba8888, Shape};

// All corners with radius 10
RoundedRect::new(10, 10, 100, 60, 10, 10, 10, 10)
    .fill_solid(Rgba8888::rgb(0, 255, 0))
    .draw(&mut rasterizer);

// With gradient
RoundedRect::new(10, 10, 100, 60, 10, 10, 10, 10)
    .fill_linear_h(Rgba8888::rgb(255, 0, 0), Rgba8888::rgb(0, 0, 255))
    .draw(&mut rasterizer);
```

### Arc
```rust
use rust_gfx::{Arc, Rgba8888, Shape};

// Draw 270° arc
Arc::new(120, 120, 50, 0, 270)
    .stroke(5, Rgba8888::rgb(255, 0, 255))
    .draw(&mut rasterizer);
```

### Line
```rust
use rust_gfx::{Line, Rgba8888, Shape};

Line::new(10, 10, 200, 200)
    .stroke(2, Rgba8888::rgb(255, 255, 255))
    .draw(&mut rasterizer);
```

## Running Tests on PC

The library can be compiled and tested on desktop systems:

```bash
cd rust-gfx
cargo run --example render_test
```

This generates `output.png` with various rendered shapes for quality verification.

## Using in Embedded Projects

Add to your `Cargo.toml`:

```toml
[dependencies]
rust-gfx = { path = "../rust-gfx" }
```

Then implement the `Rasterizer` trait for your display driver, or use the built-in `Rgb565Rasterizer`.

## Architecture

- **color.rs** - Rgba8888 color type
- **rasterizer.rs** - Trait defining the rendering interface + RGB565 implementation
- **shapes/** - Circle, Arc, RoundedRect, Line implementations
- **fill.rs** - Fill and stroke style definitions
- **three_d/** - 3D rendering (model, STL parsing, projection)

## License

Part of the better-os project.
