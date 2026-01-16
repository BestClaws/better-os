#include "lvgl/lvgl.h"
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <math.h>

#define SDL_MAIN_HANDLED
#include "SDL2/SDL.h"

#define SPRITE_WIDTH 102
#define SPRITE_HEIGHT 125

// Global sprite counter
static int sprite_index = 0;
static lv_display_t *disp = NULL;
static uint8_t *display_buf = NULL;
static uint8_t *canvas_buf = NULL;
static lv_obj_t *screen = NULL;
static lv_obj_t *canvas = NULL;
static lv_layer_t layer;

// Helper to save current frame as individual file with descriptive name
static void capture_sprite(const char *name) {
    // Finish the current layer drawing
    lv_canvas_finish_layer(canvas, &layer);
    
    // Get the canvas buffer (already in ARGB8888 format)
    const uint8_t *buf = lv_canvas_get_buf(canvas);
    
    SDL_Surface *sprite_surf = SDL_CreateRGBSurfaceFrom(
        (void*)buf, SPRITE_WIDTH, SPRITE_HEIGHT, 32, SPRITE_WIDTH * 4,
        0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000);
    
    if (sprite_surf) {
        // Create filename for individual sprite
        char filename[256];
        snprintf(filename, sizeof(filename), "sprites/%04d_%s.bmp", sprite_index, name);
        
        // Save individual sprite
        SDL_SaveBMP(sprite_surf, filename);
        SDL_FreeSurface(sprite_surf);
    }
    
    sprite_index++;
    
    // Clear the canvas for the next sprite (transparent background)
    lv_canvas_fill_bg(canvas, lv_color_black(), LV_OPA_TRANSP);
    
    // Reinitialize layer for next drawing
    lv_canvas_init_layer(canvas, &layer);
}

// ============================================================================
// RECTANGLE PRIMITIVES
// ============================================================================

