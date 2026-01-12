#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

typedef uint8_t lv_opa_t;
typedef struct { int32_t x1, y1, x2, y2; } lv_area_t;
typedef struct { int32_t x, y; } lv_point_t;

typedef struct {
    int32_t radius;
    lv_opa_t * cir_opa;
    uint16_t * opa_start_on_y;
    uint16_t * x_start_on_y;
    uint8_t * buf;
} lv_draw_sw_mask_radius_circle_dsc_t;

typedef struct {
    lv_area_t rect;
    int32_t radius;
    bool outer;
    lv_draw_sw_mask_radius_circle_dsc_t circle;
} lv_draw_sw_mask_radius_param_t;

// Include the full circ_calc_aa4 implementation from before
static void circ_init(lv_point_t * c, int32_t * tmp, int32_t radius) {
    c->x = radius;
    c->y = 0;
    *tmp = 1 - radius;
}

static bool circ_cont(lv_point_t * c) {
    return c->y <= c->x;
}

static void circ_next(lv_point_t * c, int32_t * tmp) {
    if(*tmp <= 0) {
        (*tmp) += 2 * c->y + 3;
    } else {
        (*tmp) += 2 * (c->y - c->x) + 5;
        c->x--;
    }
    c->y++;
}

static void circ_calc_aa4(lv_draw_sw_mask_radius_circle_dsc_t * c, int32_t radius) {
    if(radius == 0) return;
    c->radius = radius;

    c->buf = malloc(radius * 6 + 6);
    c->cir_opa = c->buf;
    c->opa_start_on_y = (uint16_t *)(c->buf + 2 * radius + 2);
    c->x_start_on_y = (uint16_t *)(c->buf + 4 * radius + 4);

    if(radius == 1) {
        c->cir_opa[0] = 180;
        c->opa_start_on_y[0] = 0;
        c->opa_start_on_y[1] = 1;
        c->x_start_on_y[0] = 0;
        return;
    }

    const size_t cir_xy_size = (radius + 1) * 2 * 2 * sizeof(int32_t);
    int32_t * cir_x = calloc(1, cir_xy_size);
    int32_t * cir_y = &cir_x[(radius + 1) * 2];

    uint32_t y_8th_cnt = 0;
    lv_point_t cp;
    int32_t tmp;
    circ_init(&cp, &tmp, radius * 4);
    int32_t i;

    uint32_t x_int[4];
    uint32_t x_fract[4];
    int32_t cir_size = 0;
    x_int[0] = cp.x >> 2;
    x_fract[0] = 0;

    while(circ_cont(&cp)) {
        for(i = 0; i < 4; i++) {
            circ_next(&cp, &tmp);
            if(circ_cont(&cp) == false) break;
            x_int[i] = cp.x >> 2;
            x_fract[i] = cp.x & 0x3;
        }
        if(i != 4) break;

        if(x_int[0] == x_int[3]) {
            cir_x[cir_size] = x_int[0];
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = x_fract[0] + x_fract[1] + x_fract[2] + x_fract[3];
            c->cir_opa[cir_size] *= 16;
            cir_size++;
        }
        else if(x_int[0] != x_int[1]) {
            cir_x[cir_size] = x_int[0];
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = x_fract[0];
            c->cir_opa[cir_size] *= 16;
            cir_size++;

            cir_x[cir_size] = x_int[0] - 1;
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = 1 * 4 + x_fract[1] + x_fract[2] + x_fract[3];
            c->cir_opa[cir_size] *= 16;
            cir_size++;
        }
        else if(x_int[0] != x_int[2]) {
            cir_x[cir_size] = x_int[0];
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = x_fract[0] + x_fract[1];
            c->cir_opa[cir_size] *= 16;
            cir_size++;

            cir_x[cir_size] = x_int[0] - 1;
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = 2 * 4 + x_fract[2] + x_fract[3];
            c->cir_opa[cir_size] *= 16;
            cir_size++;
        }
        else {
            cir_x[cir_size] = x_int[0];
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = x_fract[0] + x_fract[1] + x_fract[2];
            c->cir_opa[cir_size] *= 16;
            cir_size++;

            cir_x[cir_size] = x_int[0] - 1;
            cir_y[cir_size] = y_8th_cnt;
            c->cir_opa[cir_size] = 3 * 4 + x_fract[3];
            c->cir_opa[cir_size] *= 16;
            cir_size++;
        }

        y_8th_cnt++;
    }

    int32_t mid = radius * 723;
    int32_t mid_int = mid >> 10;
    if(cir_x[cir_size - 1] != mid_int || cir_y[cir_size - 1] != mid_int) {
        int32_t tmp_val = mid - (mid_int << 10);
        if(tmp_val <= 512) {
            tmp_val = tmp_val * tmp_val * 2;
            tmp_val = tmp_val >> (10 + 6);
        }
        else {
            tmp_val = 1024 - tmp_val;
            tmp_val = tmp_val * tmp_val * 2;
            tmp_val = tmp_val >> (10 + 6);
            tmp_val = 15 - tmp_val;
        }

        cir_x[cir_size] = mid_int;
        cir_y[cir_size] = mid_int;
        c->cir_opa[cir_size] = tmp_val;
        c->cir_opa[cir_size] *= 16;
        cir_size++;
    }

    for(i = cir_size - 2; i >= 0; i--, cir_size++) {
        cir_x[cir_size] = cir_y[i];
        cir_y[cir_size] = cir_x[i];
        c->cir_opa[cir_size] = c->cir_opa[i];
    }

    for(i = 0; i < radius + 1; i++) {
        c->opa_start_on_y[i] = 0xFFFF;
    }

    int32_t y;
    i = 0;
    c->opa_start_on_y[0] = 0;
    for(y = 0; i < cir_size; y++) {
        c->opa_start_on_y[y] = i;
        c->x_start_on_y[y] = cir_x[i];

        while(i < cir_size && cir_y[i] == y) {
            if(cir_x[i] < (int32_t)c->x_start_on_y[y]) c->x_start_on_y[y] = cir_x[i];
            i++;
        }
    }

    free(cir_x);
}

