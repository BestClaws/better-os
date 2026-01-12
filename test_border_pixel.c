#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <SDL2/SDL.h>

// Minimal LVGL structs needed
typedef struct { int32_t x1, y1, x2, y2; } lv_area_t;
typedef uint8_t lv_opa_t;
typedef struct { uint8_t blue, green, red, alpha; } lv_color32_t;

#define LV_OPA_COVER 255

lv_color32_t blend_pixel(lv_color32_t bg, lv_color32_t fg, lv_opa_t opa) {
    if (opa >= 255) return fg;
    if (opa == 0) return bg;
    
    uint32_t inv_opa = 255 - opa;
    lv_color32_t result;
    result.red = ((fg.red * opa + bg.red * inv_opa) * 0x8081) >> 23;
    result.green = ((fg.green * opa + bg.green * inv_opa) * 0x8081) >> 23;
    result.blue = ((fg.blue * opa + bg.blue * inv_opa) * 0x8081) >> 23;
    result.alpha = opa;  // Store foreground opa
    return result;
}

int main() {
    // Pixel (26,30): background gray(50,50,50,255), border yellow(255,255,0) at mask=80
    lv_color32_t bg = {50, 50, 50, 255};
    lv_color32_t fg = {0, 255, 255, 255};  // BGR format
    lv_opa_t mask = 80;
    
    lv_color32_t result = blend_pixel(bg, fg, mask);
    printf("Input: bg=(%d,%d,%d,%d), fg=(%d,%d,%d), mask=%d\n",
           bg.red, bg.green, bg.blue, bg.alpha,
           fg.red, fg.green, fg.blue, mask);
    printf("Result: (%d,%d,%d,%d)\n", result.red, result.green, result.blue, result.alpha);
    printf("Expected from reference: (170,170,20,136)\n");
    
    return 0;
}