static void generate_rectangles(void) {
    lv_draw_rect_dsc_t rect_dsc;
    lv_area_t area;
    
    // Solid fills with various radius
    int32_t radii[] = {0, 5, 10, 20, LV_RADIUS_CIRCLE};
    const char *radius_names[] = {"r0", "r5", "r10", "r20", "rcircle"};
    lv_color_t colors[] = {
        lv_color_make(255, 100, 100), lv_color_make(100, 255, 100),
        lv_color_make(100, 100, 255), lv_color_make(255, 255, 100)
    };
    const char *color_names[] = {"red", "green", "blue", "yellow"};
    
    for (int r = 0; r < 5; r++) {
        for (int c = 0; c < 4; c++) {
            lv_draw_rect_dsc_init(&rect_dsc);
            area.x1 = 20; area.y1 = 30;
            area.x2 = 82; area.y2 = 95;
            
            rect_dsc.radius = radii[r];
            rect_dsc.bg_opa = LV_OPA_COVER;
            rect_dsc.bg_color = colors[c];
            
            char name[128];
            snprintf(name, sizeof(name), "rect_solid_%s_%s", radius_names[r], color_names[c]);
            lv_draw_rect(&layer, &rect_dsc, &area);
            capture_sprite(name);
        }
    }
    
    // Gradients (horizontal, vertical, radial, conical)
    lv_grad_dir_t grad_dirs[] = {LV_GRAD_DIR_HOR, LV_GRAD_DIR_VER, LV_GRAD_DIR_RADIAL, LV_GRAD_DIR_CONICAL};
    const char *grad_names[] = {"hor", "ver", "radial", "conical"};
    
    for (int g = 0; g < 4; g++) {
        for (int r = 0; r < 3; r++) {
            lv_draw_rect_dsc_init(&rect_dsc);
            area.x1 = 20; area.y1 = 30;
            area.x2 = 82; area.y2 = 95;

            rect_dsc.radius = radii[r];
            rect_dsc.bg_opa = LV_OPA_COVER;
            rect_dsc.bg_color = lv_color_make(255, 0, 0);
            rect_dsc.bg_grad.dir = grad_dirs[g];
            rect_dsc.bg_grad.stops[0].color = lv_color_make(255, 0, 0);
            rect_dsc.bg_grad.stops[0].opa = LV_OPA_COVER;
            rect_dsc.bg_grad.stops[0].frac = 0;
            rect_dsc.bg_grad.stops[1].color = lv_color_make(0, 0, 255);
            rect_dsc.bg_grad.stops[1].opa = LV_OPA_COVER;
            rect_dsc.bg_grad.stops[1].frac = 255;
            rect_dsc.bg_grad.stops_count = 2;

            char name[128];
            snprintf(name, sizeof(name), "rect_grad_%s_%s", grad_names[g], radius_names[r]);
            lv_draw_rect(&layer, &rect_dsc, &area);
            capture_sprite(name);
        }
    }

    int32_t grad_border_widths[] = {3, 6};
    const char *grad_border_width_names[] = {"w3", "w6"};
    lv_color_t grad_border_colors[] = {
        lv_color_make(255, 255, 255),
        lv_color_make(40, 40, 40)
    };
    const char *grad_border_color_names[] = {"white", "charcoal"};

    for (int g = 0; g < 4; g++) {
        for (int r = 0; r < 3; r++) {
            for (int b = 0; b < 2; b++) {
                lv_draw_rect_dsc_init(&rect_dsc);
                area.x1 = 20; area.y1 = 30;
                area.x2 = 82; area.y2 = 95;

                rect_dsc.radius = radii[r];
                rect_dsc.bg_opa = LV_OPA_COVER;
                rect_dsc.bg_color = lv_color_make(255, 0, 0);
                rect_dsc.bg_grad.dir = grad_dirs[g];
                rect_dsc.bg_grad.stops[0].color = lv_color_make(255, 0, 0);
                rect_dsc.bg_grad.stops[0].opa = LV_OPA_COVER;
                rect_dsc.bg_grad.stops[0].frac = 0;
                rect_dsc.bg_grad.stops[1].color = lv_color_make(0, 0, 255);
                rect_dsc.bg_grad.stops[1].opa = LV_OPA_COVER;
                rect_dsc.bg_grad.stops[1].frac = 255;
                rect_dsc.bg_grad.stops_count = 2;
                rect_dsc.border_opa = LV_OPA_COVER;
                rect_dsc.border_width = grad_border_widths[b];
                rect_dsc.border_color = grad_border_colors[b];
                rect_dsc.border_side = LV_BORDER_SIDE_FULL;

                char name[160];
                snprintf(name, sizeof(name), "rect_gradborder_%s_%s_%s_%s",
                         grad_names[g], radius_names[r],
                         grad_border_width_names[b], grad_border_color_names[b]);
                lv_draw_rect(&layer, &rect_dsc, &area);
                capture_sprite(name);
            }
        }
    }
    
    // Borders with various widths and sides
    int32_t border_widths[] = {1, 3, 6, 10};
    lv_border_side_t border_sides[] = {
        LV_BORDER_SIDE_FULL,
        LV_BORDER_SIDE_TOP | LV_BORDER_SIDE_BOTTOM,
        LV_BORDER_SIDE_LEFT | LV_BORDER_SIDE_RIGHT,
        LV_BORDER_SIDE_TOP
    };
    const char *border_side_names[] = {"full", "topbottom", "leftright", "top"};
    
    for (int w = 0; w < 4; w++) {
        for (int s = 0; s < 4; s++) {
            lv_draw_rect_dsc_init(&rect_dsc);
            area.x1 = 20; area.y1 = 30;
            area.x2 = 82; area.y2 = 95;
            
            rect_dsc.radius = 10;
            rect_dsc.bg_opa = LV_OPA_COVER;
            rect_dsc.bg_color = lv_color_make(50, 50, 50);
            rect_dsc.border_opa = LV_OPA_COVER;
            rect_dsc.border_width = border_widths[w];
            rect_dsc.border_color = lv_color_make(255, 255, 0);
            rect_dsc.border_side = border_sides[s];
            
            char name[128];
            snprintf(name, sizeof(name), "rect_border_w%d_%s", border_widths[w], border_side_names[s]);
            lv_draw_rect(&layer, &rect_dsc, &area);
            
            // Debug: print pixel at (26,30) for w1_full
            if (w == 0 && s == 0) {
                const uint8_t *buf = lv_canvas_get_buf(canvas);
                uint32_t *pixels = (uint32_t*)buf;
                uint32_t pixel_26_30 = pixels[30 * SPRITE_WIDTH + 26];
                uint8_t *rgba = (uint8_t*)&pixel_26_30;
                printf("LVGL pixel (26,30): ARGB=0x%08x = (%d,%d,%d,%d)\n", 
                       pixel_26_30, rgba[2], rgba[1], rgba[0], rgba[3]);
            }
            
            capture_sprite(name);
        }
    }
    
    // Shadows
    int32_t shadow_configs[][3] = {
        {2, 2, 0}, {5, 5, 0}, {8, 0, 4}, {3, 3, 6}
    };
    
    for (int i = 0; i < 4; i++) {
        for (int r = 0; r < 2; r++) {
            lv_draw_rect_dsc_init(&rect_dsc);
            area.x1 = 25; area.y1 = 35;
            area.x2 = 77; area.y2 = 90;
            
            rect_dsc.radius = r == 0 ? 0 : 15;
            rect_dsc.bg_opa = LV_OPA_COVER;
            rect_dsc.bg_color = lv_color_make(200, 200, 200);
            rect_dsc.shadow_opa = LV_OPA_50;
            rect_dsc.shadow_width = shadow_configs[i][0];
            rect_dsc.shadow_offset_x = shadow_configs[i][1];
            rect_dsc.shadow_offset_y = shadow_configs[i][1];
            rect_dsc.shadow_spread = shadow_configs[i][2];
            rect_dsc.shadow_color = lv_color_make(0, 0, 0);
            
            char name[128];
            snprintf(name, sizeof(name), "rect_shadow_w%d_off%d_spr%d_%s",
                     shadow_configs[i][0], shadow_configs[i][1], shadow_configs[i][2],
                     r == 0 ? "square" : "rounded");
            lv_draw_rect(&layer, &rect_dsc, &area);
            capture_sprite(name);
        }
    }
    
    // Outlines
    int32_t outline_configs[][2] = {{2, 2}, {4, 4}, {6, 1}, {3, 8}};
    
    for (int i = 0; i < 4; i++) {
        lv_draw_rect_dsc_init(&rect_dsc);
        area.x1 = 30; area.y1 = 40;
        area.x2 = 72; area.y2 = 85;
        
        rect_dsc.radius = 8;
        rect_dsc.bg_opa = LV_OPA_COVER;
        rect_dsc.bg_color = lv_color_make(150, 150, 150);
        rect_dsc.outline_opa = LV_OPA_COVER;
        rect_dsc.outline_width = outline_configs[i][0];
        rect_dsc.outline_pad = outline_configs[i][1];
        rect_dsc.outline_color = lv_color_make(0, 255, 255);
        
        char name[128];
        snprintf(name, sizeof(name), "rect_outline_w%d_pad%d",
                 outline_configs[i][0], outline_configs[i][1]);
        lv_draw_rect(&layer, &rect_dsc, &area);
        capture_sprite(name);
    }
    
    // Opacity variations
    lv_opa_t opas[] = {LV_OPA_COVER, LV_OPA_70, LV_OPA_50, LV_OPA_30};
    const char *opa_names[] = {"100", "70", "50", "30"};
    
    for (int o = 0; o < 4; o++) {
        lv_draw_rect_dsc_init(&rect_dsc);
        area.x1 = 20; area.y1 = 30;
        area.x2 = 82; area.y2 = 95;
        
        rect_dsc.radius = 12;
        rect_dsc.bg_opa = opas[o];
        rect_dsc.bg_color = lv_color_make(255, 150, 50);
        
        char name[128];
        snprintf(name, sizeof(name), "rect_opa%s", opa_names[o]);
        lv_draw_rect(&layer, &rect_dsc, &area);
        capture_sprite(name);
    }
}

// ============================================================================
// CIRCLE PRIMITIVES
// ============================================================================

