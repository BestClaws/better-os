# LVGL Algorithm Reference Guide

**Purpose:** Implementation reference for GFX primitives based on LVGL v9.x  
**Date:** 11 January 2026

---

## Key LVGL Concepts Learned

### 1. Fill/Rectangle Algorithm

**Location:** `references/lvgl/src/draw/sw/lv_draw_sw_fill.c`

#### Simple Case (No Radius, No Gradient)
```c
// LVGL approach: Direct blend for simple rectangles
if(dsc->radius == 0 && (grad_dir == LV_GRAD_DIR_NONE)) {
    blend_dsc.blend_area = &bg_coords;
    blend_dsc.opa = dsc->opa;
    lv_draw_sw_blend(t, &blend_dsc);
    return;
}
```

**Takeaway:** Optimize the common case - plain rectangles go straight to blend without masks.

#### Complex Case (Rounded Corners)
```c
// 1. Create radius mask
mask_buf = lv_malloc(clipped_w);
lv_draw_sw_mask_radius_init(&mask_rout_param, &bg_coords, rout, false);

// 2. Draw top and bottom arcs (mirrored)
for(h = 0; h < rout; h++) {
    // Apply mask for this scanline
    lv_memset(mask_buf, opa, clipped_w);
    blend_dsc.mask_res = lv_draw_sw_mask_apply(mask_list, mask_buf, x, y, w);
    
    // Draw top line
    lv_draw_sw_blend(t, &blend_dsc);
    
    // Draw bottom line (mirrored)
    lv_draw_sw_blend(t, &blend_dsc);
}

// 3. Draw center (straight sides)
lv_draw_sw_blend(t, &blend_dsc);
```

**Takeaway:** 
- Use scanline approach for rounded corners
- Top and bottom arcs are rendered separately then mirrored
- Center is a simple rectangle

### 2. Blend Operation

**Location:** `references/lvgl/src/draw/sw/blend/lv_draw_sw_blend.c`

#### Blend Workflow
```c
void lv_draw_sw_blend(task, blend_dsc) {
    // 1. Early exit checks
    if(opa <= LV_OPA_MIN) return;
    if(mask_res == TRANSP) return;
    
    // 2. Clip to target area
    if(!lv_area_intersect(&blend_area, blend_area, clip_area)) return;
    
    // 3. Choose blend path
    if(src_buf == NULL) {
        // Fill blend (solid color)
        lv_draw_sw_blend_color(layer_cf, &fill_dsc);
    } else {
        // Image blend (source buffer)
        lv_draw_sw_blend_image(layer_cf, &image_dsc);
    }
}
```

**Key Points:**
- Separate code paths for fill vs image blending
- Always clip to target area first
- Format-specific blend handlers

### 3. RGB565 Blend Implementation

**Location:** `references/lvgl/src/draw/sw/blend/lv_draw_sw_blend_to_rgb565.c`

#### Color Blend Function (Fill)
```c
void lv_draw_sw_blend_color_to_rgb565(fill_dsc) {
    // Setup
    uint16_t * dest = fill_dsc->dest_buf;
    uint32_t stride = fill_dsc->dest_stride / 2; // Convert bytes to pixels
    uint16_t color = lv_color_to_u16(fill_dsc->color);
    
    // Fast paths based on opacity and mask
    if(mask_buf == NULL) {
        if(opa >= LV_OPA_MAX) {
            // Opaque, no mask - fastest path
            for(y = 0; y < h; y++) {
                lv_memcpy_u16(dest, color, w); // Fast fill
                dest += stride;
            }
        } else {
            // With opacity
            for(y = 0; y < h; y++) {
                for(x = 0; x < w; x++) {
                    dest[x] = lv_color_mix_rgb565(color, dest[x], opa);
                }
                dest += stride;
            }
        }
    } else {
        // With mask
        for(y = 0; y < h; y++) {
            for(x = 0; x < w; x++) {
                uint8_t mix = mask_buf[x];
                if(mix) dest[x] = lv_color_mix_rgb565(color, dest[x], mix);
            }
            dest += stride;
            mask_buf += mask_stride;
        }
    }
}
```

**Optimization Insights:**
1. **Fast path for opaque fills** - Use memcpy or fill operation
2. **Separate loop for each blend mode** - Avoids branching in inner loop
3. **Skip fully transparent pixels** - Early continue on mask == 0
4. **Process scanlines** - Better cache locality

### 4. Color Mixing (RGB565)

```c
// LVGL's RGB565 alpha blend
uint16_t lv_color_mix_rgb565(uint16_t fg, uint16_t bg, uint8_t opa) {
    if(opa >= LV_OPA_MAX) return fg;
    if(opa <= LV_OPA_MIN) return bg;
    
    // Extract channels
    uint16_t fg_r = (fg >> 11) & 0x1F;
    uint16_t fg_g = (fg >> 5) & 0x3F;
    uint16_t fg_b = fg & 0x1F;
    
    uint16_t bg_r = (bg >> 11) & 0x1F;
    uint16_t bg_g = (bg >> 5) & 0x3F;
    uint16_t bg_b = bg & 0x1F;
    
    // Blend
    uint16_t out_r = ((fg_r * opa) + (bg_r * (255 - opa))) / 255;
    uint16_t out_g = ((fg_g * opa) + (bg_g * (255 - opa))) / 255;
    uint16_t out_b = ((fg_b * opa) + (bg_b * (255 - opa))) / 255;
    
    return (out_r << 11) | (out_g << 5) | out_b;
}
```

