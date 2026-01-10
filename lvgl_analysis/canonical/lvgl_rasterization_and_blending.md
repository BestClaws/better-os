# LVGL Rasterization and Color Format System

**Version:** Based on LVGL v9.x  
**Date:** 10 January 2026  
**Source:** LVGL Software Renderer Implementation Analysis

---

## Table of Contents

1. [Overview](#overview)
2. [Color Format System](#color-format-system)
3. [Blend Architecture](#blend-architecture)
4. [Rasterization Process](#rasterization-process)
5. [Format-Specific Implementations](#format-specific-implementations)
6. [Pixel Operations](#pixel-operations)
7. [Hardware Acceleration](#hardware-acceleration)

---

## Overview

LVGL v9 uses a **format-independent rendering architecture** where:

1. **Drawing operations** are format-agnostic (fill, line, arc, etc.)
2. **Rasterization** generates shape coverage and color data
3. **Blending layer** handles format-specific pixel writing

### Key Architectural Insight

Drawing primitives do NOT directly write pixels. Instead:

```
Drawing Primitive (format-independent)
    ↓
Rasterization (geometry → coverage masks)
    ↓
Blend Descriptor (color + mask + opacity)
    ↓
Format-Specific Blending (actual pixel writes)
    ↓
Target Buffer (RGB565, ARGB8888, etc.)
```

**Source Evidence:**
- `lv_draw_sw_fill.c`: Creates blend descriptors, doesn't touch pixels
- `lv_draw_sw_blend.c`: Routes to format-specific handlers
- `lv_draw_sw_blend_to_*.c`: Actual pixel manipulation per format

---

## Color Format System

### Supported Formats

From `lv_color.h` lines 140-180:

```c
typedef enum {
    LV_COLOR_FORMAT_UNKNOWN           = 0x00,
    
    // Indexed/Grayscale (≤1 byte)
    LV_COLOR_FORMAT_L8                = 0x06,  // 8-bit luminance
    LV_COLOR_FORMAT_I1                = 0x07,  // 1-bit indexed
    LV_COLOR_FORMAT_I2                = 0x08,  // 2-bit indexed
    LV_COLOR_FORMAT_I4                = 0x09,  // 4-bit indexed
    LV_COLOR_FORMAT_I8                = 0x0A,  // 8-bit indexed
    LV_COLOR_FORMAT_A8                = 0x0E,  // 8-bit alpha only
    
    // 16-bit formats
    LV_COLOR_FORMAT_AL88              = 0x10,  // 8-bit alpha + 8-bit luma
    LV_COLOR_FORMAT_RGB565            = 0x12,  // 5-6-5 RGB (most common)
    LV_COLOR_FORMAT_RGB565_SWAPPED    = 0x13,  // Byte-swapped RGB565
    LV_COLOR_FORMAT_RGB565A8          = 0x14,  // RGB565 + separate A8
    LV_COLOR_FORMAT_ARGB1555          = 0x15,  // 1-bit alpha + 5-5-5 RGB
    LV_COLOR_FORMAT_ARGB4444          = 0x16,  // 4-bit each channel
    
    // 24-bit formats
    LV_COLOR_FORMAT_RGB888            = 0x0F,  // 8-bit per channel RGB
    LV_COLOR_FORMAT_ARGB8565          = 0x11,  // 8-bit alpha + RGB565
    
    // 32-bit formats
    LV_COLOR_FORMAT_ARGB8888          = 0x17,  // 8-bit per channel + alpha
    LV_COLOR_FORMAT_XRGB8888          = 0x18,  // RGB888 with unused byte
    LV_COLOR_FORMAT_ARGB8888_PREMULTIPLIED = 0x19, // Pre-multiplied alpha
} lv_color_format_t;
```

### Bits Per Pixel Calculation

From `lv_color.h` lines 60-100:

```c
#define LV_COLOR_FORMAT_GET_BPP(cf) (       \
    (cf) == LV_COLOR_FORMAT_I1 ? 1 :        \
    (cf) == LV_COLOR_FORMAT_A1 ? 1 :        \
    (cf) == LV_COLOR_FORMAT_I2 ? 2 :        \
    (cf) == LV_COLOR_FORMAT_A2 ? 2 :        \
    (cf) == LV_COLOR_FORMAT_I4 ? 4 :        \
    (cf) == LV_COLOR_FORMAT_A4 ? 4 :        \
    (cf) == LV_COLOR_FORMAT_L8 ? 8 :        \
    (cf) == LV_COLOR_FORMAT_A8 ? 8 :        \
    (cf) == LV_COLOR_FORMAT_I8 ? 8 :        \
    (cf) == LV_COLOR_FORMAT_AL88 ? 16 :     \
    (cf) == LV_COLOR_FORMAT_RGB565 ? 16 :   \
    (cf) == LV_COLOR_FORMAT_RGB565_SWAPPED ? 16 : \
    (cf) == LV_COLOR_FORMAT_ARGB1555 ? 16 : \
    (cf) == LV_COLOR_FORMAT_ARGB4444 ? 16 : \
    (cf) == LV_COLOR_FORMAT_ARGB8565 ? 24 : \
    (cf) == LV_COLOR_FORMAT_RGB888 ? 24 :   \
    (cf) == LV_COLOR_FORMAT_ARGB8888 ? 32 : \
    (cf) == LV_COLOR_FORMAT_XRGB8888 ? 32 : \
    0 \
)

#define LV_COLOR_FORMAT_GET_SIZE(cf) ((LV_COLOR_FORMAT_GET_BPP(cf) + 7) >> 3)
```

### Color Type Definitions

From `lv_color.h` lines 105-135:

```c
// Native 24-bit color (used internally)
typedef struct {
    uint8_t blue;
    uint8_t green;
    uint8_t red;
} lv_color_t;

// 16-bit RGB565 representation
typedef struct {
    uint16_t blue : 5;
    uint16_t green : 6;
    uint16_t red : 5;
} lv_color16_t;

// 32-bit ARGB8888 representation
typedef struct {
    uint8_t blue;
    uint8_t green;
    uint8_t red;
    uint8_t alpha;
} lv_color32_t;

// Luminance + Alpha
typedef struct {
    uint8_t lumi;
    uint8_t alpha;
} lv_color16a_t;

// HSV color space
typedef struct {
    uint16_t h;  // Hue 0-360
    uint8_t s;   // Saturation 0-100
    uint8_t v;   // Value 0-100
} lv_color_hsv_t;
```

### Opacity System

From `lv_color.h` lines 38-52:

```c
typedef uint8_t lv_opa_t;  // Opacity: 0-255

enum _lv_opacity_level_t {
    LV_OPA_TRANSP = 0,
    LV_OPA_0      = 0,
    LV_OPA_10     = 25,
    LV_OPA_20     = 51,
    LV_OPA_30     = 76,
    LV_OPA_40     = 102,
    LV_OPA_50     = 127,
    LV_OPA_60     = 153,
    LV_OPA_70     = 178,
    LV_OPA_80     = 204,
    LV_OPA_90     = 229,
    LV_OPA_100    = 255,
    LV_OPA_COVER  = 255,
};

#define LV_OPA_MIN 2    // Fully transparent if opa <= 2
#define LV_OPA_MAX 253  // Fully opaque if opa >= 253
```

---

## Blend Architecture

### Blend System Overview

**Location:** `src/draw/sw/blend/lv_draw_sw_blend.c`

The blend system is LVGL's **critical abstraction layer** that separates format-independent drawing primitives from format-specific pixel operations. This is the **single dispatch point** where all rendering funnels through.

### Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  Drawing Primitives (Format-Independent)        │
│  Files: lv_draw_sw_fill.c, lv_draw_sw_line.c   │
│         lv_draw_sw_arc.c, lv_draw_sw_triangle.c │
│                                                 │
│  Responsibilities:                              │
│  • Geometry calculations                        │
│  • Mask generation (radius, dash, angle)        │
│  • Gradient color map generation                │
│  • Clipping calculations                        │
│  • NO format knowledge - never touch pixels     │
│                                                 │
│  Output: Blend descriptors (color+area+mask)    │
└──────────────┬──────────────────────────────────┘
               │ Calls: lv_draw_sw_blend()
               ▼
┌─────────────────────────────────────────────────┐
│  Blend Router (Format Dispatcher)               │
│  File: lv_draw_sw_blend.c                      │
│  Function: lv_draw_sw_blend()                  │
│                                                 │
│  Single responsibility:                         │
│  • Read layer->color_format                     │
│  • Route to format-specific handler             │
│  • Check for hardware acceleration hooks        │
│                                                 │
│  Two routing functions:                         │
│  • lv_draw_sw_blend_color() - solid fills       │
│  • lv_draw_sw_blend_image() - source images     │
└──────────────┬──────────────────────────────────┘
               │ Routes to format handler
               ▼
┌─────────────────────────────────────────────────┐
│  Format Implementations (Pixel Writers)         │
│  Files: lv_draw_sw_blend_to_*.c                │
│                                                 │
│  Actual pixel manipulation:                     │
│  • Format-specific casting                      │
│  • Component packing/unpacking                  │
│  • Alpha blending math                          │
│  • Optimized pixel write loops                  │
└─────────────────────────────────────────────────┘
```

**Key Architectural Principle:** Drawing primitives generate **abstract blend descriptors** containing color, area, opacity, and masks. The format is selected **once per blend** at the router, not in primitive logic. This means adding a new format requires only implementing the format-specific blend functions - all primitives automatically work.

### Blend Descriptor Types

**Fill Blend Descriptor** (solid colors):
```c
typedef struct {
    void * dest_buf;           // Target buffer pointer
    int32_t dest_w;            // Destination width
    int32_t dest_h;            // Destination height
    int32_t dest_stride;       // Destination stride in bytes
    
    lv_color_t color;          // Fill color
    lv_opa_t opa;              // Opacity
    
    lv_opa_t * mask_buf;       // Coverage mask (NULL if none)
    int32_t mask_stride;       // Mask stride
    
    lv_area_t relative_area;   // Relative coordinates
} lv_draw_sw_blend_fill_dsc_t;
```

**Image Blend Descriptor** (source image):
```c
typedef struct {
    const void * src_buf;      // Source image buffer
    lv_color_format_t src_color_format; // Source format
    int32_t src_stride;        // Source stride
    lv_area_t src_area;        // Source area
    
    void * dest_buf;           // Target buffer
    int32_t dest_w;            // Destination dimensions
    int32_t dest_h;
    int32_t dest_stride;
    
    lv_opa_t opa;              // Global opacity
    lv_blend_mode_t blend_mode; // Blending mode
    
    lv_opa_t * mask_buf;       // Coverage mask
    int32_t mask_stride;
    
    lv_area_t relative_area;   // Relative coordinates
} lv_draw_sw_blend_image_dsc_t;
```

### Main Blend Entry Point

**Function:** `lv_draw_sw_blend()`  
**Location:** `lv_draw_sw_blend.c` lines 72-169  
**Called by:** ALL drawing primitives (fill, line, arc, triangle, image, text, etc.)

```c
void lv_draw_sw_blend(lv_draw_task_t * t, const lv_draw_sw_blend_dsc_t * blend_dsc)
{
    // Skip transparent operations
    if(blend_dsc->opa <= LV_OPA_MIN) return;
    if(blend_dsc->mask_buf && blend_dsc->mask_res == LV_DRAW_SW_MASK_RES_TRANSP) return;

    // Clip to draw area
    lv_area_t blend_area;
    if(!lv_area_intersect(&blend_area, blend_dsc->blend_area, &t->clip_area)) return;

    lv_layer_t * layer = t->target_layer;
    
    // Get format-specific handler
    lv_color_format_t target_format = layer->color_format;
    
    if(blend_dsc->src_buf == NULL) {
        // FILL operation (solid color)
        lv_draw_sw_blend_fill_dsc_t fill_dsc;
        // ... populate fill_dsc ...
        
        lv_draw_sw_blend_color(target_format, &fill_dsc);
    }
    else {
        // IMAGE operation (source buffer)
        lv_draw_sw_blend_image_dsc_t image_dsc;
        // ... populate image_dsc ...
        
        lv_draw_sw_blend_image(target_format, &image_dsc);
    }
}
```

### Format Routers

LVGL has **two format routing functions** - one for solid colors, one for source images.

#### Router 1: Solid Color Fills

**Function:** `lv_draw_sw_blend_color()`  
**Location:** `lv_draw_sw_blend.c` lines 177-244 (static inline)  
**Purpose:** Routes solid color fills to format-specific handlers  
**Used by:** Rectangle fills, lines without images, solid fills from primitives

```c
static void lv_draw_sw_blend_color(lv_color_format_t layer_cf,
                                   lv_draw_sw_blend_fill_dsc_t * fill_dsc)
{
    switch(layer_cf) {
#if LV_DRAW_SW_SUPPORT_RGB565
        case LV_COLOR_FORMAT_RGB565:
            lv_draw_sw_blend_color_to_rgb565(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_RGB565_SWAPPED
        case LV_COLOR_FORMAT_RGB565_SWAPPED:
            lv_draw_sw_blend_color_to_rgb565_swapped(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_ARGB8888
        case LV_COLOR_FORMAT_ARGB8888:
            lv_draw_sw_blend_color_to_argb8888(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_RGB888
        case LV_COLOR_FORMAT_RGB888:
            lv_draw_sw_blend_color_to_rgb888(fill_dsc, 3);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_XRGB8888
        case LV_COLOR_FORMAT_XRGB8888:
            lv_draw_sw_blend_color_to_rgb888(fill_dsc, 4);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_L8
        case LV_COLOR_FORMAT_L8:
            lv_draw_sw_blend_color_to_l8(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_A8
        case LV_COLOR_FORMAT_A8:
            lv_draw_sw_blend_color_to_a8(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_AL88
        case LV_COLOR_FORMAT_AL88:
            lv_draw_sw_blend_color_to_al88(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_I1
        case LV_COLOR_FORMAT_I1:
            lv_draw_sw_blend_color_to_i1(fill_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_ARGB8888_PREMULTIPLIED
        case LV_COLOR_FORMAT_ARGB8888_PREMULTIPLIED:
            lv_draw_sw_blend_color_to_argb8888_premultiplied(fill_dsc);
            break;
#endif
        default:
            LV_LOG_WARN("Unsupported color format: %d", layer_cf);
            break;
    }
}
```

**Notes:**
- Each `#if` block is compile-time conditional - only enabled formats are compiled
- Handler functions live in separate files: `lv_draw_sw_blend_to_*.c`
- XRGB8888 shares handler with RGB888 (just different stride: 4 bytes vs 3)
- This is a **static inline** function for performance (inlined at call site)

#### Router 2: Image Blending

**Function:** `lv_draw_sw_blend_image()`  
**Location:** `lv_draw_sw_blend.c` lines 247-297 (static inline)  
**Purpose:** Routes image/source buffer blending to format-specific handlers  
**Used by:** Image drawing, texture mapping, source buffer blits, sprite rendering

```c
static inline void lv_draw_sw_blend_image(lv_color_format_t layer_cf,
                                          lv_draw_sw_blend_image_dsc_t * image_dsc)
{
    switch(layer_cf) {
#if LV_DRAW_SW_SUPPORT_RGB565
        case LV_COLOR_FORMAT_RGB565:
        case LV_COLOR_FORMAT_RGB565A8:  // RGB565 with separate alpha channel
            lv_draw_sw_blend_image_to_rgb565(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_RGB565_SWAPPED
        case LV_COLOR_FORMAT_RGB565_SWAPPED:
            lv_draw_sw_blend_image_to_rgb565_swapped(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_ARGB8888
        case LV_COLOR_FORMAT_ARGB8888:
            lv_draw_sw_blend_image_to_argb8888(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_RGB888
        case LV_COLOR_FORMAT_RGB888:
            lv_draw_sw_blend_image_to_rgb888(image_dsc, 3);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_XRGB8888
        case LV_COLOR_FORMAT_XRGB8888:
            lv_draw_sw_blend_image_to_rgb888(image_dsc, 4);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_ARGB8888_PREMULTIPLIED
        case LV_COLOR_FORMAT_ARGB8888_PREMULTIPLIED:
            lv_draw_sw_blend_image_to_argb8888_premultiplied(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_A8
        case LV_COLOR_FORMAT_A8:
            lv_draw_sw_blend_image_to_a8(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_L8
        case LV_COLOR_FORMAT_L8:
            lv_draw_sw_blend_image_to_l8(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_AL88
        case LV_COLOR_FORMAT_AL88:
            lv_draw_sw_blend_image_to_al88(image_dsc);
            break;
#endif
#if LV_DRAW_SW_SUPPORT_I1
        case LV_COLOR_FORMAT_I1:
            lv_draw_sw_blend_image_to_i1(image_dsc);
            break;
#endif
        default:
            LV_LOG_WARN("Unsupported color format: %d", layer_cf);
            break;
    }
}
```

**Notes:**
- Handles source images which may be in different formats than destination
- Format conversion happens inside handlers (e.g., RGB888 source → RGB565 dest)
- More complex than color fills due to source format variations
- Image descriptor includes source buffer, source format, blend mode, recolor

### Complete Format Handler Files

Each format has dedicated implementation files in `src/draw/sw/blend/`:

| File | Target Format | Approx Lines | Description |
|------|---------------|--------------|-------------|
| `lv_draw_sw_blend_to_rgb565.c` | RGB565 | 1532 | Most optimized, common in embedded |
| `lv_draw_sw_blend_to_rgb565_swapped.c` | RGB565_SWAPPED | ~800 | Byte-swapped RGB565 for certain displays |
| `lv_draw_sw_blend_to_argb8888.c` | ARGB8888 | ~900 | 32-bit with alpha channel |
| `lv_draw_sw_blend_to_argb8888_premultiplied.c` | ARGB8888_PREMUL | ~600 | Pre-multiplied alpha |
| `lv_draw_sw_blend_to_rgb888.c` | RGB888/XRGB8888 | ~800 | 24/32-bit RGB (no alpha) |
| `lv_draw_sw_blend_to_l8.c` | L8 | ~400 | 8-bit grayscale |
| `lv_draw_sw_blend_to_a8.c` | A8 | ~300 | 8-bit alpha only |
| `lv_draw_sw_blend_to_al88.c` | AL88 | ~500 | Alpha + luminance |
| `lv_draw_sw_blend_to_i1.c` | I1 | ~200 | 1-bit indexed (monochrome) |

Each file contains both `_color` and `_image` variants:
- `lv_draw_sw_blend_color_to_rgb565()` - solid color fills
- `lv_draw_sw_blend_image_to_rgb565()` - source image blending with format conversion

### Drawing Primitive Call Chain

**Example:** Drawing a filled rectangle

```
User Code: lv_obj_set_style_bg_color()
    ↓
lv_draw_rect(layer, &dsc)  [API layer - task submission]
    ↓
lv_draw_sw_fill(task, dsc, coords)  [lv_draw_sw_fill.c - format-independent]
    • Geometry calculations (clip to area)
    • Generate radius mask (if rounded corners)
    • Generate gradient map (if gradient fill)
    • Setup blend_dsc (color, area, mask, opacity)
    • NO FORMAT KNOWLEDGE HERE
    ↓
lv_draw_sw_blend(task, &blend_dsc)  [lv_draw_sw_blend.c - main dispatcher]
    • Check hardware acceleration hook
    • Extract layer->color_format
    • Populate fill_dsc or image_dsc
    • Decide: solid color or image?
    ↓
lv_draw_sw_blend_color(format, &fill_dsc)  [lv_draw_sw_blend.c - router]
    • switch(format) → route to format handler
    • FORMAT DECISION HAPPENS HERE (once per blend)
    ↓
lv_draw_sw_blend_color_to_rgb565(&fill_dsc)  [lv_draw_sw_blend_to_rgb565.c - handler]
    • Cast dest_buf to uint16_t*
    • Convert lv_color32_t to RGB565 (5:6:5 bit packing)
    • Write pixels scanline-by-scanline
    • Apply masks, opacity, alpha blending
```

**Example:** Drawing a horizontal line

```
User Code: lv_draw_line(layer, &line_dsc)
    ↓
lv_draw_sw_line(task, dsc)  [lv_draw_sw_line.c - format-independent]
    • Detect: horizontal, vertical, or diagonal?
    • Horizontal → convert to rectangle
    • draw_line_hor() creates lv_area_t for line width
    ↓
lv_draw_sw_fill(task, &fill_dsc, &line_area)  [lv_draw_sw_fill.c]
    • Horizontal line is now a thin rectangle
    • Same fill path as rectangles
    ↓
[continues same as rectangle above through blend router to format handler]
```

**Key Architectural Fact:** 
- The primitive layer (`lv_draw_sw_fill`, `lv_draw_sw_line`, `lv_draw_sw_arc`, etc.) **never knows** about `RGB565`, `ARGB8888`, or any format
- They only generate **abstract blend descriptors**: areas, masks, colors, opacity
- Format selection happens **exactly once** at the routing layer
- This means:
  - Adding a new format = implement 2 blend handlers (color + image)
  - All primitives automatically support the new format
  - Zero code duplication across formats
  - Clean separation of geometry vs pixel operations

---

## Rasterization Process

### Fill/Rectangle Rasterization

From `lv_draw_sw_fill.c` lines 48-95:

```c
void lv_draw_sw_fill(lv_draw_task_t * t, lv_draw_fill_dsc_t * dsc, const lv_area_t * coords)
{
    if(dsc->opa <= LV_OPA_MIN) return;

    // Clip to task area
    lv_area_t bg_coords;
    lv_area_copy(&bg_coords, coords);
    
    lv_area_t clipped_coords;
    if(!lv_area_intersect(&clipped_coords, &bg_coords, &t->clip_area)) return;

    lv_grad_dir_t grad_dir = dsc->grad.dir;
    lv_color_t bg_color = grad_dir == LV_GRAD_DIR_NONE ? dsc->color : dsc->grad.stops[0].color;

    lv_draw_sw_blend_dsc_t blend_dsc = {0};
    blend_dsc.color = bg_color;

    // SIMPLE CASE: No radius, no gradient
    if(dsc->radius == 0 && (grad_dir == LV_GRAD_DIR_NONE)) {
        blend_dsc.blend_area = &bg_coords;
        blend_dsc.opa = dsc->opa;
        lv_draw_sw_blend(t, &blend_dsc);  // Direct blend, no mask needed
        return;
    }

    // COMPLEX CASE: Radius or gradient requires mask
    lv_opa_t opa = dsc->opa >= LV_OPA_MAX ? LV_OPA_COVER : dsc->opa;
    
    // Calculate actual radius (can't exceed half of shortest side)
    int32_t coords_bg_w = lv_area_get_width(&bg_coords);
    int32_t coords_bg_h = lv_area_get_height(&bg_coords);
    int32_t short_side = LV_MIN(coords_bg_w, coords_bg_h);
    int32_t rout = LV_MIN(dsc->radius, short_side >> 1);
    
    // Create radius mask if needed
    int32_t clipped_w = lv_area_get_width(&clipped_coords);
    lv_opa_t * mask_buf = lv_malloc(clipped_w);  // One scanline of mask
    lv_draw_sw_mask_radius_param_t mask_rout_param;
    lv_draw_sw_mask_radius_init(&mask_rout_param, &bg_coords, rout, false);
    
    // Setup gradient if needed
    lv_draw_sw_grad_calc_t * grad = lv_draw_sw_grad_get(&dsc->grad, coords_bg_w, coords_bg_h);
    
    // Render scanline by scanline
    for(int32_t y = clipped_coords.y1; y <= clipped_coords.y2; y++) {
        // Generate mask for this scanline
        lv_draw_sw_mask_apply(&mask_buf, clipped_coords.x1, y, clipped_w);
        
        // Blend this scanline
        blend_dsc.mask_buf = mask_buf;
        blend_dsc.blend_area = &scanline_area;
        lv_draw_sw_blend(t, &blend_dsc);
    }
}
```

**Key Insights:**
1. Simple fills bypass masking entirely
2. Rounded corners use radius masks
3. Gradients generate color maps
4. Rasterization happens **scanline-by-scanline**
5. Masks are reused per scanline (memory efficient)

### Line Rasterization

From `lv_draw_sw_line.c` lines 50-110:

```c
void lv_draw_sw_line(lv_draw_task_t * t, const lv_draw_line_dsc_t * dsc)
{
    if(dsc->width == 0) return;
    if(dsc->opa <= LV_OPA_MIN) return;
    
    // Calculate bounding box
    lv_area_t clip_line;
    clip_line.x1 = LV_MIN(dsc->p1.x, dsc->p2.x) - dsc->width / 2;
    clip_line.x2 = LV_MAX(dsc->p1.x, dsc->p2.x) + dsc->width / 2;
    clip_line.y1 = LV_MIN(dsc->p1.y, dsc->p2.y) - dsc->width / 2;
    clip_line.y2 = LV_MAX(dsc->p1.y, dsc->p2.y) + dsc->width / 2;
    
    // Clip to drawing area
    bool is_common = lv_area_intersect(&clip_line, &clip_line, &t->clip_area);
    if(!is_common) return;

    // Dispatch to optimized routines
    if(dsc->p1.y == dsc->p2.y) draw_line_hor(t, dsc);      // Horizontal
    else if(dsc->p1.x == dsc->p2.x) draw_line_ver(t, dsc); // Vertical
    else draw_line_skew(t, dsc);                           // Diagonal
    
    // Add rounded ends if requested
    if(dsc->round_end || dsc->round_start) {
        lv_draw_fill_dsc_t cir_dsc;
        cir_dsc.radius = LV_RADIUS_CIRCLE;
        cir_dsc.color = dsc->color;
        cir_dsc.opa = dsc->opa;
        
        int32_t r = dsc->width >> 1;
        
        if(dsc->round_start) {
            lv_area_t cir_area;
            cir_area.x1 = dsc->p1.x - r;
            cir_area.y1 = dsc->p1.y - r;
            cir_area.x2 = dsc->p1.x + r;
            cir_area.y2 = dsc->p1.y + r;
            lv_draw_sw_fill(t, &cir_dsc, &cir_area);  // Draw circle at start
        }
        
        if(dsc->round_end) {
            // Similar for end point
        }
    }
}
```

**Horizontal Line** (optimized case):
```c
static void draw_line_hor(lv_draw_task_t * t, const lv_draw_line_dsc_t * dsc)
{
    int32_t w = dsc->width - 1;
    int32_t w_half0 = w >> 1;
    int32_t w_half1 = w_half0 + (w & 0x1);  // Compensate rounding

    lv_area_t blend_area;
    blend_area.x1 = LV_MIN(dsc->p1.x, dsc->p2.x);
    blend_area.x2 = LV_MAX(dsc->p1.x, dsc->p2.x) - 1;
    blend_area.y1 = dsc->p1.y - w_half1;
    blend_area.y2 = dsc->p1.y + w_half0;

    if(!lv_area_intersect(&blend_area, &blend_area, &t->clip_area)) return;

    // If no dashing, just fill rectangle
    if(!dsc->dash_gap || !dsc->dash_width) {
        lv_draw_sw_blend_dsc_t blend_dsc = {0};
        blend_dsc.blend_area = &blend_area;
        blend_dsc.color = dsc->color;
        blend_dsc.opa = dsc->opa;
        lv_draw_sw_blend(t, &blend_dsc);
    }
    else {
        // Generate dash mask and blend scanline-by-scanline
        // ...
    }
}
```

**Key Insights:**
1. Lines use specialized horizontal/vertical/diagonal routines
2. Horizontal lines are just rectangles
3. Dashed lines use masks
4. Rounded ends are separate circle fills
5. Thick lines expand bounding box by width/2

---

## Format-Specific Implementations

### RGB565 Fill Implementation

From `lv_draw_sw_blend_to_rgb565.c` (simplified):

```c
void lv_draw_sw_blend_color_to_rgb565(lv_draw_sw_blend_fill_dsc_t * dsc)
{
    lv_opa_t opa = dsc->opa;
    uint16_t * dest_buf = (uint16_t *)dsc->dest_buf;
    int32_t dest_stride = dsc->dest_stride / 2;  // Convert bytes to pixels
    
    // Convert 24-bit color to RGB565
    uint16_t color_rgb565 = lv_color_to_u16(dsc->color);
    
    // CASE 1: Full opacity, no mask (fastest path)
    if(opa >= LV_OPA_MAX && dsc->mask_buf == NULL) {
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            // Fill entire scanline with solid color
            lv_memset16(dest_buf, color_rgb565, dsc->dest_w);
            dest_buf += dest_stride;
        }
    }
    // CASE 2: With opacity but no mask
    else if(dsc->mask_buf == NULL) {
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            for(int32_t x = 0; x < dsc->dest_w; x++) {
                dest_buf[x] = lv_color_mix_rgb565(color_rgb565, dest_buf[x], opa);
            }
            dest_buf += dest_stride;
        }
    }
    // CASE 3: With mask (per-pixel opacity)
    else {
        lv_opa_t * mask_buf = dsc->mask_buf;
        int32_t mask_stride = dsc->mask_stride;
        
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            for(int32_t x = 0; x < dsc->dest_w; x++) {
                // Combine global opacity with mask
                lv_opa_t pixel_opa = (opa * mask_buf[x]) >> 8;
                if(pixel_opa > LV_OPA_MIN) {
                    dest_buf[x] = lv_color_mix_rgb565(color_rgb565, dest_buf[x], pixel_opa);
                }
            }
            dest_buf += dest_stride;
            mask_buf += mask_stride;
        }
    }
}
```

### RGB565 Color Mixing

```c
static inline uint16_t lv_color_mix_rgb565(uint16_t fg, uint16_t bg, uint8_t mix)
{
    // Extract RGB565 components
    uint8_t fg_r = (fg >> 11) & 0x1F;
    uint8_t fg_g = (fg >> 5) & 0x3F;
    uint8_t fg_b = fg & 0x1F;
    
    uint8_t bg_r = (bg >> 11) & 0x1F;
    uint8_t bg_g = (bg >> 5) & 0x3F;
    uint8_t bg_b = bg & 0x1F;
    
    // Alpha blend each component
    uint8_t out_r = ((fg_r * mix) + (bg_r * (255 - mix))) / 255;
    uint8_t out_g = ((fg_g * mix) + (bg_g * (255 - mix))) / 255;
    uint8_t out_b = ((fg_b * mix) + (bg_b * (255 - mix))) / 255;
    
    // Pack back to RGB565
    return (out_r << 11) | (out_g << 5) | out_b;
}
```

### RGB565 Image Blending

From `lv_draw_sw_blend_to_rgb565.c` lines 400-600:

```c
static void rgb565_image_blend(lv_draw_sw_blend_image_dsc_t * dsc)
{
    const uint16_t * src_buf = (const uint16_t *)dsc->src_buf;
    uint16_t * dest_buf = (uint16_t *)dsc->dest_buf;
    
    int32_t src_stride = dsc->src_stride / 2;
    int32_t dest_stride = dsc->dest_stride / 2;
    lv_opa_t opa = dsc->opa;
    
    // CASE 1: Direct copy (full opacity, no mask, normal blend mode)
    if(opa >= LV_OPA_MAX && dsc->mask_buf == NULL && 
       dsc->blend_mode == LV_BLEND_MODE_NORMAL) {
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            lv_memcpy(dest_buf, src_buf, dsc->dest_w * 2);
            src_buf += src_stride;
            dest_buf += dest_stride;
        }
    }
    // CASE 2: Alpha blending needed
    else {
        lv_opa_t * mask_buf = dsc->mask_buf;
        int32_t mask_stride = dsc->mask_stride;
        
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            for(int32_t x = 0; x < dsc->dest_w; x++) {
                lv_opa_t pixel_opa = opa;
                
                // Apply mask if present
                if(mask_buf) {
                    pixel_opa = (opa * mask_buf[x]) >> 8;
                }
                
                if(pixel_opa > LV_OPA_MIN) {
                    if(pixel_opa >= LV_OPA_MAX) {
                        dest_buf[x] = src_buf[x];  // Opaque copy
                    }
                    else {
                        dest_buf[x] = lv_color_mix_rgb565(src_buf[x], dest_buf[x], pixel_opa);
                    }
                }
            }
            src_buf += src_stride;
            dest_buf += dest_stride;
            if(mask_buf) mask_buf += mask_stride;
        }
    }
}
```

### ARGB8888 Implementation

From `lv_draw_sw_blend_to_argb8888.c`:

```c
void lv_draw_sw_blend_color_to_argb8888(lv_draw_sw_blend_fill_dsc_t * dsc)
{
    lv_color32_t * dest_buf = (lv_color32_t *)dsc->dest_buf;
    int32_t dest_stride = dsc->dest_stride / 4;  // 4 bytes per pixel
    
    lv_color32_t color_argb;
    color_argb.red = dsc->color.red;
    color_argb.green = dsc->color.green;
    color_argb.blue = dsc->color.blue;
    color_argb.alpha = dsc->opa;
    
    // Full opacity, no mask
    if(dsc->opa >= LV_OPA_MAX && dsc->mask_buf == NULL) {
        color_argb.alpha = 255;
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            for(int32_t x = 0; x < dsc->dest_w; x++) {
                dest_buf[x] = color_argb;
            }
            dest_buf += dest_stride;
        }
    }
    // Alpha blending
    else {
        for(int32_t y = 0; y < dsc->dest_h; y++) {
            for(int32_t x = 0; x < dsc->dest_w; x++) {
                lv_opa_t pixel_opa = dsc->opa;
                if(dsc->mask_buf) {
                    pixel_opa = (dsc->opa * dsc->mask_buf[x]) >> 8;
                }
                
                if(pixel_opa > LV_OPA_MIN) {
                    lv_color32_t result;
                    result.red   = ((color_argb.red   * pixel_opa) + (dest_buf[x].red   * (255 - pixel_opa))) / 255;
                    result.green = ((color_argb.green * pixel_opa) + (dest_buf[x].green * (255 - pixel_opa))) / 255;
                    result.blue  = ((color_argb.blue  * pixel_opa) + (dest_buf[x].blue  * (255 - pixel_opa))) / 255;
                    result.alpha = 255;  // Destination alpha is always opaque
                    dest_buf[x] = result;
                }
            }
            dest_buf += dest_stride;
            if(dsc->mask_buf) dsc->mask_buf += dsc->mask_stride;
        }
    }
}
```

### Format Conversion During Blending

LVGL automatically converts between formats during image blending:

```c
// From lv_draw_sw_blend_to_rgb565.c - blending RGB888 source to RGB565 target
static void rgb888_image_blend(lv_draw_sw_blend_image_dsc_t * dsc)
{
    const uint8_t * src_buf = dsc->src_buf;
    uint16_t * dest_buf = (uint16_t *)dsc->dest_buf;
    
    for(int32_t y = 0; y < dsc->dest_h; y++) {
        for(int32_t x = 0; x < dsc->dest_w; x++) {
            // Read RGB888 pixel
            uint8_t src_r = src_buf[x * 3 + 0];
            uint8_t src_g = src_buf[x * 3 + 1];
            uint8_t src_b = src_buf[x * 3 + 2];
            
            // Convert to RGB565
            uint16_t src_rgb565 = ((src_r >> 3) << 11) | 
                                  ((src_g >> 2) << 5) | 
                                  (src_b >> 3);
            
            // Blend
            lv_opa_t pixel_opa = calculate_opacity(dsc, x);
            dest_buf[x] = lv_color_mix_rgb565(src_rgb565, dest_buf[x], pixel_opa);
        }
        src_buf += dsc->src_stride;
        dest_buf += dsc->dest_stride / 2;
    }
}
```

---

## Pixel Operations

### Alpha Blending Formula

Standard alpha compositing (Porter-Duff "over" operator):

```
result = (src * alpha) + (dst * (1 - alpha))
```

In LVGL (using 8-bit alpha):

```c
// Component-wise blending
out_component = ((src_component * alpha) + (dst_component * (255 - alpha))) / 255;

// Optimized using bit shifts
out_component = ((src_component * alpha) + (dst_component * (255 - alpha))) >> 8;
```

### Pre-multiplied Alpha

For `LV_COLOR_FORMAT_ARGB8888_PREMULTIPLIED`:

```c
// Pre-multiplied alpha means RGB channels are already multiplied by alpha
// Formula: result = src + (dst * (1 - src_alpha))

void argb8888_premultiplied_blend(lv_draw_sw_blend_image_dsc_t * dsc)
{
    lv_color32_t * src = (lv_color32_t *)dsc->src_buf;
    lv_color32_t * dest = (lv_color32_t *)dsc->dest_buf;
    
    for(int32_t y = 0; y < dsc->dest_h; y++) {
        for(int32_t x = 0; x < dsc->dest_w; x++) {
            uint8_t inv_alpha = 255 - src[x].alpha;
            
            dest[x].red   = src[x].red   + ((dest[x].red   * inv_alpha) >> 8);
            dest[x].green = src[x].green + ((dest[x].green * inv_alpha) >> 8);
            dest[x].blue  = src[x].blue  + ((dest[x].blue  * inv_alpha) >> 8);
            dest[x].alpha = 255;
        }
        src += dsc->src_stride / 4;
        dest += dsc->dest_stride / 4;
    }
}
```

### Blend Modes

Beyond normal alpha blending, LVGL supports:

```c
typedef enum {
    LV_BLEND_MODE_NORMAL,      // Standard alpha blend
    LV_BLEND_MODE_ADDITIVE,    // result = src + dst
    LV_BLEND_MODE_SUBTRACTIVE, // result = dst - src
    LV_BLEND_MODE_MULTIPLY,    // result = src * dst
    LV_BLEND_MODE_REPLACE,     // result = src (no blending)
} lv_blend_mode_t;
```

**Additive Blending:**
```c
if(blend_mode == LV_BLEND_MODE_ADDITIVE) {
    result.red   = LV_MIN(255, src.red   + dest.red);
    result.green = LV_MIN(255, src.green + dest.green);
    result.blue  = LV_MIN(255, src.blue  + dest.blue);
}
```

**Multiply Blending:**
```c
if(blend_mode == LV_BLEND_MODE_MULTIPLY) {
    result.red   = (src.red   * dest.red)   / 255;
    result.green = (src.green * dest.green) / 255;
    result.blue  = (src.blue  * dest.blue)  / 255;
}
```

---

## Hardware Acceleration

### Acceleration Points

LVGL allows hardware acceleration at multiple levels:

1. **Custom Draw Units** - Replace entire rendering backend
2. **Custom Blend Handlers** - Accelerate specific format operations
3. **Assembly Optimizations** - SIMD for critical loops

### Custom Blend Handler

From `lv_draw_sw_blend.c` lines 75-85:

```c
void lv_draw_sw_blend(lv_draw_task_t * t, const lv_draw_sw_blend_dsc_t * blend_dsc)
{
    lv_layer_t * layer = t->target_layer;
    
    // Check for custom hardware handler
    lv_draw_sw_blend_handler_t handler = lv_draw_sw_get_blend_handler(layer->color_format);
    if(handler) {
        handler(t, blend_dsc);  // Use hardware acceleration
        return;
    }
    
    // Fall back to software rendering
    // ...
}
```

### SIMD Optimizations

LVGL supports NEON (ARM), Helium (Cortex-M), and custom assembly:

```c
// From lv_draw_sw_blend_to_rgb565.c
#if LV_USE_DRAW_SW_ASM == LV_DRAW_SW_ASM_NEON
    #include "neon/lv_blend_neon.h"
#elif LV_USE_DRAW_SW_ASM == LV_DRAW_SW_ASM_HELIUM
    #include "helium/lv_blend_helium.h"
#elif LV_USE_DRAW_SW_ASM == LV_DRAW_SW_ASM_CUSTOM
    #include LV_DRAW_SW_ASM_CUSTOM_INCLUDE
#endif

// In blending functions:
#if defined(LV_USE_NEON) && LV_USE_NEON
    result = lv_blend_neon_rgb565_fill(dest, color, width, height, opacity);
    if(result == LV_RESULT_OK) return;  // NEON succeeded
#endif

// Fallback to scalar code
for(int x = 0; x < width; x++) {
    dest[x] = scalar_blend(dest[x], color, opacity);
}
```

### DMA2D Example (STM32)

```c
// Custom blend handler using STM32 DMA2D (Chrom-ART)
lv_result_t dma2d_blend_fill(lv_draw_task_t * t, const lv_draw_sw_blend_dsc_t * dsc)
{
    // Only accelerate simple fills without masks
    if(dsc->mask_buf != NULL) return LV_RESULT_INVALID;
    if(dsc->opa < LV_OPA_MAX) return LV_RESULT_INVALID;
    
    // Configure DMA2D for solid fill
    DMA2D->FGPFCCR = format_to_dma2d(layer->color_format);
    DMA2D->FGCOLR = color_to_dma2d(dsc->color);
    DMA2D->OMAR = (uint32_t)dest_address;
    DMA2D->OOR = output_line_offset;
    DMA2D->NLR = (width << 16) | height;
    
    // Start DMA2D in register-to-memory mode
    DMA2D->CR = DMA2D_R2M | DMA2D_CR_START;
    
    // Wait for completion (or return and handle async)
    while(DMA2D->CR & DMA2D_CR_START);
    
    return LV_RESULT_OK;
}
```

---

## Summary

### Key Architectural Principles

1. **Format Independence**: Drawing operations are agnostic to pixel format
2. **Layered Architecture**: Rasterization → Blending → Format-specific writing
3. **Optimization Levels**: Simple paths bypass complex processing
4. **Mask-Based Coverage**: Soft edges use alpha masks, not antialiasing
5. **Scanline Processing**: Memory-efficient per-scanline rasterization
6. **Hardware Hooks**: Multiple acceleration points for GPUs/DMA

### Performance Characteristics

**Fastest Operations:**
- Simple fills (no radius, no mask): Direct memset/memcpy
- Horizontal/vertical lines: Rectangle fills
- Opaque blits: Direct memory copy

**Moderate Operations:**
- Alpha blending: Per-pixel math (can be SIMD-accelerated)
- Rounded corners: Scanline masking
- Dashed lines: Mask generation

**Expensive Operations:**
- Gradients: Color interpolation per pixel
- Complex shapes: Mask computation
- Format conversion: Per-pixel conversion during blend
- Blur: Multi-pass convolution

### Format Selection Guide

| Format | BPP | Use Case | Pros | Cons |
|--------|-----|----------|------|------|
| **RGB565** | 16 | General embedded | Good color, compact | No alpha |
| **ARGB8888** | 32 | High-end displays | Full color + alpha | Memory intensive |
| **RGB888** | 24 | Good color depth | Better than RGB565 | Awkward alignment |
| **L8** | 8 | Grayscale | Very compact | Monochrome only |
| **A8** | 8 | Masks/glyphs | Minimal memory | Alpha only |
| **AL88** | 16 | Grayscale + alpha | Balanced | Limited color |
| **I1/I2/I4/I8** | 1-8 | Indexed color | Tiny memory | Palette limited |

### Format-Specific Optimization

LVGL provides highly optimized implementations for each format:

- **RGB565**: Most optimized, common embedded target
- **ARGB8888**: Straightforward but memory-heavy
- **L8/A8**: Minimal conversions, direct operations
- **Indexed**: Palette lookup overhead

Each format has specialized blend functions avoiding unnecessary conversions.