static void generate_circles(void) {
    lv_draw_rect_dsc_t circle_dsc;
    lv_area_t area;

    lv_color_t fill_colors[] = {
        lv_color_make(255, 120, 120),
        lv_color_make(120, 255, 180),
        lv_color_make(120, 180, 255),
        lv_color_make(255, 220, 120)
    };
    const char *fill_names[] = {"coral", "mint", "sky", "sun"};

    for (int i = 0; i < 4; i++) {
        lv_draw_rect_dsc_init(&circle_dsc);
        area.x1 = 20; area.y1 = 25;
        area.x2 = 82; area.y2 = 87;

        circle_dsc.radius = LV_RADIUS_CIRCLE;
        circle_dsc.bg_opa = LV_OPA_COVER;
        circle_dsc.bg_color = fill_colors[i];

        char name[128];
        snprintf(name, sizeof(name), "circle_solid_%s", fill_names[i]);
        lv_draw_rect(&layer, &circle_dsc, &area);
        capture_sprite(name);
    }

    lv_grad_dir_t grad_dirs[] = {LV_GRAD_DIR_HOR, LV_GRAD_DIR_VER, LV_GRAD_DIR_RADIAL, LV_GRAD_DIR_CONICAL};
    const char *grad_names[] = {"hor", "ver", "radial", "conical"};

    for (int g = 0; g < 4; g++) {
        lv_draw_rect_dsc_init(&circle_dsc);
        area.x1 = 20; area.y1 = 25;
        area.x2 = 82; area.y2 = 87;

        circle_dsc.radius = LV_RADIUS_CIRCLE;
        circle_dsc.bg_opa = LV_OPA_COVER;
        circle_dsc.bg_color = lv_color_make(255, 0, 0);
        circle_dsc.bg_grad.dir = grad_dirs[g];
        circle_dsc.bg_grad.stops[0].color = lv_color_make(255, 0, 0);
        circle_dsc.bg_grad.stops[0].opa = LV_OPA_COVER;
        circle_dsc.bg_grad.stops[0].frac = 0;
        circle_dsc.bg_grad.stops[1].color = lv_color_make(0, 0, 255);
        circle_dsc.bg_grad.stops[1].opa = LV_OPA_COVER;
        circle_dsc.bg_grad.stops[1].frac = 255;
        circle_dsc.bg_grad.stops_count = 2;

        char name[128];
        snprintf(name, sizeof(name), "circle_grad_%s", grad_names[g]);
        lv_draw_rect(&layer, &circle_dsc, &area);
        capture_sprite(name);
    }

    int32_t border_widths[] = {2, 4, 8};
    const char *border_width_names[] = {"w2", "w4", "w8"};
    lv_color_t border_colors[] = {
        lv_color_make(255, 255, 255),
        lv_color_make(255, 200, 0),
        lv_color_make(80, 255, 255)
    };
    const char *border_color_names[] = {"white", "gold", "aqua"};

    for (int w = 0; w < 3; w++) {
        for (int c = 0; c < 3; c++) {
            lv_draw_rect_dsc_init(&circle_dsc);
            area.x1 = 20; area.y1 = 25;
            area.x2 = 82; area.y2 = 87;

            circle_dsc.radius = LV_RADIUS_CIRCLE;
            circle_dsc.bg_opa = LV_OPA_COVER;
            circle_dsc.bg_color = lv_color_make(45, 45, 45);
            circle_dsc.border_opa = LV_OPA_COVER;
            circle_dsc.border_width = border_widths[w];
            circle_dsc.border_color = border_colors[c];
            circle_dsc.border_side = LV_BORDER_SIDE_FULL;

            char name[160];
            snprintf(name, sizeof(name), "circle_border_%s_%s", border_width_names[w], border_color_names[c]);
            lv_draw_rect(&layer, &circle_dsc, &area);
            capture_sprite(name);
        }
    }

    for (int g = 0; g < 2; g++) {
        for (int w = 0; w < 2; w++) {
            lv_draw_rect_dsc_init(&circle_dsc);
            area.x1 = 20; area.y1 = 25;
            area.x2 = 82; area.y2 = 87;

            circle_dsc.radius = LV_RADIUS_CIRCLE;
            circle_dsc.bg_opa = LV_OPA_COVER;
            circle_dsc.bg_color = lv_color_make(255, 80, 0);
            circle_dsc.bg_grad.dir = grad_dirs[g];
            circle_dsc.bg_grad.stops[0].color = lv_color_make(255, 80, 0);
            circle_dsc.bg_grad.stops[0].opa = LV_OPA_COVER;
            circle_dsc.bg_grad.stops[0].frac = 0;
            circle_dsc.bg_grad.stops[1].color = lv_color_make(80, 0, 255);
            circle_dsc.bg_grad.stops[1].opa = LV_OPA_COVER;
            circle_dsc.bg_grad.stops[1].frac = 255;
            circle_dsc.bg_grad.stops_count = 2;
            circle_dsc.border_opa = LV_OPA_COVER;
            circle_dsc.border_width = border_widths[w + 1];
            circle_dsc.border_color = lv_color_make(255, 255, 255);
            circle_dsc.border_side = LV_BORDER_SIDE_FULL;

            char name[160];
            snprintf(name, sizeof(name), "circle_gradborder_%s_%s", grad_names[g], border_width_names[w + 1]);
            lv_draw_rect(&layer, &circle_dsc, &area);
            capture_sprite(name);
        }
    }
}

// ============================================================================
// TRIANGLE PRIMITIVES
// ============================================================================

