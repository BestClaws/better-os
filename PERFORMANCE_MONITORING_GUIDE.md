# Performance Monitoring Guide for Watch App

## Overview

The watch app now includes comprehensive performance monitoring to identify bottlenecks and ensure smooth 60fps animations. This guide explains how to interpret the performance logs and optimize accordingly.

## Performance Metrics Structure

### Frame-Level Metrics
- **FPS**: Frames per second (target: 60fps)
- **Frame Duration**: Total time per frame (target: <16ms)
- **Total Render Time**: Canvas drawing time
- **Compositor Time**: UI composition and display update time

### Component-Level Metrics
- **Background**: Atmosphere, grid, and particle rendering
- **Outer Ring**: Metallic sweep and marker rendering
- **Hour Markers**: Hour and minute tick rendering
- **3D Orb**: Gradient, 3D model, and scanline rendering
- **Hands**: Hour, minute, second hand, and trail rendering
- **Overlays**: HUD elements, arcs, and label area

### Dirty Region Analysis
- **Dirty Regions Count**: Number of regions needing redraw
- **Dirty Regions Area**: Total pixels needing redraw

## Log Output Examples

### Debug Logs (Every Frame)
```
Background: 2ms
Outer ring: 1ms
Hour markers: 1ms
3D Orb: 8ms
Hands: 3ms
Overlays: 1ms
Compositor: 1ms
Dirty regions: 3 (1200px)
```

### Performance Summary (Every 60 Frames)
```
=== PERFORMANCE SUMMARY ===
FPS: 60, Frame: 16ms
Background: 2ms, Ring: 1ms, Markers: 1ms
3D Orb: 8ms, Hands: 3ms, Overlays: 1ms
Compositor: 1ms, Dirty: 3 regions (1200px)
```

### Performance Warnings
```
SLOW FRAME: 20ms (target: 16ms for 60fps)
SLOW 3D: 12ms (consider reducing model complexity)
MANY DIRTY REGIONS: 15 (consider batching)
FRAME DROP: 18ms (target: 16ms for 60fps)
```

## Performance Targets

### Frame Rate Targets
- **Target FPS**: 60fps (16.67ms per frame)
- **Acceptable FPS**: 30fps (33.33ms per frame)
- **Critical**: <30fps (smooth animation lost)

### Component Time Targets
- **Background**: <3ms (gradients and particles)
- **Outer Ring**: <2ms (metallic effects)
- **Hour Markers**: <2ms (simple geometry)
- **3D Orb**: <8ms (most complex component)
- **Hands**: <4ms (anti-aliased lines)
- **Overlays**: <2ms (simple shapes)
- **Compositor**: <2ms (display update)

### Dirty Region Targets
- **Optimal**: 1-5 regions per frame
- **Acceptable**: 6-10 regions per frame
- **Warning**: >10 regions (consider batching)

## Optimization Strategies

### If 3D Orb is Slow (>8ms)
1. **Reduce Model Complexity**: Use fewer vertices in STL model
2. **Disable Anti-aliasing**: Set `aa_mode: AntiAliasing::None`
3. **Simplify Shading**: Use `ShadingMode::Flat` instead of Gouraud
4. **Reduce Scanlines**: Fewer holographic scanlines
5. **Lower Resolution**: Render at lower resolution, scale up

### If Hands are Slow (>4ms)
1. **Reduce Trail Segments**: Fewer second hand trail segments
2. **Simplify Anti-aliasing**: Use simpler line drawing
3. **Optimize Hand Thickness**: Thinner hands = fewer pixels

### If Background is Slow (>3ms)
1. **Reduce Particles**: Fewer floating particles
2. **Simplify Gradients**: Use fewer gradient stops
3. **Optimize Grid**: Fewer grid lines

### If Too Many Dirty Regions (>10)
1. **Enable Batching**: Use `begin_drawing_batch()` for complex operations
2. **Reduce Animation Frequency**: Animate fewer elements per frame
3. **Optimize Dirty Region Coalescing**: Ensure regions merge properly

### If Frame Rate is Low (<60fps)
1. **Identify Slowest Component**: Focus on the component with highest time
2. **Reduce Visual Complexity**: Simplify effects and animations
3. **Optimize Algorithms**: Use faster rendering algorithms
4. **Profile Memory Usage**: Ensure no memory allocation in hot path

## Performance Monitoring Commands

### Enable Debug Logging
```rust
// In your main.rs or config
defmt::set_max_level(defmt::LevelFilter::Debug);
```

### Monitor Specific Components
```rust
// Add custom timing for specific operations
let custom_start = Instant::now();
// ... your custom operation ...
let custom_time = custom_start.elapsed();
debug!("Custom operation: {}ms", custom_time.as_millis());
```

### Performance Profiling
```rust
// Profile a specific function
fn profile_function<F, R>(name: &str, f: F) -> R 
where 
    F: FnOnce() -> R 
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    debug!("{}: {}ms", name, elapsed.as_millis());
    result
}
```

## Common Performance Issues

### 1. Frame Drops
**Symptoms**: `FRAME DROP: 20ms` warnings
**Causes**: 
- Complex 3D rendering
- Too many dirty regions
- Memory allocation in hot path
- Blocking operations

**Solutions**:
- Optimize slowest component
- Use batched operations
- Pre-allocate buffers
- Use async operations

### 2. High 3D Rendering Time
**Symptoms**: `SLOW 3D: 12ms` warnings
**Causes**:
- Complex STL model
- Expensive shading calculations
- Too many triangles
- Inefficient projection

**Solutions**:
- Simplify 3D model
- Use flat shading
- Reduce triangle count
- Optimize projection matrix

### 3. Many Dirty Regions
**Symptoms**: `MANY DIRTY REGIONS: 15` warnings
**Causes**:
- Scattered pixel updates
- No batching
- Inefficient region coalescing

**Solutions**:
- Use `begin_drawing_batch()`
- Group related operations
- Optimize region merging

### 4. Inconsistent Frame Times
**Symptoms**: Varying frame times (8ms, 25ms, 12ms, 30ms)
**Causes**:
- Conditional rendering paths
- Memory allocation spikes
- Garbage collection
- Interrupt handling

**Solutions**:
- Pre-allocate all buffers
- Use consistent rendering paths
- Minimize dynamic allocation
- Profile interrupt handlers

## Performance Testing

### Stress Testing
```rust
// Test with maximum complexity
let stress_test = true;
if stress_test {
    // Enable all effects
    // Use complex 3D model
    // Add more particles
    // Increase animation complexity
}
```

### Benchmarking
```rust
// Run for 1000 frames and measure average
let mut total_time = Duration::from_millis(0);
let frame_count = 1000;
for _ in 0..frame_count {
    let start = Instant::now();
    // ... render frame ...
    total_time += start.elapsed();
}
let avg_frame_time = total_time / frame_count;
info!("Average frame time: {}ms", avg_frame_time.as_millis());
```

## Conclusion

The performance monitoring system provides detailed insights into rendering performance. Use the logs to:

1. **Identify Bottlenecks**: Focus on the slowest components
2. **Optimize Systematically**: Address issues in order of impact
3. **Maintain Smooth Animation**: Keep frame times under 16ms
4. **Monitor Trends**: Watch for performance degradation over time

With proper monitoring and optimization, the watch app can maintain smooth 60fps animation while providing rich visual effects suitable for space-grade embedded applications.
