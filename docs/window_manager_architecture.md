# Window Manager Architecture

This document describes the new window management system for Better OS, replacing the previous monolithic compositor with a modular, app-based architecture.

## Overview

The system consists of five main components:

1. **Window Manager** - Manages windows and their frame buffers
2. **Surface** - Abstraction layer for drawing operations
3. **App Shell** - Spawns and manages application lifecycle
4. **Compositor** - Composites windows with animations
5. **Demo Apps** - Example applications demonstrating the system

## Architecture

### Window Manager ([window_manager.rs](panzer/src/system/window_manager.rs))

The window manager is responsible for:
- Creating and destroying windows
- Assigning unique IDs to each window
- Managing window frame buffers
- Tracking window metadata (name, geometry, visibility, z-order)

```rust
// Create a window
let window_id = window_manager.create_window(
    "My Window".to_string(),
    WindowGeometry { x: 0, y: 0, width: 240, height: 240 },
    2, // bytes per pixel (RGB565)
);

// Access window
if let Some(window) = window_manager.get_window_mut(window_id) {
    // Work with window frame buffer
}
```

### Surface ([surface.rs](panzer/src/system/surface.rs))

Surfaces wrap frame buffers and implement the `RasterTarget` trait from the gfx crate:

```rust
// Create a surface from window frame buffer
let mut surface = Surface::new_rgb565(
    &mut window.frame_buffer,
    width,
    height,
);

// Draw to surface
surface.clear(Color::rgba(0, 0, 0, 255));
let mut rasterizer = surface.rasterizer();
Rectangle::new()
    .bounds(bounds)
    .fill(FillStyle::Solid(color))
    .draw(&mut rasterizer);
```

### App Shell ([app_shell.rs](panzer/src/system/app_shell.rs))

The app shell manages application lifecycle:

```rust
// Define an app
struct MyApp { /* ... */ }

impl App for MyApp {
    fn init(&mut self, surface: &mut Surface) {
        // Initialize with surface
    }
    
    fn update(&mut self, surface: &mut Surface, delta_ms: u32) {
        // Draw to surface each frame
    }
    
    fn name(&self) -> &str { "My App" }
}

// Spawn the app
let app_id = app_shell.spawn_app(
    "My App".to_string(),
    Box::new(MyApp::new()),
    &mut window_manager,
    window_geometry,
);
```

Apps receive a `Surface` that wraps their window's frame buffer, allowing them to draw without knowledge of the underlying buffer management.

### Compositor ([compositor.rs](panzer/src/system/compositor.rs))

The compositor combines windows and renders them to the display with animations:

```rust
// Create compositor
let mut compositor = Compositor::new();

// Switch windows with animation
compositor.switch_to_window(
    window_id,
    TransitionType::Fade,  // or SlideLeft, SlideRight, etc.
    500,                   // duration in ms
    Easing::EaseInOut,
);

// Composite to display
compositor.composite(&mut display_rasterizer, &window_manager);
```

Supported transitions:
- **Fade** - Fade in/out between windows
- **SlideLeft/Right/Top/Bottom** - Slide transitions
- **Scale** - Scale/zoom effect
- **None** - Instant switch

### Demo Apps ([demo_apps.rs](panzer/src/system/demo_apps.rs))

Three demo applications showcase the system:

1. **ShapesDemo** - Animated shapes and borders
2. **GradientDemo** - Animated color gradients
3. **InfoDemo** - UI mockup with title bar and content

Each app:
- Takes a `Surface` as argument
- Implements the `App` trait
- Draws to its surface each frame
- Manages its own state independently

## Integration

### New Compositor Service

The `window_compositor_service` task in [compositor_srv.rs](panzer/src/system/tasks/compositor_srv.rs) integrates all components:

```rust
#[embassy_executor::task]
pub async fn window_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    // Initialize components
    let mut window_manager = WindowManager::new();
    let mut app_shell = AppShell::new();
    let mut compositor = Compositor::new();
    
    // Spawn demo apps
    app_shell.spawn_app(...);
    
    // Main loop
    loop {
        // Update apps (they draw to their surfaces)
        app_shell.update_apps(&mut window_manager, delta_ms);
        
        // Composite windows to display
        compositor.composite(&mut display_rasterizer, &window_manager);
        
        // Draw to physical display
        display_facade.draw_full(&buffer).await;
    }
}
```

### Usage in Main

To use the new system, spawn the window compositor service:

```rust
spawner.spawn(window_compositor_service(display_static)).unwrap();
```

The old `ui_compositor_service` is still available but marked as deprecated.

## Key Benefits

1. **Modularity** - Clear separation between window management, apps, and compositing
2. **Encapsulation** - Apps work with surfaces, not raw buffers
3. **Flexibility** - Easy to add new apps and transition effects
4. **Testability** - Components can be tested independently
5. **Extensibility** - Foundation for future features (input handling, window decorations, etc.)

## Frame Buffer Format

Windows use RGB565 format by default (2 bytes per pixel):
- 5 bits red (bits 15-11)
- 6 bits green (bits 10-5)  
- 5 bits blue (bits 4-0)
- Big-endian byte order

LUMA4 (4-bit grayscale) is also supported for memory-constrained scenarios.

## Performance Considerations

- Windows are composited only when dirty or during transitions
- Frame buffers are allocated on the heap using `Box<[u8]>`
- Compositor uses optimized blitting for pixel transfer
- Apps update independently at their own pace

## Future Enhancements

Potential improvements:
- Input event routing to apps
- Window decorations (title bars, borders)
- Multi-window tiling and overlapping
- Hardware acceleration for compositing
- More transition effects
- Window focus management
- Inter-app communication

## Files Created

- `panzer/src/system/window_manager.rs` - Window management
- `panzer/src/system/surface.rs` - Surface abstraction
- `panzer/src/system/app_shell.rs` - App lifecycle management
- `panzer/src/system/compositor.rs` - Compositing with animations
- `panzer/src/system/demo_apps.rs` - Example applications

## Files Modified

- `panzer/src/system/mod.rs` - Added new module declarations
- `panzer/src/system/tasks/compositor_srv.rs` - Added new compositor service