static void generate_triangles(void) {
    lv_draw_triangle_dsc_t tri_dsc;
    lv_draw_line_dsc_t line_dsc;
    
    // Different orientations
    lv_point_precise_t triangles[][3] = {
        // Up
        {{51, 30}, {25, 90}, {77, 90}},
        // Down
        {{51, 90}, {25, 30}, {77, 30}},
        // Left
        {{25, 60}, {77, 30}, {77, 90}},
        // Right
        {{77, 60}, {25, 30}, {25, 90}},
        // Equilateral
        {{51, 25}, {20, 85}, {82, 85}},
        // Right-angled
        {{25, 35}, {25, 85}, {75, 85}}
    };
    const char *tri_orient_names[] = {"up", "down", "left", "right", "equi", "rightangle"};
    
    lv_color_t tri_colors[] = {
        lv_color_make(255, 100, 100),
        lv_color_make(100, 255, 100),
        lv_color_make(100, 100, 255),
        lv_color_make(255, 255, 100)
    };
    const char *tri_color_names[] = {"red", "green", "blue", "yellow"};
    
    for (int t = 0; t < 6; t++) {
        for (int c = 0; c < 4; c++) {
            lv_draw_triangle_dsc_init(&tri_dsc);
            tri_dsc.p[0] = triangles[t][0];
            tri_dsc.p[1] = triangles[t][1];
            tri_dsc.p[2] = triangles[t][2];
            tri_dsc.color = tri_colors[c];
            tri_dsc.opa = LV_OPA_COVER;
            
            char name[128];
            snprintf(name, sizeof(name), "tri_%s_%s", tri_orient_names[t], tri_color_names[c]);
            lv_draw_triangle(&layer, &tri_dsc);
            capture_sprite(name);
        }
    }
    
    // Gradients
    lv_grad_dir_t grad_dirs[] = {LV_GRAD_DIR_HOR, LV_GRAD_DIR_VER};
    const char *grad_names[] = {"hor", "ver"};
    
    for (int g = 0; g < 2; g++) {
        for (int t = 0; t < 3; t++) {
            lv_draw_triangle_dsc_init(&tri_dsc);
            tri_dsc.p[0] = triangles[t][0];
            tri_dsc.p[1] = triangles[t][1];
            tri_dsc.p[2] = triangles[t][2];
            tri_dsc.opa = LV_OPA_COVER;
            tri_dsc.grad.dir = grad_dirs[g];
            tri_dsc.grad.stops[0].color = lv_color_make(255, 0, 255);
            tri_dsc.grad.stops[0].opa = LV_OPA_COVER;
            tri_dsc.grad.stops[0].frac = 0;
            tri_dsc.grad.stops[1].color = lv_color_make(0, 255, 255);
            tri_dsc.grad.stops[1].opa = LV_OPA_COVER;
            tri_dsc.grad.stops[1].frac = 255;
            tri_dsc.grad.stops_count = 2;
            
            char name[128];
            snprintf(name, sizeof(name), "tri_grad_%s_%s", grad_names[g], tri_orient_names[t]);
            lv_draw_triangle(&layer, &tri_dsc);
            capture_sprite(name);
        }
    }
    
    // Opacity variations
    lv_opa_t opas[] = {LV_OPA_COVER, LV_OPA_70, LV_OPA_40};
    const char *opa_names[] = {"100", "70", "40"};
    
    for (int o = 0; o < 3; o++) {
        lv_draw_triangle_dsc_init(&tri_dsc);
        tri_dsc.p[0] = triangles[0][0];
        tri_dsc.p[1] = triangles[0][1];
        tri_dsc.p[2] = triangles[0][2];
        tri_dsc.color = lv_color_make(255, 128, 0);
        tri_dsc.opa = opas[o];
        
        char name[128];
        snprintf(name, sizeof(name), "tri_opa%s", opa_names[o]);
        lv_draw_triangle(&layer, &tri_dsc);
        capture_sprite(name);
    }

    int tri_border_indices[] = {0, 1, 4};
    int32_t border_widths[] = {2, 5};
    const char *border_width_names[] = {"w2", "w5"};
    lv_color_t border_colors[] = {
        lv_color_make(255, 255, 255),
        lv_color_make(255, 220, 0),
        lv_color_make(255, 105, 180)
    };
    const char *border_color_names[] = {"white", "gold", "pink"};

    for (int ti = 0; ti < 3; ti++) {
        int tri_idx = tri_border_indices[ti];
        for (int w = 0; w < 2; w++) {
            for (int c = 0; c < 3; c++) {
                lv_draw_triangle_dsc_init(&tri_dsc);
                tri_dsc.p[0] = triangles[tri_idx][0];
                tri_dsc.p[1] = triangles[tri_idx][1];
                tri_dsc.p[2] = triangles[tri_idx][2];
                tri_dsc.color = tri_colors[(ti + c) % 4];
                tri_dsc.opa = LV_OPA_COVER;

                lv_draw_triangle(&layer, &tri_dsc);

                lv_draw_line_dsc_init(&line_dsc);
                line_dsc.width = border_widths[w];
                line_dsc.color = border_colors[c];
                line_dsc.opa = LV_OPA_COVER;
                line_dsc.round_start = 1;
                line_dsc.round_end = 1;

                for (int edge = 0; edge < 3; edge++) {
                    line_dsc.p1 = triangles[tri_idx][edge];
                    line_dsc.p2 = triangles[tri_idx][(edge + 1) % 3];
                    lv_draw_line(&layer, &line_dsc);
                }

                char name[160];
                snprintf(name, sizeof(name), "tri_border_%s_%s_%s",
                         tri_orient_names[tri_idx], border_width_names[w], border_color_names[c]);
                capture_sprite(name);
            }
        }
    }
}

// ============================================================================
// LINE PRIMITIVES
// ============================================================================

