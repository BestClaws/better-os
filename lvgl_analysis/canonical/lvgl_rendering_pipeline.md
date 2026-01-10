# LVGL Rendering Pipeline Documentation

**Version:** Based on LVGL v9.x  
**Date:** 10 January 2026  
**Source:** LVGL Core Architecture Analysis

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Components](#architecture-components)
3. [Rendering Pipeline Stages](#rendering-pipeline-stages)
4. [Draw Task System](#draw-task-system)
5. [Layer Management](#layer-management)
6. [Draw Unit System](#draw-unit-system)
7. [Display and Buffer Management](#display-and-buffer-management)
8. [Task States and Lifecycle](#task-states-and-lifecycle)
9. [Dispatching and Scheduling](#dispatching-and-scheduling)
10. [Synchronous vs Asynchronous Rendering](#synchronous-vs-asynchronous-rendering)
11. [Performance Optimization](#performance-optimization)
12. [Implementation Guide](#implementation-guide)

---

## Overview

LVGL v9 introduces a sophisticated rendering pipeline designed for modern embedded systems with hardware acceleration support. The architecture separates rendering into discrete, schedulable tasks that can be executed by multiple draw units in parallel or asynchronously.

### Key Design Principles

1. **Task-Based Rendering**: All drawing operations are converted into discrete tasks
2. **Multi-Unit Support**: Multiple draw units (SW, GPU, DMA2D, etc.) can process tasks simultaneously
3. **Layer Architecture**: Hierarchical rendering with compositing support
4. **Asynchronous Execution**: Non-blocking rendering for responsive UIs
5. **Smart Dispatch**: Intelligent task assignment based on draw unit capabilities

### High-Level Pipeline Flow

```
User Code → Widget Tree → Refresh/Invalidation → Draw Task Creation →
Task Evaluation → Task Dispatch → Draw Unit Execution → Buffer Flush → Display
```

---

## Architecture Components

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                      LVGL Rendering System                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │   Display    │   │    Layer     │   │  Draw Task   │   │
│  │  Management  │──▶│  Management  │──▶│    Queue     │   │
│  └──────────────┘   └──────────────┘   └──────────────┘   │
│         │                   │                   │           │
│         ▼                   ▼                   ▼           │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   │
│  │    Buffer    │   │   Refresh    │   │  Dispatcher  │   │
│  │  Management  │   │   System     │   │              │   │
│  └──────────────┘   └──────────────┘   └──────────────┘   │
│                                                 │           │
│                         ┌───────────────────────┘           │
│                         ▼                                   │
│         ┌───────────────────────────────────┐              │
│         │        Draw Units                  │              │
│         ├───────────────────────────────────┤              │
│         │  SW │ GPU │ DMA2D │ VG_LITE │...  │              │
│         └───────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Purpose |
|-----------|---------|
| **Display** | Manages screen properties, buffers, and flush callbacks |
| **Layer** | Hierarchical drawing surfaces with independent clipping and transformation |
| **Draw Task** | Atomic rendering operation with type, area, and descriptor |
| **Draw Unit** | Rendering backend (software, GPU, DMA, etc.) |
| **Dispatcher** | Task assignment and scheduling logic |
| **Buffer Manager** | Memory allocation for draw buffers and layers |
| **Refresh System** | Invalidation tracking and dirty region management |

---

## Rendering Pipeline Stages

### Stage 1: Invalidation and Refresh Trigger

**Purpose**: Mark areas that need redrawing

```
Widget Updates → lv_obj_invalidate() → Invalidation Area Tracking →
Timer Callback → lv_refr_now() → Refresh Process Start
```

**Key Functions**:
- `lv_inv_area()` - Add area to invalidation list
- `lv_refr_now()` - Force immediate refresh
- `lv_display_get_invalid_areas()` - Get areas needing redraw

**Process**:
1. Widgets mark themselves or areas as invalid when properties change
2. Invalid areas are accumulated during the frame
3. Timer callback triggers refresh at configured interval
4. System determines which widgets overlap invalid regions

### Stage 2: Layer Creation and Setup

**Purpose**: Create rendering surfaces for compositing

```
Screen Hierarchy → Layer Allocation → Buffer Assignment →
Clipping Configuration → Transform Matrix Setup
```

**Layer Properties**:
```c
struct lv_layer_t {
    lv_draw_buf_t * draw_buf;           // Target buffer
    lv_draw_task_t * draw_task_head;    // Task list
    lv_layer_t * parent;                 // Parent layer
    lv_layer_t * next;                   // Sibling layer
    lv_area_t buf_area;                  // Buffer coordinates
    lv_area_t phy_clip_area;             // Physical clip area
    lv_area_t _clip_area;                // Current clip area
    lv_matrix_t matrix;                  // Transform matrix
    lv_opa_t opa;                        // Layer opacity
    lv_color_format_t color_format;      // Pixel format
    bool all_tasks_added;                // Task completion flag
}
```

**Layer Types**:
- **Screen Layer**: Top-level layer for entire display
- **Widget Layer**: Intermediate layers for complex widgets
- **Effect Layer**: Temporary layers for shadows, blurs, etc.

### Stage 3: Draw Task Creation

**Purpose**: Convert drawing operations into discrete tasks

```
Widget Rendering → Primitive API Calls → Draw Task Allocation →
Task Descriptor Population → Task Queuing
```

**Task Creation Process**:
```c
// 1. Allocate task with descriptor
lv_draw_task_t * task = lv_draw_add_task(layer, &coords, task_type);

// 2. Populate task descriptor
lv_draw_rect_dsc_t * dsc = task->draw_dsc;
dsc->bg_color = color;
dsc->radius = radius;
// ... set other properties

// 3. Finalize task
lv_draw_finalize_task_creation(layer, task);
```

**Task Types**:
```c
typedef enum {
    LV_DRAW_TASK_TYPE_NONE,
    LV_DRAW_TASK_TYPE_FILL,          // Solid fills
    LV_DRAW_TASK_TYPE_BORDER,        // Borders
    LV_DRAW_TASK_TYPE_BOX_SHADOW,    // Shadows
    LV_DRAW_TASK_TYPE_LETTER,        // Single character
    LV_DRAW_TASK_TYPE_LABEL,         // Text strings
    LV_DRAW_TASK_TYPE_IMAGE,         // Images/sprites
    LV_DRAW_TASK_TYPE_LAYER,         // Layer blending
    LV_DRAW_TASK_TYPE_LINE,          // Lines
    LV_DRAW_TASK_TYPE_ARC,           // Arcs/circles
    LV_DRAW_TASK_TYPE_TRIANGLE,      // Triangles
    LV_DRAW_TASK_TYPE_MASK_RECTANGLE,// Rectangle masks
    LV_DRAW_TASK_TYPE_MASK_BITMAP,   // Bitmap masks
    LV_DRAW_TASK_TYPE_BLUR,          // Blur effects
    LV_DRAW_TASK_TYPE_VECTOR,        // Vector graphics
} lv_draw_task_type_t;
```

### Stage 4: Task Evaluation

**Purpose**: Determine optimal draw unit for each task

```
New Task → Evaluate All Draw Units → Score Assignment →
Preferred Unit Selection
```

**Evaluation Process**:
```c
// For each draw unit
unit->evaluate_cb(unit, task);

// Draw unit sets:
task->preferred_draw_unit_id = unit_id;
task->preference_score = score;  // 80 = 20% better than SW
                                  // 100 = same as SW
                                  // 110 = 10% worse than SW
```

**Scoring Strategy**:
- **< 100**: Unit is faster/better for this task
- **= 100**: Same performance as software renderer
- **> 100**: Unit is slower/worse for this task
- **NONE**: Unit cannot handle this task

### Stage 5: Task Dispatch

**Purpose**: Assign tasks to available draw units

```
Task Queue → Dependency Check → Unit Availability →
Task Assignment → Unit Execution Start
```

**Dispatch Logic**:
```c
void lv_draw_dispatch(void) {
    // For each layer
    for (layer in display->layer_head) {
        // For each draw unit
        for (unit in draw_units) {
            // Let unit pick compatible tasks
            int taken = unit->dispatch_cb(unit, layer);
            
            // taken = 1: Task accepted and started
            // taken = 0: Task queued but not started yet
            // taken = -1: Unit wanted task but failed (retry later)
        }
    }
}
```

**Task Independence Check**:
- Tasks are independent if their areas don't overlap
- Independent tasks can be processed in parallel
- Dependent tasks must wait for overlapping tasks to complete

### Stage 6: Draw Unit Execution

**Purpose**: Perform actual rendering

```
Task Assignment → Draw Unit Processing → Pixel Generation →
Buffer Writing → Task Completion
```

**Draw Unit Interface**:
```c
struct lv_draw_unit_t {
    const char * name;                           // "SW", "GPU", etc.
    int32_t idx;                                 // Unit index
    
    // Evaluate task compatibility and performance
    int32_t (*evaluate_cb)(lv_draw_unit_t * unit, lv_draw_task_t * task);
    
    // Dispatch and execute tasks
    int32_t (*dispatch_cb)(lv_draw_unit_t * unit, lv_layer_t * layer);
    
    // Wait for asynchronous completion
    int32_t (*wait_for_finish_cb)(lv_draw_unit_t * unit);
    
    // Cleanup
    int32_t (*delete_cb)(lv_draw_unit_t * unit);
    
    // Event handling
    void (*event_cb)(lv_event_t * event);
};
```

**Execution Types**:
1. **Synchronous**: Task completes before dispatch_cb returns
2. **Asynchronous**: Task queued, completes later
3. **Pipelined**: Multiple tasks queued and processed in order

### Stage 7: Layer Compositing

**Purpose**: Blend child layers into parent layers

```
Child Layer Complete → LAYER Task Unblock →
Alpha Blending → Parent Buffer Update
```

**Compositing Flow**:
1. Child layer accumulates all draw tasks
2. `layer->all_tasks_added` flag set when complete
3. Parent layer has `LV_DRAW_TASK_TYPE_LAYER` task in BLOCKED state
4. When child completes, parent task transitions to WAITING
5. Draw unit blends child buffer into parent with opacity/transforms

### Stage 8: Buffer Flush

**Purpose**: Transfer rendered buffer to physical display

```
All Tasks Complete → Flush Callback → DMA Transfer →
Display Controller Update → Buffer Swap
```

**Flush Process**:
```c
// User-provided flush callback
void my_flush_cb(lv_display_t * disp, const lv_area_t * area, uint8_t * px_map) {
    // Copy px_map to display at area coordinates
    dma_copy(px_map, area);
    
    // Signal completion
    lv_display_flush_ready(disp);
}
```

**Flush Modes**:
- **Blocking**: Wait for transfer to complete
- **Non-blocking**: Return immediately, signal via interrupt
- **Custom Wait**: Use `flush_wait_cb` for complex synchronization

---

## Draw Task System

### Task Structure

```c
struct lv_draw_task_t {
    lv_draw_task_t * next;                // Linked list
    
    lv_draw_task_type_t type;             // Task type
    lv_area_t area;                        // Drawing area
    lv_area_t _real_area;                  // Extended area (shadows, etc.)
    lv_area_t clip_area;                   // Saved clip area
    lv_layer_t * target_layer;             // Target layer
    
    lv_draw_unit_t * draw_unit;            // Assigned unit
    volatile int state;                     // Task state (atomic)
    void * draw_dsc;                        // Type-specific descriptor
    
    lv_opa_t opa;                          // Opacity
    uint8_t preferred_draw_unit_id;        // Preferred unit
    uint8_t preference_score;              // Preference score
    
    #if LV_DRAW_TRANSFORM_USE_MATRIX
    lv_matrix_t matrix;                    // Transform matrix
    #endif
};
```

### Task Descriptor Examples

**Fill Task**:
```c
typedef struct {
    lv_draw_dsc_base_t base;
    int32_t radius;
    lv_opa_t opa;
    lv_color_t color;
    lv_grad_dsc_t grad;
} lv_draw_fill_dsc_t;
```

**Image Task**:
```c
typedef struct {
    lv_draw_dsc_base_t base;
    const void * src;
    lv_image_header_t header;
    int32_t rotation;
    int32_t scale_x;
    int32_t scale_y;
    lv_point_t pivot;
    lv_color_t recolor;
    lv_opa_t opa;
    // ... more fields
} lv_draw_image_dsc_t;
```

### Task Lifecycle

```
Created (WAITING) → Evaluated → Dispatched →
Assigned → In Progress → Finished → Cleaned Up
```

**State Diagram**:
```
          ┌─────────────┐
          │   BLOCKED   │  (waiting for dependencies)
          └──────┬──────┘
                 │ dependencies resolved
                 ▼
          ┌─────────────┐
    ┌────▶│   WAITING   │◀────┐
    │     └──────┬──────┘     │
    │            │             │
    │  dispatch  │             │ task returned
    │            ▼             │
    │     ┌─────────────┐     │
    │     │   QUEUED    │─────┘
    │     └──────┬──────┘
    │            │ unit picks task
    │            ▼
    │     ┌─────────────┐
    │     │ IN_PROGRESS │
    │     └──────┬──────┘
    │            │ rendering complete
    │            ▼
    │     ┌─────────────┐
    └─────│  FINISHED   │
          └─────────────┘
```

---

## Layer Management

### Understanding Layer vs Canvas

**Important Distinction**: LVGL has two related but different concepts that serve distinct purposes.

#### Layer (`lv_layer_t`) - Internal Rendering System

**What it is**: The core rendering abstraction used by LVGL's draw engine for all rendering operations.

**Structure**:
```c
struct _lv_layer_t {
    lv_draw_buf_t * draw_buf;        // Backing pixel buffer
    lv_draw_task_t * draw_task_head; // Queued draw tasks
    lv_layer_t * parent;              // Parent in hierarchy
    lv_layer_t * next;                // Next sibling layer
    lv_area_t buf_area;               // Buffer coordinates
    lv_area_t phy_clip_area;          // Physical clipping region
    lv_area_t _clip_area;             // Current clip area
    lv_opa_t opa;                     // Layer opacity
    lv_color_format_t color_format;   // Pixel format
    // ... more fields
};
```

**Purpose**:
- ✅ Automatic widget rendering
- ✅ Effects (blur, shadows, transformations)
- ✅ Layer compositing and blending
- ✅ Hardware acceleration support
- ✅ **Managed internally by LVGL** during refresh

**Usage**: You don't directly create layers - LVGL creates them automatically when:
- Rendering widgets with effects (shadows, blur, transforms)
- Compositing multiple visual elements
- Applying opacity or blend modes
- Hardware accelerated operations

**API Example** (internal LVGL functions):
```c
void lv_draw_fill(lv_layer_t * layer, const lv_draw_fill_dsc_t * dsc, const lv_area_t * coords);
void lv_draw_blur(lv_layer_t * layer, const lv_draw_blur_dsc_t * dsc, const lv_area_t * coords);
void lv_draw_layer(lv_layer_t * layer, const lv_draw_image_dsc_t * dsc, const lv_area_t * coords);
```

#### Canvas (`lv_canvas_t`) - User Widget for Custom Drawing

**What it is**: A **widget** (like button or label) that provides a drawable buffer for direct pixel manipulation.

**Structure**:
```c
struct _lv_canvas_t {
    lv_image_t img;              // Base: extends image widget
    lv_draw_buf_t * draw_buf;    // User-provided buffer
    lv_draw_buf_t static_buf;    // Static buffer storage
};
```

**Purpose**:
- ✅ Custom graphics (games, charts, visualizations)
- ✅ Direct pixel access and manipulation
- ✅ Dynamic content generation
- ✅ **Explicitly created and managed by user code**

**Usage**: You create and control canvas widgets for custom drawing needs:

```c
// Create canvas widget
lv_obj_t * canvas = lv_canvas_create(parent);

// Provide a buffer
static uint8_t canvas_buffer[240 * 200 * 4]; // ARGB8888
lv_canvas_set_buffer(canvas, canvas_buffer, 240, 200, LV_COLOR_FORMAT_ARGB8888);

// Draw directly to the buffer
lv_canvas_set_px(canvas, x, y, color, opacity);
lv_canvas_fill_bg(canvas, bg_color, LV_OPA_COVER);
lv_canvas_draw_rect(canvas, x, y, w, h, &dsc);
lv_canvas_draw_line(canvas, points, point_cnt, &dsc);
lv_canvas_draw_text(canvas, x, y, max_w, &dsc, "Hello");
```

**Key Operations**:
```c
// Initialize layer rendering context for canvas
lv_canvas_init_layer(canvas, layer);

// Custom drawing operations
// ... use layer drawing APIs

// Finish and composite to canvas
lv_canvas_finish_layer(canvas, layer);
```

#### The Relationship

```
┌─────────────────────────────────────────┐
│  Application Layer                      │
│  ┌──────────────┐   ┌──────────────┐   │
│  │ Widgets      │   │ Canvas Widget│   │
│  │ (Button,     │   │ (User draws) │   │
│  │  Label, etc) │   │              │   │
│  └──────┬───────┘   └──────┬───────┘   │
└─────────┼──────────────────┼───────────┘
          │                  │
          ▼                  ▼
┌─────────────────────────────────────────┐
│  LVGL Internal Rendering Engine         │
│  ┌─────────────────────────────────┐    │
│  │   Layer System (lv_layer_t)     │    │
│  │   - Buffers & compositing       │    │
│  │   - Draw task queue             │    │
│  │   - Effects (blur, shadow)      │    │
│  │   - Hardware acceleration       │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────┐
│  Draw Units (SW, GPU, DMA, etc)         │
└─────────────────────────────────────────┘
```

**When Canvas Uses Layers**:
- Canvas widget's buffer becomes a layer target during rendering
- Drawing operations on canvas go through the layer system
- Canvas gets composited into parent layer like any widget
- Benefits from hardware acceleration and effect support

**Summary Table**:

| Feature | Layer (`lv_layer_t`) | Canvas (`lv_canvas_t`) |
|---------|---------------------|------------------------|
| **Type** | Internal rendering abstraction | User-facing widget |
| **Created by** | LVGL automatically | User code explicitly |
| **Purpose** | Widget rendering, effects, compositing | Custom drawing, pixel manipulation |
| **Buffer** | Managed by LVGL | Provided by user |
| **Access** | Internal draw APIs only | Public widget APIs |
| **Use case** | All standard widgets | Games, charts, custom graphics |
| **Hierarchy** | Parent/child relationships | Widget tree member |
| **Effects** | Native support | Inherits from widget system |

### Layer Hierarchy

```
Display
  └─ Screen Layer (root)
      ├─ Widget Layer 1
      │   ├─ Child Layer 1.1
      │   └─ Child Layer 1.2
      ├─ Widget Layer 2
      └─ Effect Layer (shadow)
```

### Layer Operations

**Creating a Layer**:
```c
lv_layer_t * layer = lv_malloc(sizeof(lv_layer_t));
layer->draw_buf = allocate_buffer();
layer->parent = parent_layer;
layer->all_tasks_added = false;
```

**Adding Tasks to Layer**:
```c
lv_draw_task_t * task = lv_draw_add_task(layer, &area, task_type);
// ... configure task
lv_draw_finalize_task_creation(layer, task);
```

**Finalizing Layer**:
```c
layer->all_tasks_added = true;
// If parent exists, unblock LAYER task in parent
```

### Layer Buffers

**Buffer Allocation**:
- Allocated from heap or static memory
- Size based on layer area and color format
- Can be reused for different layers in sequence

**Buffer Formats**:
- Match display color format or use intermediate format
- Common formats: RGB565, RGB888, ARGB8888
- Alpha channels for transparency support

---

## Draw Unit System

### Built-in Draw Units

| Draw Unit | Description | Typical Use |
|-----------|-------------|-------------|
| **SW** | Software renderer | Fallback, always available |
| **DMA2D** | STM32 hardware accelerator | Fills, blits, color conversion |
| **VG_LITE** | Vector graphics GPU | Paths, transformations |
| **ESP_LCD** | ESP32 LCD controller | Direct display writes |
| **NXP PXP** | NXP pixel pipeline | Image compositing |
| **SDL** | Desktop rendering | Simulation/testing |

### Creating Custom Draw Units

**Registration**:
```c
// Allocate and initialize
my_draw_unit_t * unit = lv_draw_create_unit(sizeof(my_draw_unit_t));
unit->base.name = "MY_GPU";
unit->base.evaluate_cb = my_evaluate_cb;
unit->base.dispatch_cb = my_dispatch_cb;
unit->base.wait_for_finish_cb = my_wait_cb;
unit->base.delete_cb = my_delete_cb;

// Unit automatically added to global list
```

**Evaluation Callback**:
```c
int32_t my_evaluate_cb(lv_draw_unit_t * u, lv_draw_task_t * task) {
    // Check if we can handle this task type
    if (task->type == LV_DRAW_TASK_TYPE_FILL) {
        // Check if profitable (e.g., area size)
        int32_t area_size = lv_area_get_size(&task->area);
        if (area_size > 1000) {
            task->preferred_draw_unit_id = u->idx;
            task->preference_score = 70;  // 30% better than SW
            return 0;
        }
    }
    return -1;  // Cannot handle
}
```

**Dispatch Callback**:
```c
int32_t my_dispatch_cb(lv_draw_unit_t * u, lv_layer_t * layer) {
    // Get available task
    lv_draw_task_t * task = lv_draw_get_available_task(layer, NULL, u->idx);
    if (task == NULL) return 0;
    
    // Check if we can handle it
    if (!my_can_handle(task)) return 0;
    
    // Claim task
    task->state = LV_DRAW_TASK_STATE_IN_PROGRESS;
    task->draw_unit = u;
    
    // Execute (sync) or queue (async)
    my_execute_task(task);
    
    // Mark complete (or will be marked by interrupt later)
    task->state = LV_DRAW_TASK_STATE_FINISHED;
    
    return 1;  // Took 1 task
}
```

### Multi-Unit Coordination

**Parallel Execution**:
- Multiple units can work on independent tasks simultaneously
- Task independence determined by area overlap
- Automatic dependency resolution

**Priority and Scheduling**:
- Units with lower preference scores get first chance
- Tasks can be reassigned if unit fails
- Fallback to software rendering always available

---

## Display and Buffer Management

### Display Configuration

**Basic Setup**:
```c
lv_display_t * disp = lv_display_create(800, 480);
lv_display_set_color_format(disp, LV_COLOR_FORMAT_RGB565);
lv_display_set_flush_cb(disp, my_flush_cb);
```

**Buffer Modes**:
```c
// Mode 1: Partial rendering (minimal RAM)
void * buf1 = malloc(800 * 100 * 2);  // 1/5 screen
lv_display_set_buffers(disp, buf1, NULL, 800 * 100 * 2, 
                       LV_DISPLAY_RENDER_MODE_PARTIAL);

// Mode 2: Double buffering (full screen, no tearing)
void * buf1 = malloc(800 * 480 * 2);
void * buf2 = malloc(800 * 480 * 2);
lv_display_set_buffers(disp, buf1, buf2, 800 * 480 * 2,
                       LV_DISPLAY_RENDER_MODE_DIRECT);

// Mode 3: Full refresh (always redraw everything)
void * buf1 = malloc(800 * 480 * 2);
void * buf2 = malloc(800 * 480 * 2);
lv_display_set_buffers(disp, buf1, buf2, 800 * 480 * 2,
                       LV_DISPLAY_RENDER_MODE_FULL);
```

### Render Modes Explained

**PARTIAL Mode**:
- Render in multiple passes with smaller buffer
- Each pass renders a horizontal strip
- Lower RAM usage, more CPU overhead
- Best for: Memory-constrained devices

**DIRECT Mode**:
- Buffer size = screen size
- Only changed areas are redrawn
- Efficient for small updates
- Best for: Moderate updates, sufficient RAM

**FULL Mode**:
- Always redraw entire screen
- Requires screen-sized buffer
- Eliminates dirty tracking overhead
- Best for: Animated interfaces, double-buffered displays

### Buffer Stride

**Standard Layout**:
```c
// Pixels packed consecutively
stride = width * bytes_per_pixel
```

**Custom Stride**:
```c
// For aligned framebuffers
void * buf = aligned_alloc(64, stride * height);
lv_display_set_buffers_with_stride(disp, buf, NULL, 
                                   stride * height, stride,
                                   LV_DISPLAY_RENDER_MODE_DIRECT);
```

---

## Task States and Lifecycle

### State Definitions

```c
typedef enum {
    LV_DRAW_TASK_STATE_BLOCKED,      // Waiting for dependencies
    LV_DRAW_TASK_STATE_WAITING,      // Ready to be dispatched
    LV_DRAW_TASK_STATE_QUEUED,       // Assigned to unit queue
    LV_DRAW_TASK_STATE_IN_PROGRESS,  // Being rendered
    LV_DRAW_TASK_STATE_FINISHED,     // Complete, ready for cleanup
} lv_draw_task_state_t;
```

### State Transitions

**Normal Flow**:
```
WAITING → QUEUED → IN_PROGRESS → FINISHED
```

**With Dependencies**:
```
BLOCKED → [dependency resolves] → WAITING → QUEUED → IN_PROGRESS → FINISHED
```

**Failed Attempt**:
```
WAITING → QUEUED → [unit error] → WAITING → [retry] → QUEUED → ...
```

### Dependency Management

**Area Overlap Check**:
```c
bool is_dependent = lv_area_is_on(&task1->area, &task2->area);
```

**Blocked Task Example**:
```c
// Child layer creates LAYER task in parent
lv_draw_task_t * layer_task = lv_draw_add_task(parent_layer, &area, 
                                                LV_DRAW_TASK_TYPE_LAYER);
layer_task->state = LV_DRAW_TASK_STATE_BLOCKED;

// When child layer completes
child_layer->all_tasks_added = true;
// System automatically transitions layer_task to WAITING
```

---

## Dispatching and Scheduling

### Dispatch Trigger Points

1. **After task creation**: `lv_draw_finalize_task_creation()`
2. **After task completion**: Draw unit calls `lv_draw_dispatch_request()`
3. **Periodic check**: During refresh cycle
4. **Manual trigger**: `lv_draw_dispatch()`

### Dispatch Algorithm

```c
void lv_draw_dispatch(void) {
    // 1. Remove finished tasks
    cleanup_finished_tasks();
    
    // 2. Unblock dependent tasks
    check_and_unblock_dependencies();
    
    // 3. For each draw unit
    for (unit in units) {
        // Let unit pick tasks
        while (unit->dispatch_cb(unit, layer) > 0) {
            // Continue while unit takes tasks
        }
    }
    
    // 4. If no progress, wait for async completion
    if (!any_task_dispatched) {
        lv_draw_wait_for_finish();
    }
}
```

### Task Selection Strategy

**Single Unit System**:
```c
// Simply get first waiting task
task = layer->draw_task_head;
while (task && task->state != LV_DRAW_TASK_STATE_WAITING)
    task = task->next;
```

**Multi-Unit System**:
```c
// Find independent task for this unit
task = layer->draw_task_head;
while (task) {
    if (task->preferred_draw_unit_id == unit_id &&
        task->state == LV_DRAW_TASK_STATE_WAITING &&
        is_independent(task)) {
        return task;
    }
    task = task->next;
}
```

### Dispatch Request Mechanism

**Without OS** (Polling):
```c
static volatile int dispatch_req = 0;

void lv_draw_dispatch_request(void) {
    dispatch_req = 1;
}

void lv_draw_dispatch_wait_for_request(void) {
    while (!dispatch_req);
    dispatch_req = 0;
}
```

**With OS** (Semaphore):
```c
void lv_draw_dispatch_request(void) {
    lv_thread_sync_signal(&sync);
}

void lv_draw_dispatch_wait_for_request(void) {
    lv_thread_sync_wait(&sync);
}
```

---

## Synchronous vs Asynchronous Rendering

### Synchronous Rendering

**Characteristics**:
- Task completes before dispatch_cb returns
- Simple error handling
- No queuing complexity
- May block UI thread

**Example**:
```c
int32_t sw_dispatch_cb(lv_draw_unit_t * u, lv_layer_t * layer) {
    lv_draw_task_t * task = get_next_task(layer, u->idx);
    if (!task) return 0;
    
    task->state = LV_DRAW_TASK_STATE_IN_PROGRESS;
    
    // Render immediately
    render_task(task);
    
    task->state = LV_DRAW_TASK_STATE_FINISHED;
    
    return 1;
}
```

**Timeline**:
```
LVGL Thread
│
├─ Dispatch task1 ───────┐
│                         │ render task1 (blocking)
│                         ▼
│                    task1 complete
│
├─ Dispatch task2 ───────┐
│                         │ render task2 (blocking)
│                         ▼
│                    task2 complete
│
└─ Continue
```

### Asynchronous Rendering

**Characteristics**:
- Tasks queued and processed separately
- Non-blocking UI thread
- Requires synchronization
- Better throughput

**Example**:
```c
int32_t gpu_dispatch_cb(lv_draw_unit_t * u, lv_layer_t * layer) {
    my_gpu_unit_t * gpu = (my_gpu_unit_t *)u;
    
    lv_draw_task_t * task = get_next_task(layer, u->idx);
    if (!task) return 0;
    
    // Check if queue is full
    if (gpu->queue_count >= MAX_QUEUE) return 0;
    
    task->state = LV_DRAW_TASK_STATE_QUEUED;
    
    // Add to internal queue
    gpu->queue[gpu->queue_count++] = task;
    
    // Submit to hardware (non-blocking)
    if (gpu->hw_idle) {
        submit_to_hw(task);
        task->state = LV_DRAW_TASK_STATE_IN_PROGRESS;
        gpu->hw_idle = false;
    }
    
    return 1;  // Task queued successfully
}

void gpu_interrupt_handler(void) {
    // Task completed
    completed_task->state = LV_DRAW_TASK_STATE_FINISHED;
    
    // Submit next queued task
    if (gpu->queue_count > 0) {
        next_task = gpu->queue[0];
        submit_to_hw(next_task);
        next_task->state = LV_DRAW_TASK_STATE_IN_PROGRESS;
        // Shift queue
    } else {
        gpu->hw_idle = true;
    }
    
    // Request dispatch for more tasks
    lv_draw_dispatch_request();
}

int32_t gpu_wait_for_finish_cb(lv_draw_unit_t * u) {
    my_gpu_unit_t * gpu = (my_gpu_unit_t *)u;
    
    // Wait for all queued tasks to complete
    while (!gpu->hw_idle) {
        wait_for_interrupt();
    }
    
    return 0;
}
```

**Timeline**:
```
LVGL Thread                GPU Thread               Hardware
│                              │                        │
├─ Dispatch task1 ─────────▶  queue task1              │
│                              │                        │
├─ Dispatch task2 ─────────▶  queue task2              │
│                              │                        │
├─ Dispatch task3 ─────────▶  queue task3              │
│                              │                        │
├─ Dispatch task4 ─────────▶  queue task4              │
│                              │                        │
│  (continue UI work)          │                        │
│                              │                        │
│                              ├─ Submit task1 ──────▶  render task1
│                              │                        │
│                              │                        ├─ complete ─┐
│                              │  ◀─────────────────────┘            │
│                              ├─ Submit task2,3,4 ──▶  render...    │
│                              │                                     │
│  (end of frame)              │                                     │
├─ wait_for_finish() ────────▶ wait for queue         ◀──────────────┘
│                              │
│  ◀──────────────────────────┤ all done
│
└─ Flush and swap
```

### Choosing Rendering Mode

| Factor | Synchronous | Asynchronous |
|--------|-------------|--------------|
| **Complexity** | Simple | Complex |
| **Latency** | Higher | Lower |
| **Throughput** | Lower | Higher |
| **Memory** | Less | More (queues) |
| **Best For** | Simple GPUs, low-cost MCUs | High-performance GPUs, multi-core |

---

## Performance Optimization

### Optimization Strategies

#### 1. Task Batching

**Problem**: Creating many small tasks has overhead

**Solution**: Combine related operations
```c
// Instead of: multiple small fills
for (i = 0; i < 100; i++)
    draw_fill(small_area[i]);

// Combine into: one large fill or fewer tasks
merge_areas_and_draw();
```

#### 2. Layer Reuse

**Problem**: Layer allocation is expensive

**Solution**: Reuse layers for similar operations
```c
static lv_layer_t * cached_layer = NULL;

if (!cached_layer || size_changed) {
    cached_layer = allocate_layer();
}
use_layer(cached_layer);
```

#### 3. Smart Dispatch

**Problem**: Units checking tasks they can't handle

**Solution**: Early filtering in evaluate_cb
```c
int32_t my_evaluate_cb(lv_draw_unit_t * u, lv_draw_task_t * task) {
    // Quick reject
    if (task->type != SUPPORTED_TYPE) return -1;
    
    // Check size threshold
    if (area_too_small(task)) return -1;
    
    // Accept and score
    task->preference_score = calculate_score(task);
    return 0;
}
```

#### 4. Dirty Region Optimization

**Problem**: Redrawing unchanged areas

**Solution**: Careful invalidation
```c
// Only invalidate changed area
lv_obj_invalidate_area(obj, &changed_area);

// Not the entire object
// lv_obj_invalidate(obj);  // Avoid if possible
```

#### 5. Parallel Rendering

**Problem**: Single-threaded bottleneck

**Solution**: Use tile-based rendering
```c
lv_display_set_tile_cnt(disp, 4);  // Split screen into 4 tiles
// Each tile can be rendered independently
```

### Performance Metrics

**Measure Points**:
```c
LV_PROFILER_DRAW_BEGIN;
// ... operation ...
LV_PROFILER_DRAW_END;
```

**Key Metrics**:
- Task creation time
- Dispatch latency
- Rendering time per task type
- Buffer flush time
- Frame time (invalidation to flush)

---

## Implementation Guide

### Minimal Setup

```c
// 1. Create display
lv_display_t * disp = lv_display_create(800, 480);

// 2. Allocate buffers
size_t buf_size = 800 * 100 * 2;  // Partial buffer
void * buf1 = malloc(buf_size);
void * buf2 = malloc(buf_size);

// 3. Set buffers and mode
lv_display_set_buffers(disp, buf1, buf2, buf_size, 
                       LV_DISPLAY_RENDER_MODE_PARTIAL);

// 4. Set flush callback
lv_display_set_flush_cb(disp, my_flush_cb);

// 5. Draw units created automatically (SW renderer)
```

### Adding Hardware Acceleration

```c
// 1. Define custom unit structure
typedef struct {
    lv_draw_unit_t base;
    // Custom fields
    void * hw_handle;
    bool busy;
} my_gpu_unit_t;

// 2. Implement callbacks
static int32_t my_evaluate_cb(lv_draw_unit_t * u, lv_draw_task_t * task);
static int32_t my_dispatch_cb(lv_draw_unit_t * u, lv_layer_t * layer);
static int32_t my_wait_cb(lv_draw_unit_t * u);
static int32_t my_delete_cb(lv_draw_unit_t * u);

// 3. Register unit
void my_gpu_init(void) {
    my_gpu_unit_t * unit = lv_draw_create_unit(sizeof(my_gpu_unit_t));
    unit->base.name = "MY_GPU";
    unit->base.evaluate_cb = my_evaluate_cb;
    unit->base.dispatch_cb = my_dispatch_cb;
    unit->base.wait_for_finish_cb = my_wait_cb;
    unit->base.delete_cb = my_delete_cb;
    
    // Initialize hardware
    unit->hw_handle = init_hw();
    unit->busy = false;
}
```

### Flush Callback Implementation

**Simple Blocking**:
```c
void my_flush_cb(lv_display_t * disp, const lv_area_t * area, uint8_t * px_map) {
    int32_t w = lv_area_get_width(area);
    int32_t h = lv_area_get_height(area);
    
    // Send to display (blocking)
    lcd_set_window(area->x1, area->y1, area->x2, area->y2);
    lcd_write_pixels(px_map, w * h);
    
    // Signal complete
    lv_display_flush_ready(disp);
}
```

**DMA Non-Blocking**:
```c
void my_flush_cb(lv_display_t * disp, const lv_area_t * area, uint8_t * px_map) {
    int32_t w = lv_area_get_width(area);
    int32_t h = lv_area_get_height(area);
    
    // Start DMA transfer (non-blocking)
    lcd_set_window(area->x1, area->y1, area->x2, area->y2);
    dma_start_transfer(px_map, w * h);
    
    // Will call lv_display_flush_ready() in interrupt
    // lv_display_flush_ready(disp);  // Not here!
}

void dma_complete_interrupt(void) {
    lv_display_flush_ready(current_display);
}
```

### Layer Management Example

```c
// Creating effect layer for shadow
void draw_card_with_shadow(lv_layer_t * parent_layer, lv_area_t * card_area) {
    // 1. Create shadow layer
    lv_layer_t * shadow_layer = lv_layer_create(parent_layer, shadow_area);
    
    // 2. Draw shadow to shadow layer
    lv_draw_box_shadow(shadow_layer, card_area, &shadow_dsc);
    
    // 3. Finalize shadow layer
    shadow_layer->all_tasks_added = true;
    
    // 4. Draw card to parent layer
    lv_draw_rect(parent_layer, card_area, &card_dsc);
    
    // 5. Shadow layer will be composited automatically
}
```

---

## Summary

### Key Takeaways

1. **Task-Based Architecture**: All rendering is broken into discrete, schedulable tasks
2. **Flexible Acceleration**: Multiple draw units can coexist and specialize
3. **Async-Ready**: Pipeline supports both sync and async execution models
4. **Smart Dispatch**: Automatic task assignment based on capabilities and performance
5. **Layered Rendering**: Hierarchical compositing for complex effects
6. **Zero-Copy Possible**: Direct rendering to display framebuffer in DIRECT/FULL modes

### Pipeline Benefits

| Benefit | Description |
|---------|-------------|
| **Modularity** | Easy to add new rendering backends |
| **Performance** | Hardware acceleration without code changes |
| **Flexibility** | Multiple rendering strategies supported |
| **Scalability** | Parallel execution on multi-core systems |
| **Maintainability** | Clear separation of concerns |

### Common Patterns

**Pattern 1: Simple Software Rendering**
- Single SW draw unit
- Partial or full buffering
- Synchronous rendering
- Good for: Small displays, simple UIs

**Pattern 2: GPU Acceleration**
- SW + GPU draw units
- GPU for large fills, images
- Async execution
- Good for: Medium displays, complex graphics

**Pattern 3: Multi-Unit Pipeline**
- SW + DMA2D + VG_LITE
- Task specialization by type
- Parallel execution
- Good for: Large displays, high performance

### Future Considerations

- **Vulkan/Metal Backend**: Direct GPU access for complex effects
- **Tile-Based Rendering**: Parallel processing of screen regions
- **Compression**: On-the-fly frame buffer compression
- **Adaptive Quality**: Dynamic quality adjustment based on performance
- **Distributed Rendering**: Multi-core CPU rendering

---

## References

- LVGL v9.x Source Code: `src/draw/lv_draw.c`, `lv_draw.h`
- LVGL Documentation: https://docs.lvgl.io/
- Display Driver Guide: https://docs.lvgl.io/master/porting/display.html
- Draw Unit Examples: `src/draw/sw/`, `src/draw/dma2d/`, etc.

