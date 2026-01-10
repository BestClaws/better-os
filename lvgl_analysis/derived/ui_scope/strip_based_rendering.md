# Strip-Based Partial Rendering

**Focus:** Pure rendering architecture for memory-constrained systems  
**Date:** 11 January 2026

---

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [The Rendering Problem](#the-rendering-problem)
4. [Strip-Based Solution](#strip-based-solution)
5. [Detailed Walkthrough](#detailed-walkthrough)
6. [Coordinate Systems](#coordinate-systems)
7. [Task Lifecycle](#task-lifecycle)
8. [Implementation Details](#implementation-details)
9. [Memory Analysis](#memory-analysis)

---

## Overview

**Strip-based rendering** solves the problem of rendering to displays larger than available RAM by dividing the screen into horizontal strips and rendering each strip sequentially using a single reusable buffer.

### Key Principle

Instead of allocating a full-screen buffer, allocate a smaller buffer and render the screen in multiple passes, updating the buffer metadata to represent different screen regions on each pass.

```
Full Screen Buffer: 240×240×2 = 115,200 bytes (115 KB)
Strip Buffer:       240×100×2 =  48,000 bytes (48 KB)
                                  
Savings: 67 KB (58% reduction)
```

---

## Core Concepts

### 1. Area (Rectangle)

A rectangle defined by inclusive coordinates:

```rust
pub struct Area {
    pub x1: i32,  // Left edge (inclusive)
    pub y1: i32,  // Top edge (inclusive)
    pub x2: i32,  // Right edge (inclusive)
    pub y2: i32,  // Bottom edge (inclusive)
}

impl Area {
    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }
    
    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }
    
    pub fn intersects(&self, other: &Area) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 &&
        self.y1 <= other.y2 && self.y2 >= other.y1
    }
}
```

**Example:**
```rust
let rect = Area { x1: 10, y1: 20, x2: 50, y2: 60 };
// Covers pixels from (10,20) to (50,60)
// Width: 41 pixels
// Height: 41 pixels
```

### 2. Buffer

Raw pixel memory - a flat array of bytes:

```rust
pub struct Buffer {
    data: *mut [u8],
    width: u32,
    height: u32,
    stride: usize,           // Bytes per row
    color_format: ColorFormat,
}
```

**Layout (RGB565):**
```
Row 0: [pixel0][pixel1][pixel2]...[pixel239]  (480 bytes)
Row 1: [pixel0][pixel1][pixel2]...[pixel239]  (480 bytes)
...
Row 99: [pixel0][pixel1][pixel2]...[pixel239] (480 bytes)

Total: 100 rows × 480 bytes = 48,000 bytes
```

**Pixel addressing:**
```rust
fn pixel_offset(x: u32, y: u32, stride: usize) -> usize {
    y as usize * stride + x as usize * 2  // 2 bytes per pixel for RGB565
}
```

### 3. Layer

Rendering surface metadata - tracks what region the buffer represents:

```rust
pub struct Layer {
    buffer: *mut Buffer,        // The pixel buffer
    buf_area: Area,             // Screen region this buffer represents
    clip_area: Area,            // Current clipping rectangle
    partial_y_offset: i32,      // Y offset for partial rendering
    color_format: ColorFormat,
    opacity: Opacity,
}
```

**Critical field: `buf_area`**

This tells all drawing operations: "The buffer represents THIS region of the screen."

```rust
// Strip 1
layer.buf_area = Area { x1: 0, y1: 0, x2: 239, y2: 99 };
// "Buffer pixel[0][0] corresponds to screen pixel (0, 0)"
// "Buffer pixel[239][99] corresponds to screen pixel (239, 99)"

// Strip 2
layer.buf_area = Area { x1: 0, y1: 100, x2: 239, y2: 199 };
// "Buffer pixel[0][0] NOW corresponds to screen pixel (0, 100)"
// "Buffer pixel[239][99] NOW corresponds to screen pixel (239, 199)"
```

### 4. Task

A drawing operation with screen-space coordinates:

```rust
pub struct DrawTask {
    task_type: TaskType,
    area: Area,              // Screen coordinates (absolute)
    target_layer: *mut Layer,
    descriptor: DrawDescriptor,
    state: TaskState,
}

pub enum TaskType {
    Fill,       // Solid rectangle
    Border,     // Rectangle outline
    Line,       // Line segment
    Arc,        // Arc/circle
    Triangle,   // Triangle
    Image,      // Blit image
    Text,       // Render text
}
```

**Key point:** `task.area` is always in **screen coordinates**, never buffer coordinates.

---

## The Rendering Problem

### Scenario

```
Display: 240×240 pixels
Color format: RGB565 (2 bytes per pixel)
Available RAM: 64 KB for framebuffer
```

**Full screen buffer:**
```
240 × 240 × 2 = 115,200 bytes (112.5 KB) ❌ Too much!
```

**Drawing requests:**
```rust
let rect_top = DrawRequest {
    type: Fill,
    area: Area { x1: 10, y1: 10, x2: 60, y2: 60 },
    color: BLUE,
};

let triangle_bottom = DrawRequest {
    type: Triangle,
    area: Area { x1: 180, y1: 200, x2: 230, y2: 250 },
    color: RED,
};
```

**Problem:** Rectangle at top (y=10-60), triangle at bottom (y=200-250). Can't fit entire screen in buffer.

---

## Strip-Based Solution

### Approach

1. Allocate buffer for **part** of screen (e.g., 100 rows)
2. Divide screen into horizontal **strips**
3. For each strip:
   - Update layer metadata to map buffer to current strip
   - Create tasks for drawing operations that intersect strip
   - Execute tasks (they write to buffer using coordinate mapping)
   - Flush buffer to display
   - Reuse buffer for next strip

### Memory Usage

```
Strip buffer: 240 × 100 × 2 = 48,000 bytes (47 KB) ✅ Fits!

Strips needed: ceil(240 / 100) = 3 strips
  Strip 1: rows 0-99
  Strip 2: rows 100-199
  Strip 3: rows 200-239
```

---

## Detailed Walkthrough

### Setup

```rust
// Screen dimensions
const SCREEN_WIDTH: u32 = 240;
const SCREEN_HEIGHT: u32 = 240;

// Buffer (partial)
const STRIP_HEIGHT: u32 = 100;
let mut buffer = vec![0u8; (SCREEN_WIDTH * STRIP_HEIGHT * 2) as usize];

// Layer
let mut layer = Layer {
    buffer: &mut buffer,
    buf_area: Area::ZERO,      // Will be updated per strip
    clip_area: Area::ZERO,     // Will be updated per strip
    partial_y_offset: 0,
    color_format: ColorFormat::RGB565,
    opacity: Opacity::OPAQUE,
};

// Drawing requests (in screen coordinates)
let rect_request = DrawRequest {
    area: Area { x1: 10, y1: 10, x2: 60, y2: 60 },
    color: BLUE,
};

let triangle_request = DrawRequest {
    area: Area { x1: 180, y1: 200, x2: 230, y2: 250 },
    color: RED,
};
```

---

### Strip 1: Rows 0-99

#### Step 1: Configure Layer

```rust
layer.buf_area = Area {
    x1: 0,
    y1: 0,
    x2: 239,
    y2: 99,
};
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 0;
```

**Interpretation:**
- Buffer position `[0][0]` = screen position `(0, 0)`
- Buffer position `[239][99]` = screen position `(239, 99)`
- Any drawing at screen Y=50 writes to buffer row 50

#### Step 2: Task Creation (Intersection Test)

```rust
let mut tasks = Vec::new();

// Check rectangle
if rect_request.area.intersects(&layer.clip_area) {
    // Area {10, 10, 60, 60} intersects {0, 0, 239, 99} → YES
    let task = DrawTask {
        task_type: TaskType::Fill,
        area: rect_request.area,  // {10, 10, 60, 60}
        target_layer: &mut layer,
        descriptor: FillDescriptor { color: BLUE },
        state: TaskState::WAITING,
    };
    tasks.push(task);
}

// Check triangle
if triangle_request.area.intersects(&layer.clip_area) {
    // Area {180, 200, 230, 250} intersects {0, 0, 239, 99} → NO
    // Triangle is below this strip - skip
}
```

**Result:** 1 task created (rectangle)

#### Step 3: Execute Tasks

```rust
for task in tasks {
    match task.task_type {
        TaskType::Fill => {
            execute_fill(task, &mut layer);
        }
        _ => {}
    }
}

fn execute_fill(task: &DrawTask, layer: &mut Layer) {
    let screen_area = task.area;  // {10, 10, 60, 60}
    let buf_area = layer.buf_area;  // {0, 0, 239, 99}
    
    // Map screen Y to buffer Y
    let buffer_y_start = screen_area.y1 - buf_area.y1;  // 10 - 0 = 10
    let buffer_y_end = screen_area.y2 - buf_area.y1;    // 60 - 0 = 60
    
    let buffer_x_start = screen_area.x1;  // 10
    let buffer_x_end = screen_area.x2;    // 60
    
    // Get color pixel value
    let pixel_value = rgb565_from_color(task.descriptor.color);
    
    // Write pixels
    for buffer_y in buffer_y_start..=buffer_y_end {
        for buffer_x in buffer_x_start..=buffer_x_end {
            let offset = (buffer_y * 240 + buffer_x) * 2;
            layer.buffer[offset] = (pixel_value & 0xFF) as u8;
            layer.buffer[offset + 1] = (pixel_value >> 8) as u8;
        }
    }
}
```

**Buffer state after execution:**
```
Rows 0-9:   [background pixels]
Rows 10-60: [BLUE pixels from x=10 to x=60]
Rows 61-99: [background pixels]
```

#### Step 4: Flush to Display

```rust
flush_to_display(&layer.buffer, &layer.buf_area);
// Sends buffer to display controller
// Display controller writes to physical screen rows 0-99
```

**Screen state:**
```
Rows 0-99:   ✅ Rendered (blue rectangle visible)
Rows 100-239: ⏳ Not yet rendered
```

---

### Strip 2: Rows 100-199

#### Step 1: Configure Layer

```rust
layer.buf_area = Area {
    x1: 0,
    y1: 100,  // ← Changed
    x2: 239,
    y2: 199,  // ← Changed
};
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 100;  // ← Track offset
```

**Interpretation:**
- Buffer position `[0][0]` NOW = screen position `(0, 100)`
- Buffer position `[239][99]` NOW = screen position `(239, 199)`
- Any drawing at screen Y=150 writes to buffer row: 150 - 100 = 50

#### Step 2: Task Creation

```rust
let mut tasks = Vec::new();

// Check rectangle
if rect_request.area.intersects(&layer.clip_area) {
    // Area {10, 10, 60, 60} intersects {0, 100, 239, 199} → NO
    // Rectangle is above this strip - skip
}

// Check triangle
if triangle_request.area.intersects(&layer.clip_area) {
    // Area {180, 200, 230, 250} intersects {0, 100, 239, 199} → NO
    // Triangle bottom edge (200) is at strip boundary, but no overlap in range 100-199
}
```

**Result:** 0 tasks created

#### Step 3: Execute Tasks

```rust
// No tasks to execute
// Buffer contains background color only
```

#### Step 4: Flush to Display

```rust
flush_to_display(&layer.buffer, &layer.buf_area);
// Sends background-only buffer to screen rows 100-199
```

**Screen state:**
```
Rows 0-99:    ✅ Rendered (blue rectangle)
Rows 100-199: ✅ Rendered (background only)
Rows 200-239: ⏳ Not yet rendered
```

---

### Strip 3: Rows 200-239

#### Step 1: Configure Layer

```rust
layer.buf_area = Area {
    x1: 0,
    y1: 200,  // ← Changed
    x2: 239,
    y2: 239,  // ← Changed (only 40 rows in this strip)
};
layer.clip_area = layer.buf_area;
layer.partial_y_offset = 200;
```

**Interpretation:**
- Buffer position `[0][0]` NOW = screen position `(0, 200)`
- Buffer position `[239][39]` NOW = screen position `(239, 239)`
- Note: Only using first 40 rows of 100-row buffer

#### Step 2: Task Creation

```rust
let mut tasks = Vec::new();

// Check rectangle
if rect_request.area.intersects(&layer.clip_area) {
    // Area {10, 10, 60, 60} intersects {0, 200, 239, 239} → NO
    // Rectangle is above this strip
}

// Check triangle
if triangle_request.area.intersects(&layer.clip_area) {
    // Area {180, 200, 230, 250} intersects {0, 200, 239, 239} → YES
    // Triangle spans y=200-250, strip is y=200-239
    let task = DrawTask {
        task_type: TaskType::Triangle,
        area: triangle_request.area,  // {180, 200, 230, 250}
        target_layer: &mut layer,
        descriptor: TriangleDescriptor {
            p1: Point { x: 180, y: 200 },
            p2: Point { x: 230, y: 250 },
            p3: Point { x: 205, y: 200 },
            color: RED,
        },
        state: TaskState::WAITING,
    };
    tasks.push(task);
}
```

**Result:** 1 task created (triangle)

#### Step 3: Execute Tasks

```rust
fn execute_triangle(task: &DrawTask, layer: &mut Layer) {
    let screen_area = task.area;  // {180, 200, 230, 250}
    let buf_area = layer.buf_area;  // {0, 200, 239, 239}
    let clip = layer.clip_area;     // {0, 200, 239, 239}
    
    // Triangle vertices (screen coordinates)
    let p1 = task.descriptor.p1;  // (180, 200)
    let p2 = task.descriptor.p2;  // (230, 250)
    let p3 = task.descriptor.p3;  // (205, 200)
    
    // Rasterize triangle scanline by scanline
    for screen_y in screen_area.y1..=screen_area.y2 {
        // Check if scanline is in current strip
        if screen_y < clip.y1 || screen_y > clip.y2 {
            continue;  // Scanline y=240-250 clipped
        }
        
        // Map screen Y to buffer Y
        let buffer_y = screen_y - buf_area.y1;
        // screen_y=200 → buffer_y=0
        // screen_y=239 → buffer_y=39
        // screen_y=250 → would be 50, but clipped above
        
        // Calculate triangle edges at this scanline
        let (x_left, x_right) = calculate_triangle_edges(p1, p2, p3, screen_y);
        
        // Draw scanline
        for screen_x in x_left..=x_right {
            if screen_x >= clip.x1 && screen_x <= clip.x2 {
                let buffer_x = screen_x;  // No X offset in partial rendering
                let offset = (buffer_y * 240 + buffer_x) * 2;
                
                let pixel_value = rgb565_from_color(RED);
                layer.buffer[offset] = (pixel_value & 0xFF) as u8;
                layer.buffer[offset + 1] = (pixel_value >> 8) as u8;
            }
        }
    }
}
```

**Buffer state after execution:**
```
Rows 0-39:  [RED triangle pixels (partial, top 40 rows only)]
             (Triangle extends to row 50 in screen coords, 
              but we only render rows 200-239 = buffer rows 0-39)
Rows 40-99: [unused - buffer taller than strip]
```

#### Step 4: Flush to Display

```rust
flush_to_display(&layer.buffer, &layer.buf_area);
// Only send first 40 rows (what we actually rendered)
// Display controller writes to physical screen rows 200-239
```

**Screen state:**
```
Rows 0-99:    ✅ Rendered (blue rectangle)
Rows 100-199: ✅ Rendered (background)
Rows 200-239: ✅ Rendered (red triangle, clipped at bottom)
```

**Final result:** Complete screen rendered with 48 KB buffer instead of 115 KB.

---

## Coordinate Systems

### Screen Space (Absolute)

All drawing requests use screen coordinates:

```rust
// Rectangle in screen space
rect.area = Area { x1: 10, y1: 10, x2: 60, y2: 60 };

// This means:
// "Draw from pixel (10, 10) to pixel (60, 60) on the physical screen"
```

**Screen space never changes** - it's the fixed coordinate system of the display.

### Buffer Space (Relative)

Buffer coordinates are relative to buffer start:

```rust
// Buffer dimensions
buffer: 240 × 100 pixels

// Buffer space coordinates
buffer[y][x] where:
  x: 0 to 239
  y: 0 to 99
```

**Buffer space is reused** - the same buffer represents different screen regions.

### Coordinate Mapping

The key to strip rendering is mapping between coordinate systems:

```rust
// Formula
buffer_y = screen_y - layer.buf_area.y1
buffer_x = screen_x - layer.buf_area.x1  // Usually 0 for horizontal strips
```

**Examples:**

**Strip 1: buf_area.y1 = 0**
```
screen_y = 50  → buffer_y = 50 - 0 = 50
screen_y = 10  → buffer_y = 10 - 0 = 10
```

**Strip 2: buf_area.y1 = 100**
```
screen_y = 150 → buffer_y = 150 - 100 = 50
screen_y = 100 → buffer_y = 100 - 100 = 0
screen_y = 199 → buffer_y = 199 - 100 = 99
```

**Strip 3: buf_area.y1 = 200**
```
screen_y = 200 → buffer_y = 200 - 200 = 0
screen_y = 220 → buffer_y = 220 - 200 = 20
screen_y = 239 → buffer_y = 239 - 200 = 39
```

### Visual Representation

```
Screen Space          Strip 1 (buf_area.y1=0)    Strip 2 (buf_area.y1=100)
────────────          ────────────────────────   ──────────────────────────
    0  ┌────────┐          0  ┌────────┐ maps        [not in strip]
       │        │             │        │ to
   10  │  rect  │            10│  rect  │ screen
       │        │             │        │  0-99
   60  └────────┘            60└────────┘
       │        │             │        │
  100  │        │            99└────────┘         0  ┌────────┐ maps
       │        │              [end of buffer]       │        │ to
  150  │        │                                 50 │        │ screen
       │        │                                    │        │ 100-199
  200  │ /\     │                                 99 └────────┘
       │/  \    │                                   [end of buffer]
  239  └────────┘

Strip 3 (buf_area.y1=200)
────────────────────────
   [not in strip]

    0  ┌────────┐ maps to
       │ /\     │ screen
   20  │/  \    │ 200-239
       │    \   │
   39  └────────┘
      [only 40 rows used]
```

---

## Task Lifecycle

### State Machine

```
┌──────────────┐
│   CREATED    │  Task allocated with screen coordinates
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ INTERSECTION │  Test: task.area ∩ layer.clip_area
│    TEST      │
└──────┬───────┘
       │
       ├─ YES ─→ ┌──────────────┐
       │         │   WAITING    │  Added to layer's task queue
       │         └──────┬───────┘
       │                │
       │                ▼
       │         ┌──────────────┐
       │         │ IN_PROGRESS  │  Executing (writing to buffer)
       │         └──────┬───────┘
       │                │
       │                ▼
       │         ┌──────────────┐
       │         │  COMPLETED   │  Task finished
       │         └──────────────┘
       │
       └─ NO ──→ ┌──────────────┐
                 │   SKIPPED    │  Not in current strip
                 └──────────────┘
```

### Detailed Flow

#### 1. Task Creation

```rust
fn create_tasks_for_strip(layer: &Layer, requests: &[DrawRequest]) -> Vec<DrawTask> {
    let mut tasks = Vec::new();
    
    for request in requests {
        // Intersection test
        if request.area.intersects(&layer.clip_area) {
            let task = DrawTask {
                task_type: request.task_type,
                area: request.area,  // Keep screen coordinates
                target_layer: layer,
                descriptor: request.descriptor,
                state: TaskState::WAITING,
            };
            tasks.push(task);
        }
        // If no intersection, task is not created for this strip
    }
    
    tasks
}
```

#### 2. Task Execution

```rust
fn execute_task(task: &mut DrawTask, layer: &mut Layer) {
    task.state = TaskState::IN_PROGRESS;
    
    match task.task_type {
        TaskType::Fill => {
            // Get screen coordinates from task
            let screen_area = task.area;
            
            // Map to buffer coordinates
            let buf_y_start = screen_area.y1 - layer.buf_area.y1;
            let buf_y_end = screen_area.y2 - layer.buf_area.y1;
            
            // Clamp to buffer bounds
            let buf_y_start = buf_y_start.max(0);
            let buf_y_end = buf_y_end.min(layer.buffer.height as i32 - 1);
            
            // Render
            for buf_y in buf_y_start..=buf_y_end {
                for buf_x in screen_area.x1..=screen_area.x2 {
                    set_pixel(layer.buffer, buf_x, buf_y, task.descriptor.color);
                }
            }
        }
        // ... other task types
    }
    
    task.state = TaskState::COMPLETED;
}
```

#### 3. Task Cleanup

```rust
fn cleanup_tasks(tasks: Vec<DrawTask>) {
    // Tasks are dropped after strip is flushed
    // Same drawing request may create new task for next strip
    drop(tasks);
}
```

---

## Implementation Details

### Strip Loop

```rust
fn render_screen_partial(
    buffer: &mut [u8],
    screen_width: u32,
    screen_height: u32,
    strip_height: u32,
    requests: &[DrawRequest],
) {
    let mut layer = Layer {
        buffer,
        buf_area: Area::ZERO,
        clip_area: Area::ZERO,
        partial_y_offset: 0,
        color_format: ColorFormat::RGB565,
        opacity: Opacity::OPAQUE,
    };
    
    let num_strips = (screen_height + strip_height - 1) / strip_height;  // Round up
    
    for strip_index in 0..num_strips {
        let strip_y_start = strip_index * strip_height;
        let strip_y_end = ((strip_index + 1) * strip_height - 1).min(screen_height - 1);
        
        // Configure layer for this strip
        layer.buf_area = Area {
            x1: 0,
            y1: strip_y_start as i32,
            x2: (screen_width - 1) as i32,
            y2: strip_y_end as i32,
        };
        layer.clip_area = layer.buf_area;
        layer.partial_y_offset = strip_y_start as i32;
        
        // Clear buffer to background
        clear_buffer(&mut layer, Color::BLACK);
        
        // Create tasks for this strip
        let tasks = create_tasks_for_strip(&layer, requests);
        
        // Execute all tasks
        for task in tasks {
            execute_task(&task, &mut layer);
        }
        
        // Flush to display
        flush_to_display(&layer.buffer, &layer.buf_area);
    }
}
```

### Intersection Testing

```rust
impl Area {
    pub fn intersects(&self, other: &Area) -> bool {
        // Two rectangles intersect if:
        // - They overlap horizontally AND
        // - They overlap vertically
        
        let horizontal_overlap = self.x1 <= other.x2 && self.x2 >= other.x1;
        let vertical_overlap = self.y1 <= other.y2 && self.y2 >= other.y1;
        
        horizontal_overlap && vertical_overlap
    }
    
    pub fn intersection(&self, other: &Area) -> Option<Area> {
        if !self.intersects(other) {
            return None;
        }
        
        Some(Area {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        })
    }
}
```

### Clipping

```rust
fn render_with_clipping(
    screen_area: &Area,
    clip_area: &Area,
    buf_area: &Area,
    buffer: &mut [u8],
) {
    // Get intersection of drawing area with clip region
    let clipped = match screen_area.intersection(clip_area) {
        Some(area) => area,
        None => return,  // Completely clipped
    };
    
    // Map clipped area to buffer coordinates
    for screen_y in clipped.y1..=clipped.y2 {
        let buffer_y = screen_y - buf_area.y1;
        
        if buffer_y < 0 || buffer_y >= buffer.height as i32 {
            continue;  // Outside buffer bounds
        }
        
        for screen_x in clipped.x1..=clipped.x2 {
            let buffer_x = screen_x - buf_area.x1;
            
            if buffer_x >= 0 && buffer_x < buffer.width as i32 {
                // Write pixel
                let offset = (buffer_y as usize * buffer.stride) + (buffer_x as usize * 2);
                // ... write pixel data
            }
        }
    }
}
```

---

## Memory Analysis

### Full-Screen Rendering

```
Display: 240×240, RGB565

Buffer: 240 × 240 × 2 = 115,200 bytes

Peak RAM: 115,200 bytes
```

### Strip Rendering (100-row strips)

```
Display: 240×240, RGB565
Strip height: 100 rows

Buffer: 240 × 100 × 2 = 48,000 bytes
Number of strips: 3

Peak RAM: 48,000 bytes

Savings: 115,200 - 48,000 = 67,200 bytes (58% reduction)
```

### Trade-offs

| Aspect | Full-Screen | Strip-Based |
|--------|-------------|-------------|
| **Memory** | 115 KB | 48 KB |
| **Rendering passes** | 1 | 3 |
| **CPU overhead** | Low | Medium (3× task creation) |
| **Flush calls** | 1 | 3 |
| **Complexity** | Low | Medium |
| **Best for** | High RAM | Low RAM |

### Optimal Strip Height

```rust
fn calculate_optimal_strip_height(
    screen_height: u32,
    available_ram: usize,
    bytes_per_pixel: usize,
    screen_width: u32,
) -> u32 {
    // Maximum strip height that fits in RAM
    let max_strip_height = available_ram / (screen_width as usize * bytes_per_pixel);
    
    // Try to use divisor of screen height for even strips
    for divisor in [1, 2, 3, 4, 5, 6, 8, 10, 12, 15, 16, 20, 24, 30, 32] {
        let candidate = screen_height / divisor;
        if candidate <= max_strip_height as u32 {
            return candidate;
        }
    }
    
    max_strip_height as u32
}

// Example:
// screen_height = 240
// available_ram = 50,000 bytes
// bytes_per_pixel = 2
// screen_width = 240
//
// max_strip_height = 50,000 / (240 × 2) = 104 rows
// Best divisor: 240 / 3 = 80 rows (3 equal strips)
```

---

## Summary

### Core Principles

1. **Single buffer reused** - Same physical memory for all strips
2. **Metadata updates** - `layer.buf_area` tells where buffer maps to screen
3. **Screen coordinates** - Tasks always use absolute screen coordinates
4. **Coordinate mapping** - `buffer_y = screen_y - buf_area.y1`
5. **Intersection testing** - Only render tasks that intersect current strip

### Algorithm

```
For each strip S from 0 to N:
    1. Set layer.buf_area to strip S's screen region
    2. For each drawing request R:
        a. If R.area intersects layer.buf_area:
            - Create task with R's screen coordinates
        b. Else:
            - Skip (not in this strip)
    3. Execute all tasks:
        - Map screen coordinates to buffer coordinates
        - Write pixels to buffer
    4. Flush buffer to display at strip position
    5. Clear tasks for next strip
```

### Key Benefits

- **Memory efficient**: Render large screens with small buffers
- **Simple**: Same rendering code, just coordinate mapping
- **Flexible**: Strip height adjustable based on RAM
- **Proven**: Used by LVGL, tested in millions of devices

This architecture enables rendering on memory-constrained embedded systems while maintaining clean separation between drawing operations and buffer management.