static lv_opa_t * get_next_line(lv_draw_sw_mask_radius_circle_dsc_t * c, int32_t y, int32_t * len_out, int32_t * x_start_out) {
    if(y < 0 || y >= c->radius) {
        *len_out = 0;
        return NULL;
    }
    *len_out = c->opa_start_on_y[y + 1] - c->opa_start_on_y[y];
    *x_start_out = c->x_start_on_y[y];
    return &c->cir_opa[c->opa_start_on_y[y]];
}

static uint8_t mask_mix(uint8_t a, uint8_t b) {
    return (a * b) / 255;
}

int main() {
    // Setup masks for border w1
    lv_draw_sw_mask_radius_param_t outer_mask = {0};
    lv_draw_sw_mask_radius_param_t inner_mask = {0};
    
    outer_mask.rect = (lv_area_t){20, 30, 82, 95};
    outer_mask.radius = 10;
    outer_mask.outer = false;
    circ_calc_aa4(&outer_mask.circle, 10);
    
    inner_mask.rect = (lv_area_t){21, 31, 81, 94};
    inner_mask.radius = 9;
    inner_mask.outer = true;
    circ_calc_aa4(&inner_mask.circle, 9);
    
    // Apply masks at y=30
    int y = 30;
    int width = 63;
    uint8_t mask_buf[63];
    memset(mask_buf, 255, width);
    
    // Apply inner mask
    lv_area_t *rect = &inner_mask.rect;
    bool outer = inner_mask.outer;
    int radius = inner_mask.radius;
    int w = rect->x2 - rect->x1 + 1;
    int h = rect->y2 - rect->y1 + 1;
    int abs_y = y - rect->y1;
    int cir_y = (abs_y < radius) ? (radius - abs_y - 1) : (abs_y - (h - radius));
    int aa_len, x_start;
    lv_opa_t *aa_opa = get_next_line(&inner_mask.circle, cir_y, &aa_len, &x_start);
    
    if(aa_opa) {
        int k = rect->x1 - 20;  // x_start = 20
        int cir_x_right = k + w - radius + x_start;
        int cir_x_left = k + radius - x_start - 1;
        
        printf("Inner mask (outer=%d, radius=%d):\n", outer, radius);
        printf("  cir_y=%d, aa_len=%d, x_start=%d\n", cir_y, aa_len, x_start);
        printf("  cir_x_left=%d, cir_x_right=%d\n", cir_x_left, cir_x_right);
        printf("  aa_opa=[");
        for(int i = 0; i < aa_len; i++) printf("%d%s", aa_opa[i], i < aa_len-1 ? ", " : "");
        printf("]\n");
        
        for(int i = 0; i < aa_len; i++) {
            lv_opa_t opa = 255 - (aa_opa[aa_len - 1 - i]);
            if(cir_x_left - i >= 0 && cir_x_left - i < width) {
                mask_buf[cir_x_left - i] = mask_mix(opa, mask_buf[cir_x_left - i]);
            }
        }
        int clr_start = (cir_x_left + 1 > 0) ? cir_x_left + 1 : 0;
        int clr_len = (cir_x_right - clr_start > 0) ? cir_x_right - clr_start : 0;
        if(clr_len > width - clr_start) clr_len = width - clr_start;
        memset(&mask_buf[clr_start], 0, clr_len);
    }
    
    printf("After inner mask at x=26 (idx=6): %d\n\n", mask_buf[6]);
    
    // Apply outer mask
    rect = &outer_mask.rect;
    outer = outer_mask.outer;
    radius = outer_mask.radius;
    w = rect->x2 - rect->x1 + 1;
    h = rect->y2 - rect->y1 + 1;
    abs_y = y - rect->y1;
    cir_y = (abs_y < radius) ? (radius - abs_y - 1) : (abs_y - (h - radius));
    aa_opa = get_next_line(&outer_mask.circle, cir_y, &aa_len, &x_start);
    
    if(aa_opa) {
        int k = rect->x1 - 20;
        int cir_x_right = k + w - radius + x_start;
        int cir_x_left = k + radius - x_start - 1;
        
        printf("Outer mask (outer=%d, radius=%d):\n", outer, radius);
        printf("  cir_y=%d, aa_len=%d, x_start=%d\n", cir_y, aa_len, x_start);
        printf("  cir_x_left=%d, cir_x_right=%d\n", cir_x_left, cir_x_right);
        printf("  aa_opa=[");
        for(int i = 0; i < aa_len; i++) printf("%d%s", aa_opa[i], i < aa_len-1 ? ", " : "");
        printf("]\n");
        
        for(int i = 0; i < aa_len; i++) {
            lv_opa_t opa = aa_opa[aa_len - i - 1];
            int left_idx = cir_x_left - i;
            if(left_idx >= 0 && left_idx < width) {
                printf("  i=%d: left_idx=%d, opa=%d (from aa_opa[%d]=%d), old_mask=%d, new_mask=%d\n",
                       i, left_idx, opa, aa_len-i-1, aa_opa[aa_len-i-1], 
                       mask_buf[left_idx], mask_mix(opa, mask_buf[left_idx]));
                mask_buf[left_idx] = mask_mix(opa, mask_buf[left_idx]);
            }
        }
        
        int right_clear = cir_x_right + aa_len;
        if(right_clear < 0) right_clear = 0;
        if(right_clear < width) memset(&mask_buf[right_clear], 0, width - right_clear);
        
        int left_clear = cir_x_left - aa_len + 1;
        if(left_clear > width) left_clear = width;
        if(left_clear > 0) memset(&mask_buf[0], 0, left_clear);
    }
    
    printf("\nAfter outer mask at x=26 (idx=6): %d\n", mask_buf[6]);
    
    free(outer_mask.circle.buf);
    free(inner_mask.circle.buf);
    return 0;
}