static void generate_lines(void) {
    lv_draw_line_dsc_t line_dsc;
    
    // Various widths and angles
    int32_t widths[] = {1, 3, 6, 10};
    
    // Horizontal
    for (int w = 0; w < 4; w++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 15; line_dsc.p1.y = 62;
        line_dsc.p2.x = 87; line_dsc.p2.y = 62;
        line_dsc.width = widths[w];
        line_dsc.color = lv_color_make(255, 255, 255);
        line_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "line_hor_w%d", widths[w]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
    
    // Vertical
    for (int w = 0; w < 4; w++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 51; line_dsc.p1.y = 25;
        line_dsc.p2.x = 51; line_dsc.p2.y = 100;
        line_dsc.width = widths[w];
        line_dsc.color = lv_color_make(255, 255, 0);
        line_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "line_ver_w%d", widths[w]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
    
    // Diagonal
    for (int w = 0; w < 4; w++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 20; line_dsc.p1.y = 30;
        line_dsc.p2.x = 82; line_dsc.p2.y = 95;
        line_dsc.width = widths[w];
        line_dsc.color = lv_color_make(0, 255, 255);
        line_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "line_diag_w%d", widths[w]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
    
    // Dashed patterns
    int32_t dash_configs[][2] = {{5, 3}, {10, 5}, {2, 2}, {8, 2}};
    
    for (int d = 0; d < 4; d++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 15; line_dsc.p1.y = 62;
        line_dsc.p2.x = 87; line_dsc.p2.y = 62;
        line_dsc.width = 3;
        line_dsc.dash_width = dash_configs[d][0];
        line_dsc.dash_gap = dash_configs[d][1];
        line_dsc.color = lv_color_make(255, 100, 255);
        line_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "line_dash_w%d_g%d", dash_configs[d][0], dash_configs[d][1]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
    
    // Round caps
    uint8_t cap_configs[][2] = {{0, 0}, {1, 0}, {0, 1}, {1, 1}};
    const char *cap_names[] = {"none", "start", "end", "both"};
    
    for (int c = 0; c < 4; c++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 20; line_dsc.p1.y = 40;
        line_dsc.p2.x = 82; line_dsc.p2.y = 85;
        line_dsc.width = 8;
        line_dsc.round_start = cap_configs[c][0];
        line_dsc.round_end = cap_configs[c][1];
        line_dsc.color = lv_color_make(100, 255, 100);
        line_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "line_cap_%s", cap_names[c]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
    
    // Opacity
    lv_opa_t opas[] = {LV_OPA_COVER, LV_OPA_70, LV_OPA_40};
    const char *opa_names[] = {"100", "70", "40"};
    
    for (int o = 0; o < 3; o++) {
        lv_draw_line_dsc_init(&line_dsc);
        line_dsc.p1.x = 15; line_dsc.p1.y = 62;
        line_dsc.p2.x = 87; line_dsc.p2.y = 62;
        line_dsc.width = 5;
        line_dsc.color = lv_color_make(255, 50, 50);
        line_dsc.opa = opas[o];
        
        char name[128];
        snprintf(name, sizeof(name), "line_opa%s", opa_names[o]);
        lv_draw_line(&layer, &line_dsc);
        capture_sprite(name);
    }
}

// ============================================================================
// ARC PRIMITIVES
// ============================================================================

static void generate_arcs(void) {
    lv_draw_arc_dsc_t arc_dsc;
    
    int32_t widths[] = {3, 8, 15};
    
    // Quarter arcs at different positions
    int16_t start_angles[] = {0, 90, 180, 270};
    
    for (int w = 0; w < 3; w++) {
        for (int a = 0; a < 4; a++) {
            lv_draw_arc_dsc_init(&arc_dsc);
            arc_dsc.center.x = 51;
            arc_dsc.center.y = 62;
            arc_dsc.radius = 35;
            arc_dsc.start_angle = start_angles[a];
            arc_dsc.end_angle = start_angles[a] + 90;
            arc_dsc.width = widths[w];
            arc_dsc.color = lv_color_make(255, 200, 0);
            arc_dsc.opa = LV_OPA_COVER;
            
            char name[128];
            snprintf(name, sizeof(name), "arc_quarter_w%d_a%d", widths[w], start_angles[a]);
            lv_draw_arc(&layer, &arc_dsc);
            capture_sprite(name);
        }
    }
    
    // Different arc spans
    int16_t arc_spans[] = {45, 90, 180, 270};
    
    for (int s = 0; s < 4; s++) {
        lv_draw_arc_dsc_init(&arc_dsc);
        arc_dsc.center.x = 51;
        arc_dsc.center.y = 62;
        arc_dsc.radius = 35;
        arc_dsc.start_angle = 0;
        arc_dsc.end_angle = arc_spans[s];
        arc_dsc.width = 8;
        arc_dsc.color = lv_color_make(100, 255, 255);
        arc_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "arc_span%d", arc_spans[s]);
        lv_draw_arc(&layer, &arc_dsc);
        capture_sprite(name);
    }
    
    // Rounded ends
    for (int w = 0; w < 3; w++) {
        lv_draw_arc_dsc_init(&arc_dsc);
        arc_dsc.center.x = 51;
        arc_dsc.center.y = 62;
        arc_dsc.radius = 35;
        arc_dsc.start_angle = 45;
        arc_dsc.end_angle = 225;
        arc_dsc.width = widths[w];
        arc_dsc.rounded = 1;
        arc_dsc.color = lv_color_make(255, 100, 255);
        arc_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "arc_rounded_w%d", widths[w]);
        lv_draw_arc(&layer, &arc_dsc);
        capture_sprite(name);
    }
    
    // Opacity variations
    lv_opa_t opas[] = {LV_OPA_COVER, LV_OPA_70, LV_OPA_40};
    const char *opa_names[] = {"100", "70", "40"};
    
    for (int o = 0; o < 3; o++) {
        lv_draw_arc_dsc_init(&arc_dsc);
        arc_dsc.center.x = 51;
        arc_dsc.center.y = 62;
        arc_dsc.radius = 35;
        arc_dsc.start_angle = 0;
        arc_dsc.end_angle = 270;
        arc_dsc.width = 10;
        arc_dsc.color = lv_color_make(255, 50, 50);
        arc_dsc.opa = opas[o];
        
        char name[128];
        snprintf(name, sizeof(name), "arc_opa%s", opa_names[o]);
        lv_draw_arc(&layer, &arc_dsc);
        capture_sprite(name);
    }
    
    // Different colors
    lv_color_t colors[] = {
        lv_color_make(255, 0, 0), lv_color_make(0, 255, 0),
        lv_color_make(0, 0, 255), lv_color_make(255, 255, 0)
    };
    const char *color_names[] = {"red", "green", "blue", "yellow"};
    
    for (int c = 0; c < 4; c++) {
        lv_draw_arc_dsc_init(&arc_dsc);
        arc_dsc.center.x = 51;
        arc_dsc.center.y = 62;
        arc_dsc.radius = 35;
        arc_dsc.start_angle = 0;
        arc_dsc.end_angle = 180;
        arc_dsc.width = 6;
        arc_dsc.color = colors[c];
        arc_dsc.opa = LV_OPA_COVER;
        
        char name[128];
        snprintf(name, sizeof(name), "arc_color_%s", color_names[c]);
        lv_draw_arc(&layer, &arc_dsc);
        capture_sprite(name);
    }
}

// ============================================================================
// LABEL/TEXT PRIMITIVES
// ============================================================================

static void generate_labels(void) {
    lv_draw_label_dsc_t label_dsc;
    lv_area_t label_area;
    
    const char *texts[] = {"A", "AB", "ABC", "Text", "123", "!@#"};
    const char *text_names[] = {"A", "AB", "ABC", "Text", "123", "sym"};
    
    // Basic text with default font
    for (int t = 0; t < 6; t++) {
        lv_draw_label_dsc_init(&label_dsc);
        label_dsc.text = texts[t];
        label_dsc.font = LV_FONT_DEFAULT;
        label_dsc.color = lv_color_make(255, 255, 255);
        label_dsc.opa = LV_OPA_COVER;
        label_area.x1 = 30; label_area.y1 = 50;
        label_area.x2 = 72; label_area.y2 = 75;
        
        char name[128];
        snprintf(name, sizeof(name), "label_text_%s", text_names[t]);
        lv_draw_label(&layer, &label_dsc, &label_area);
        capture_sprite(name);
    }
    
    // Text decorations
    lv_text_decor_t decors[] = {LV_TEXT_DECOR_NONE, LV_TEXT_DECOR_UNDERLINE, LV_TEXT_DECOR_STRIKETHROUGH};
    const char *decor_names[] = {"none", "underline", "strike"};
    
    for (int d = 0; d < 3; d++) {
        lv_draw_label_dsc_init(&label_dsc);
        label_dsc.text = "Test";
        label_dsc.font = LV_FONT_DEFAULT;
        label_dsc.color = lv_color_make(255, 255, 0);
        label_dsc.opa = LV_OPA_COVER;
        label_dsc.decor = decors[d];
        label_area.x1 = 25; label_area.y1 = 50;
        label_area.x2 = 77; label_area.y2 = 75;
        
        char name[128];
        snprintf(name, sizeof(name), "label_decor_%s", decor_names[d]);
        lv_draw_label(&layer, &label_dsc, &label_area);
        capture_sprite(name);
    }
    
    // Letter spacing
    int32_t spacings[] = {0, 5, 10};
    
    for (int s = 0; s < 3; s++) {
        lv_draw_label_dsc_init(&label_dsc);
        label_dsc.text = "Abc";
        label_dsc.font = LV_FONT_DEFAULT;
        label_dsc.color = lv_color_make(100, 255, 255);
        label_dsc.opa = LV_OPA_COVER;
        label_dsc.letter_space = spacings[s];
        label_area.x1 = 15; label_area.y1 = 50;
        label_area.x2 = 87; label_area.y2 = 75;
        
        char name[128];
        snprintf(name, sizeof(name), "label_spacing%d", spacings[s]);
        lv_draw_label(&layer, &label_dsc, &label_area);
        capture_sprite(name);
    }
    
    // Opacity
    lv_opa_t opas[] = {LV_OPA_COVER, LV_OPA_70, LV_OPA_40};
    const char *opa_names[] = {"100", "70", "40"};
    
    for (int o = 0; o < 3; o++) {
        lv_draw_label_dsc_init(&label_dsc);
        label_dsc.text = "Text";
        label_dsc.font = LV_FONT_DEFAULT;
        label_dsc.color = lv_color_make(255, 100, 255);
        label_dsc.opa = opas[o];
        label_area.x1 = 25; label_area.y1 = 50;
        label_area.x2 = 77; label_area.y2 = 75;
        
        char name[128];
        snprintf(name, sizeof(name), "label_opa%s", opa_names[o]);
        lv_draw_label(&layer, &label_dsc, &label_area);
        capture_sprite(name);
    }
    
    // Different colors
    lv_color_t colors[] = {
        lv_color_make(255, 0, 0), lv_color_make(0, 255, 0),
        lv_color_make(0, 0, 255), lv_color_make(255, 128, 0)
    };
    const char *color_names[] = {"red", "green", "blue", "orange"};
    
    for (int c = 0; c < 4; c++) {
        lv_draw_label_dsc_init(&label_dsc);
        label_dsc.text = "123";
        label_dsc.font = LV_FONT_DEFAULT;
        label_dsc.color = colors[c];
        label_dsc.opa = LV_OPA_COVER;
        label_area.x1 = 30; label_area.y1 = 50;
        label_area.x2 = 72; label_area.y2 = 75;
        
        char name[128];
        snprintf(name, sizeof(name), "label_color_%s", color_names[c]);
        lv_draw_label(&layer, &label_dsc, &label_area);
        capture_sprite(name);
    }
}

// ============================================================================
// VECTOR GRAPHICS PRIMITIVES
// ============================================================================

#if LV_USE_VECTOR_GRAPHIC
static void generate_vector_graphics(void) {
    lv_draw_vector_dsc_t *vector_dsc = lv_draw_vector_dsc_create(&layer);
    if (!vector_dsc) {
        return;
    }

    lv_vector_path_t *star_path = lv_vector_path_create(LV_VECTOR_PATH_QUALITY_HIGH);
    if (!star_path) {
        lv_draw_vector_dsc_delete(vector_dsc);
        return;
    }

    // Build a star with a visible gradient fill
    lv_fpoint_t star_pts[] = {
        {51.0f, 25.0f}, {62.0f, 45.0f}, {85.0f, 48.0f}, {66.0f, 62.0f},
        {74.0f, 86.0f}, {51.0f, 72.0f}, {28.0f, 86.0f}, {36.0f, 62.0f},
        {17.0f, 48.0f}, {40.0f, 45.0f}
    };

    lv_vector_path_move_to(star_path, &star_pts[0]);
    for (uint32_t i = 1; i < sizeof(star_pts) / sizeof(star_pts[0]); i++) {
        lv_vector_path_line_to(star_path, &star_pts[i]);
    }
    lv_vector_path_close(star_path);

    lv_grad_stop_t star_stops[3];
    star_stops[0].color = lv_color_make(255, 90, 0);
    star_stops[0].opa = LV_OPA_COVER;
    star_stops[0].frac = 0;
    star_stops[1].color = lv_color_make(255, 0, 200);
    star_stops[1].opa = LV_OPA_COVER;
    star_stops[1].frac = 130;
    star_stops[2].color = lv_color_make(80, 200, 255);
    star_stops[2].opa = LV_OPA_COVER;
    star_stops[2].frac = 255;

    lv_draw_vector_dsc_set_fill_color(vector_dsc, lv_color_black());
    lv_draw_vector_dsc_set_fill_opa(vector_dsc, LV_OPA_COVER);
    lv_draw_vector_dsc_set_fill_linear_gradient(vector_dsc, 25.0f, 30.0f, 80.0f, 95.0f);
    lv_draw_vector_dsc_set_fill_gradient_color_stops(vector_dsc, star_stops, 3);
    lv_draw_vector_dsc_set_fill_rule(vector_dsc, LV_VECTOR_FILL_NONZERO);

    lv_draw_vector_dsc_set_stroke_color(vector_dsc, lv_color_make(255, 255, 255));
    lv_draw_vector_dsc_set_stroke_opa(vector_dsc, LV_OPA_70);
    lv_draw_vector_dsc_set_stroke_width(vector_dsc, 3.0f);

    lv_draw_vector_dsc_add_path(vector_dsc, star_path);
    lv_draw_vector(vector_dsc);

    lv_vector_path_delete(star_path);
    lv_draw_vector_dsc_delete(vector_dsc);
    capture_sprite("vector_star_gradient");

    vector_dsc = lv_draw_vector_dsc_create(&layer);
    if (!vector_dsc) {
        return;
    }

    lv_vector_path_t *wave_path = lv_vector_path_create(LV_VECTOR_PATH_QUALITY_HIGH);
    if (!wave_path) {
        lv_draw_vector_dsc_delete(vector_dsc);
        return;
    }

    // Use cubic curves to produce a dashed ribbon stroke
    lv_fpoint_t p0 = {22.0f, 82.0f};
    lv_fpoint_t c1 = {35.0f, 35.0f};
    lv_fpoint_t c2 = {65.0f, 95.0f};
    lv_fpoint_t p1 = {84.0f, 44.0f};
    lv_vector_path_move_to(wave_path, &p0);
    lv_vector_path_cubic_to(wave_path, &c1, &c2, &p1);

    lv_fpoint_t c3 = {70.0f, 24.0f};
    lv_fpoint_t c4 = {40.0f, 24.0f};
    lv_fpoint_t p2 = {26.0f, 46.0f};
    lv_vector_path_cubic_to(wave_path, &c3, &c4, &p2);

    lv_draw_vector_dsc_set_fill_opa(vector_dsc, LV_OPA_TRANSP);
    lv_draw_vector_dsc_set_stroke_color(vector_dsc, lv_color_make(120, 255, 120));
    lv_draw_vector_dsc_set_stroke_opa(vector_dsc, LV_OPA_COVER);
    lv_draw_vector_dsc_set_stroke_width(vector_dsc, 6.0f);
    lv_draw_vector_dsc_set_stroke_cap(vector_dsc, LV_VECTOR_STROKE_CAP_ROUND);
    lv_draw_vector_dsc_set_stroke_join(vector_dsc, LV_VECTOR_STROKE_JOIN_ROUND);

    float dash_pattern[] = {14.0f, 6.0f};
    lv_draw_vector_dsc_set_stroke_dash(vector_dsc, dash_pattern, 2);

    lv_grad_stop_t stroke_stops[2];
    stroke_stops[0].color = lv_color_make(120, 255, 120);
    stroke_stops[0].opa = LV_OPA_COVER;
    stroke_stops[0].frac = 0;
    stroke_stops[1].color = lv_color_make(0, 150, 255);
    stroke_stops[1].opa = LV_OPA_COVER;
    stroke_stops[1].frac = 255;
    lv_draw_vector_dsc_set_stroke_linear_gradient(vector_dsc, 22.0f, 82.0f, 84.0f, 44.0f);
    lv_draw_vector_dsc_set_stroke_gradient_color_stops(vector_dsc, stroke_stops, 2);

    lv_draw_vector_dsc_add_path(vector_dsc, wave_path);
    lv_draw_vector(vector_dsc);

    lv_vector_path_delete(wave_path);
    lv_draw_vector_dsc_delete(vector_dsc);
    capture_sprite("vector_wave_stroke");
}
#else
static lv_color_t fallback_star_color(float x, float y) {
    const float x0 = 25.0f;
    const float y0 = 30.0f;
    const float x1 = 80.0f;
    const float y1 = 95.0f;
    const float dx = x1 - x0;
    const float dy = y1 - y0;
    const float len_sq = dx * dx + dy * dy;
    float t = 0.0f;
    if(len_sq > 0.0f) {
        t = ((x - x0) * dx + (y - y0) * dy) / len_sq;
    }
    if(t < 0.0f) t = 0.0f;
    if(t > 1.0f) t = 1.0f;

    const float mid_frac = 130.0f / 255.0f;
    uint8_t r0 = 255, g0 = 90,  b0 = 0;
    uint8_t r1 = 255, g1 = 0,   b1 = 200;
    uint8_t r2 = 80,  g2 = 200, b2 = 255;

    float r, g, b;
    if(t <= mid_frac) {
        float lt = t / mid_frac;
        r = r0 + (r1 - r0) * lt;
        g = g0 + (g1 - g0) * lt;
        b = b0 + (b1 - b0) * lt;
    } else {
        float lt = (t - mid_frac) / (1.0f - mid_frac);
        r = r1 + (r2 - r1) * lt;
        g = g1 + (g2 - g1) * lt;
        b = b1 + (b2 - b1) * lt;
    }
    return lv_color_make((uint8_t)r, (uint8_t)g, (uint8_t)b);
}

static float fallback_wave_component(float p0, float c1, float c2, float p1, float t) {
    float it = 1.0f - t;
    return it * it * it * p0 + 3.0f * it * it * t * c1 + 3.0f * it * t * t * c2 + t * t * t * p1;
}

static void fallback_wave_point(float t, float *x, float *y) {
    if(t < 0.5f) {
        float lt = t * 2.0f;
        float p0x = 22.0f, p0y = 82.0f;
        float c1x = 35.0f, c1y = 35.0f;
        float c2x = 65.0f, c2y = 95.0f;
        float p1x = 84.0f, p1y = 44.0f;
        *x = fallback_wave_component(p0x, c1x, c2x, p1x, lt);
        *y = fallback_wave_component(p0y, c1y, c2y, p1y, lt);
    } else {
        float lt = (t - 0.5f) * 2.0f;
        float p0x = 84.0f, p0y = 44.0f;
        float c1x = 70.0f, c1y = 24.0f;
        float c2x = 40.0f, c2y = 24.0f;
        float p1x = 26.0f, p1y = 46.0f;
        *x = fallback_wave_component(p0x, c1x, c2x, p1x, lt);
        *y = fallback_wave_component(p0y, c1y, c2y, p1y, lt);
    }
}

static lv_color_t fallback_wave_color(float t) {
    if(t < 0.0f) t = 0.0f;
    if(t > 1.0f) t = 1.0f;
    uint8_t r0 = 120, g0 = 255, b0 = 120;
    uint8_t r1 = 0,   g1 = 150, b1 = 255;
    float r = r0 + (r1 - r0) * t;
    float g = g0 + (g1 - g0) * t;
    float b = b0 + (b1 - b0) * t;
    return lv_color_make((uint8_t)r, (uint8_t)g, (uint8_t)b);
}

static void generate_vector_graphics(void) {
    lv_draw_triangle_dsc_t tri_dsc;
    lv_draw_triangle_dsc_init(&tri_dsc);
    tri_dsc.opa = LV_OPA_COVER;

    lv_point_precise_t star_pts[] = {
        {51, 25}, {62, 45}, {85, 48}, {66, 62}, {74, 86},
        {51, 72}, {28, 86}, {36, 62}, {17, 48}, {40, 45}
    };
    lv_point_precise_t center = {51, 55};

    for(uint32_t i = 0; i < sizeof(star_pts) / sizeof(star_pts[0]); i++) {
        uint32_t next = (i + 1) % (sizeof(star_pts) / sizeof(star_pts[0]));
        tri_dsc.p[0] = center;
        tri_dsc.p[1] = star_pts[i];
        tri_dsc.p[2] = star_pts[next];
        float cx = (tri_dsc.p[0].x + tri_dsc.p[1].x + tri_dsc.p[2].x) / 3.0f;
        float cy = (tri_dsc.p[0].y + tri_dsc.p[1].y + tri_dsc.p[2].y) / 3.0f;
        tri_dsc.color = fallback_star_color(cx, cy);
        lv_draw_triangle(&layer, &tri_dsc);
    }

    lv_draw_line_dsc_t line_dsc;
    lv_draw_line_dsc_init(&line_dsc);
    line_dsc.width = 3;
    line_dsc.opa = LV_OPA_70;
    line_dsc.color = lv_color_make(255, 255, 255);
    line_dsc.round_start = 1;
    line_dsc.round_end = 1;

    for(uint32_t i = 0; i < sizeof(star_pts) / sizeof(star_pts[0]); i++) {
        uint32_t next = (i + 1) % (sizeof(star_pts) / sizeof(star_pts[0]));
        line_dsc.p1.x = star_pts[i].x;
        line_dsc.p1.y = star_pts[i].y;
        line_dsc.p2.x = star_pts[next].x;
        line_dsc.p2.y = star_pts[next].y;
        lv_draw_line(&layer, &line_dsc);
    }

    capture_sprite("vector_star_gradient");

    const uint32_t segments = 36;
    lv_draw_line_dsc_t wave_dsc;
    lv_draw_line_dsc_init(&wave_dsc);
    wave_dsc.width = 6;
    wave_dsc.opa = LV_OPA_COVER;
    wave_dsc.dash_width = 14;
    wave_dsc.dash_gap = 6;
    wave_dsc.round_start = 1;
    wave_dsc.round_end = 1;

    float prev_x, prev_y;
    fallback_wave_point(0.0f, &prev_x, &prev_y);

    for(uint32_t i = 1; i <= segments; i++) {
        float t = (float)i / (float)segments;
        float curr_x, curr_y;
        fallback_wave_point(t, &curr_x, &curr_y);
        wave_dsc.p1.x = prev_x;
        wave_dsc.p1.y = prev_y;
        wave_dsc.p2.x = curr_x;
        wave_dsc.p2.y = curr_y;
        wave_dsc.color = fallback_wave_color(t);
        lv_draw_line(&layer, &wave_dsc);
        prev_x = curr_x;
        prev_y = curr_y;
    }

    capture_sprite("vector_wave_stroke");
}
#endif

// ============================================================================
// BLUR PRIMITIVES
// ============================================================================

static void generate_blurs(void) {
    lv_draw_blur_dsc_t blur_dsc;
    lv_draw_rect_dsc_t rect_dsc;
    lv_area_t area;
    
    // Draw something to blur - use gradients to make blur more visible
    int32_t blur_radii[] = {2, 5, 10, 15};
    int32_t corner_radii[] = {0, 10, 20};
    const char *corner_names[] = {"square", "r10", "r20"};
    
    for (int b = 0; b < 4; b++) {
        for (int c = 0; c < 3; c++) {
            // Draw background rect with gradient
            lv_draw_rect_dsc_init(&rect_dsc);
            area.x1 = 20; area.y1 = 30;
            area.x2 = 82; area.y2 = 95;
            rect_dsc.radius = corner_radii[c];
            rect_dsc.bg_opa = LV_OPA_COVER;
            rect_dsc.bg_color = lv_color_make(255, 0, 0);
            
            // Add horizontal gradient for clearer blur visibility
            rect_dsc.bg_grad.dir = LV_GRAD_DIR_HOR;
            rect_dsc.bg_grad.stops[0].color = lv_color_make(255, 0, 0);
            rect_dsc.bg_grad.stops[0].opa = LV_OPA_COVER;
            rect_dsc.bg_grad.stops[0].frac = 0;
            rect_dsc.bg_grad.stops[1].color = lv_color_make(0, 0, 255);
            rect_dsc.bg_grad.stops[1].opa = LV_OPA_COVER;
            rect_dsc.bg_grad.stops[1].frac = 255;
            rect_dsc.bg_grad.stops_count = 2;
            
            lv_draw_rect(&layer, &rect_dsc, &area);
            
            // Apply blur
            lv_draw_blur_dsc_init(&blur_dsc);
            blur_dsc.blur_radius = blur_radii[b];
            blur_dsc.corner_radius = corner_radii[c];
            lv_draw_blur(&layer, &blur_dsc, &area);
            
            char name[128];
            snprintf(name, sizeof(name), "blur_r%d_%s", blur_radii[b], corner_names[c]);
            capture_sprite(name);
        }
    }
}

// ============================================================================
// MASK PRIMITIVES - Skipped (requires private headers)
// ============================================================================

// ============================================================================
// MAIN
// ============================================================================

static void dummy_flush_cb(lv_display_t *d, const lv_area_t *area, uint8_t *px_map) {
    (void)area; (void)px_map;
    lv_display_flush_ready(d);
}

int main(int argc, char *argv[]) {
    (void)argc; (void)argv;
    
    // Initialize
    SDL_Init(SDL_INIT_VIDEO);
    lv_init();
    
    // Create display
    disp = lv_display_create(SPRITE_WIDTH, SPRITE_HEIGHT);
    lv_display_set_color_format(disp, LV_COLOR_FORMAT_ARGB8888);
    
    size_t buf_size = SPRITE_WIDTH * SPRITE_HEIGHT * 4;
    display_buf = malloc(buf_size);
    canvas_buf = malloc(buf_size);
    memset(display_buf, 0, buf_size);
    memset(canvas_buf, 0, buf_size);
    
    lv_display_set_buffers(disp, display_buf, NULL, buf_size, LV_DISPLAY_RENDER_MODE_FULL);
    lv_display_set_flush_cb(disp, dummy_flush_cb);
    
    // Setup screen
    screen = lv_screen_active();
    lv_obj_set_style_bg_color(screen, lv_color_black(), 0);
    lv_obj_set_style_bg_opa(screen, LV_OPA_TRANSP, 0);
    lv_obj_set_style_pad_all(screen, 0, 0);
    lv_obj_remove_flag(screen, LV_OBJ_FLAG_SCROLLABLE);
    
    // Create canvas for drawing
    canvas = lv_canvas_create(screen);
    lv_canvas_set_buffer(canvas, canvas_buf, SPRITE_WIDTH, SPRITE_HEIGHT, LV_COLOR_FORMAT_ARGB8888);
    lv_obj_set_pos(canvas, 0, 0);
    lv_obj_set_style_pad_all(canvas, 0, 0);
    
    // Initialize layer for drawing
    lv_canvas_init_layer(canvas, &layer);
    
    // Create sprites directory if it doesn't exist
    system("mkdir -p sprites");
    
    printf("Generating individual sprite frames...\n");
    
    // Generate all primitives
    printf("Generating rectangles...\n");
    generate_rectangles();
    
    printf("Generating circles...\n");
    generate_circles();

    printf("Generating triangles...\n");
    generate_triangles();
    
    printf("Generating lines...\n");
    generate_lines();
    
    printf("Generating arcs...\n");
    generate_arcs();
    
    printf("Generating labels...\n");
    generate_labels();
    
    printf("Generating vector graphics...\n");
    generate_vector_graphics();

    printf("Generating blurs...\n");
    generate_blurs();
    
    printf("\nGenerated %d individual sprite frames\n", sprite_index);
    
    char cwd[1024];
    if (getcwd(cwd, sizeof(cwd)) != NULL) {
        printf("Saved sprites to: %s/sprites/\n", cwd);
    }
    
    // Cleanup
    free(display_buf);
    free(canvas_buf);
    SDL_Quit();
    
    return 0;
}