**Note:** Our current implementation is correct - matches LVGL's algorithm.

### 5. Coordinate Mapping (Layer Buffer Area)

```c
// LVGL's layer structure
typedef struct {
    void * buf;
    lv_area_t buf_area;  // Which screen region buffer represents
    lv_area_t clip_area; // Clipping region
    // ...
} lv_layer_t;

// Convert screen coordinates to buffer index
void * lv_draw_layer_go_to_xy(layer, x, y) {
    int32_t buf_x = x - layer->buf_area.x1;
    int32_t buf_y = y - layer->buf_area.y1;
    return layer->buf + (buf_y * stride + buf_x) * bpp;
}
```

**Our Implementation:** ✅ Matches this approach

### 6. Gradient Algorithm

**For Horizontal/Vertical Gradients:**
```c
// Pre-calculate gradient color map
lv_grad_calc(grad_dsc, width, height);

// For each scanline
for(y = 0; y < h; y++) {
    if(VERT_GRADIENT) {
        color = gradient_map[y];
        // Draw whole line with this color
    } else { // HORIZ_GRADIENT
        // Draw line using gradient_map as source
        blend_image(gradient_map, ...);
    }
}
```

**Takeaway:** Pre-compute gradient, then use as color array

---

## Implementation Priorities (Based on LVGL Study)

### Phase 1: Core Blending (CURRENT)
✅ 1. Simple rectangle fill (no radius, no gradient)
- Fast path for opaque
- Slow path for transparent
✅ 2. RGB565 blend functions
✅ 3. Layer coordinate mapping

### Phase 2: Rounded Rectangles
4. Mask system for radius
5. Scanline rendering with masks
6. Arc/circle rasterization

### Phase 3: Gradients
7. Gradient pre-calculation
8. Horizontal/vertical gradient fills
9. Radial gradients (if needed)

### Phase 4: Advanced
10. Border rendering
11. Box shadows
12. Line rendering
13. Arc rendering

---

## Immediate Improvements Needed

### 1. Optimize Fill Implementation

**Current Issue:** Our fill does per-pixel coordinate mapping (slow)

**LVGL Approach:** Calculate row/column offsets once, then iterate

```rust
// Better approach (following LVGL):
fn fill_rect_rgb565_optimized(&mut self, rect: Rect, color: ColorAlpha) {
    let rgb565 = color.to_color().to_rgb565();
    let alpha = color.a;
    
    let buffer = self.layer.buffer_mut();
    let layer_width = self.layer.width();
    let buf_area = self.layer.buf_area();
    
    // Calculate buffer start offset ONCE
    let buf_y_start = (rect.y - buf_area.y) as usize;
    let buf_x_start = (rect.x - buf_area.x) as usize;
    
    if alpha == 255 {
        // FAST PATH: Opaque fill
        for y in 0..rect.height {
            let row_offset = ((buf_y_start + y as usize) * layer_width as usize + buf_x_start) * 2;
            let row = &mut buffer[row_offset..row_offset + rect.width as usize * 2];
            
            // Fill row with color
            for chunk in row.chunks_exact_mut(2) {
                chunk[0] = (rgb565 >> 8) as u8;
                chunk[1] = rgb565 as u8;
            }
        }
    } else {
        // SLOW PATH: With alpha blending
        // Similar but blend each pixel
    }
}
```

### 2. Add Mask Support

Following LVGL's mask system for rounded corners.

### 3. Add More Fast Paths

- Opaque fills → memset equivalent
- Fully clipped → early return
- Single pixel → direct write

---

## Reference Locations

**Fill Algorithm:**
- `references/lvgl/src/draw/sw/lv_draw_sw_fill.c`

**Blend Core:**
- `references/lvgl/src/draw/sw/blend/lv_draw_sw_blend.c`
- `references/lvgl/src/draw/sw/blend/lv_draw_sw_blend_to_rgb565.c`

**Masks:**
- `references/lvgl/src/draw/sw/lv_draw_sw_mask.c`
- `references/lvgl/src/draw/sw/lv_draw_sw_mask_rect.c`

**Gradients:**
- `references/lvgl/src/draw/sw/lv_draw_sw_grad.c`

**Border:**
- `references/lvgl/src/draw/sw/lv_draw_sw_border.c`

**Line:**
- `references/lvgl/src/draw/sw/lv_draw_sw_line.c`

**Arc:**
- `references/lvgl/src/draw/sw/lv_draw_sw_arc.c`

---

## Next Steps

1. ✅ Study LVGL algorithms (DONE)
2. **Optimize fill implementation** (use LVGL approach)
3. **Implement DrawingSurface::DrawTarget**
4. **Test with simple app**
5. Implement circle (following LVGL's arc code)
6. Implement line (following LVGL's line code)
7. Migrate all apps
