// Test LVGL mask generation for specific pixels
#include "lvgl/lvgl.h"
#include <stdio.h>

int main() {
    lv_init();
    
    // Triangle up_red: p1=(51,30), p2=(25,90), p3=(77,90)
    lv_point_precise_t p1 = {51, 30};
    lv_point_precise_t p2 = {25, 90};
    lv_point_precise_t p3 = {77, 90};
    
    // Create the three line masks like LVGL triangle code does
    lv_draw_sw_mask_line_param_t mask_left, mask_right, mask_bottom;
    
    // After sorting, right side check gives right=true, so:
    // mask_left: (51,30)-(25,90), side=Right  
    // mask_right: (51,30)-(77,90), side=Left
    // mask_bottom: (25,90)-(77,90), side=Top
    
    lv_draw_sw_mask_line_points_init(&mask_left, 51, 30, 25, 90, LV_DRAW_SW_MASK_LINE_SIDE_RIGHT);
    lv_draw_sw_mask_line_points_init(&mask_right, 51, 30, 77, 90, LV_DRAW_SW_MASK_LINE_SIDE_LEFT);
    lv_draw_sw_mask_line_points_init(&mask_bottom, 25, 90, 77, 90, LV_DRAW_SW_MASK_LINE_SIDE_TOP);
    
    // Test the failing pixels
    int test_pixels[][2] = {
        {64, 39}, // LVGL=255, Rust=253
        {61, 43}, // LVGL=0, Rust=1
        {61, 46}, // LVGL=255, Rust=254
        {74, 46}, // LVGL=255, Rust=254
        {77, 50}, // LVGL=0, Rust=2
        {51, 69}, // LVGL=255, Rust=253
        {48, 73}, // LVGL=0, Rust=1
        {87, 76}, // LVGL=255, Rust=253
        {90, 80}, // LVGL=0, Rust=1
        {90, 83}, // LVGL=255, Rust=254
        {93, 87}, // LVGL=0, Rust=2
    };
    
    for (int i = 0; i < 11; i++) {
        int x = test_pixels[i][0];
        int y = test_pixels[i][1];
        
        printf("\nPixel (%d, %d):\n", x, y);
        
        // Apply masks one by one
        lv_opa_t mask_buf[1] = {255};
        lv_draw_sw_mask_res_t res1 = mask_left.dsc.cb(mask_buf, x, y, 1, &mask_left);
        printf("  mask_left: %d (res=%d)\n", mask_buf[0], res1);
        
        mask_buf[0] = 255;
        lv_draw_sw_mask_res_t res2 = mask_right.dsc.cb(mask_buf, x, y, 1, &mask_right);
        printf("  mask_right: %d (res=%d)\n", mask_buf[0], res2);
        
        mask_buf[0] = 255;
        lv_draw_sw_mask_res_t res3 = mask_bottom.dsc.cb(mask_buf, x, y, 1, &mask_bottom);
        printf("  mask_bottom: %d (res=%d)\n", mask_buf[0], res3);
        
        // Combine all three
        mask_buf[0] = 255;
        void* masks[4] = {&mask_left, &mask_right, &mask_bottom, NULL};
        lv_draw_sw_mask_res_t combined = lv_draw_sw_mask_apply(masks, mask_buf, x, y, 1);
        printf("  COMBINED: %d (res=%d)\n", mask_buf[0], combined);
    }
    
    return 0;
}
