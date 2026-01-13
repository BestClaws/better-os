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
    lv_draw_sw_mask_radius_circle_dsc_t *circle;
} lv_draw_sw_mask_radius_param_t;

typedef enum {
    LV_DRAW_SW_MASK_RES_TRANSP = 0,
    LV_DRAW_SW_MASK_RES_FULL_COVER = 1,
    LV_DRAW_SW_MASK_RES_CHANGED = 2,
    LV_DRAW_SW_MASK_RES_UNKNOWN = 3,
} lv_draw_sw_mask_res_t;

static inline int32_t lv_area_get_width(const lv_area_t * area)
{
    return area->x2 - area->x1 + 1;
}

static inline int32_t lv_area_get_height(const lv_area_t * area)
{
    return area->y2 - area->y1 + 1;
}

static inline int32_t LV_CLAMP(int32_t min_v, int32_t v, int32_t max_v)
{
    if(v < min_v) return min_v;
    if(v > max_v) return max_v;
    return v;
}

static void lv_memzero(void * buf, size_t size)
{
    memset(buf, 0, size);
}

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

static uint8_t mask_mix(uint8_t mask_act, uint8_t mask_new) {
    if(mask_new >= 255) return mask_act;
    if(mask_new == 0) return 0;
    uint32_t prod = (uint32_t)mask_act * (uint32_t)mask_new;
    return (uint8_t)((prod * 0x8081u) >> 23);
}

static lv_draw_sw_mask_res_t lv_draw_mask_radius(uint8_t * mask_buf, int32_t abs_x,
                                                 int32_t abs_y, int32_t len,
                                                 lv_draw_sw_mask_radius_param_t * p)
{
    bool outer = p->outer;
    int32_t radius = p->radius;
    lv_area_t rect = p->rect;

    if(!outer) {
        if(abs_y < rect.y1 || abs_y > rect.y2) {
            return LV_DRAW_SW_MASK_RES_TRANSP;
        }
    }
    else {
        if(abs_y < rect.y1 || abs_y > rect.y2) {
            return LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
    }

    if((abs_x >= rect.x1 + radius && abs_x + len <= rect.x2 - radius) ||
       (abs_y >= rect.y1 + radius && abs_y <= rect.y2 - radius)) {
        if(!outer) {
            int32_t last = rect.x1 - abs_x;
            if(last > len) return LV_DRAW_SW_MASK_RES_TRANSP;
            if(last > 0) lv_memzero(&mask_buf[0], last);

            int32_t first = rect.x2 - abs_x + 1;
            if(first <= 0) return LV_DRAW_SW_MASK_RES_TRANSP;
            if(first < len) lv_memzero(&mask_buf[first], len - first);
            if(last == 0 && first == len) return LV_DRAW_SW_MASK_RES_FULL_COVER;
            return LV_DRAW_SW_MASK_RES_CHANGED;
        }
        else {
            int32_t first = rect.x1 - abs_x;
            if(first < 0) first = 0;
            if(first <= len) {
                int32_t last = rect.x2 - abs_x - first + 1;
                if(first + last > len) last = len - first;
                if(last > 0) lv_memzero(&mask_buf[first], last);
            }
            return LV_DRAW_SW_MASK_RES_CHANGED;
        }
    }

    lv_draw_sw_mask_radius_circle_dsc_t * circle = p->circle;
    if(!circle) return LV_DRAW_SW_MASK_RES_CHANGED;

    int32_t k = rect.x1 - abs_x;
    int32_t w = lv_area_get_width(&rect);
    int32_t h = lv_area_get_height(&rect);
    int32_t rel_x = abs_x - rect.x1;
    (void)rel_x;
    int32_t rel_y = abs_y - rect.y1;

    int32_t cir_y;
    if(rel_y < radius) {
        cir_y = radius - rel_y - 1;
    }
    else {
        cir_y = rel_y - (h - radius);
    }

    int32_t aa_len = 0;
    int32_t x_start = 0;
    lv_opa_t * aa_opa = get_next_line(circle, cir_y, &aa_len, &x_start);
    int32_t cir_x_right = k + w - radius + x_start;
    int32_t cir_x_left = k + radius - x_start - 1;

    if(!outer) {
        for(int32_t i = 0; i < aa_len; i++) {
            lv_opa_t opa = aa_opa[aa_len - i - 1];
            int32_t right_idx = cir_x_right + i;
            if(right_idx >= 0 && right_idx < len) {
                mask_buf[right_idx] = mask_mix(mask_buf[right_idx], opa);
            }
            int32_t left_idx = cir_x_left - i;
            if(left_idx >= 0 && left_idx < len) {
                mask_buf[left_idx] = mask_mix(mask_buf[left_idx], opa);
            }
        }

        cir_x_right = LV_CLAMP(0, cir_x_right + aa_len, len);
        if(cir_x_right < len) {
            lv_memzero(&mask_buf[cir_x_right], len - cir_x_right);
        }

        cir_x_left = LV_CLAMP(0, cir_x_left - aa_len + 1, len);
        if(cir_x_left > 0) {
            lv_memzero(&mask_buf[0], cir_x_left);
        }
    }
    else {
        for(int32_t i = 0; i < aa_len; i++) {
            lv_opa_t opa = 255 - aa_opa[aa_len - 1 - i];
            int32_t right_idx = cir_x_right + i;
            if(right_idx >= 0 && right_idx < len) {
                mask_buf[right_idx] = mask_mix(mask_buf[right_idx], opa);
            }
            int32_t left_idx = cir_x_left - i;
            if(left_idx >= 0 && left_idx < len) {
                mask_buf[left_idx] = mask_mix(mask_buf[left_idx], opa);
            }
        }

        int32_t clr_start = LV_CLAMP(0, cir_x_left + 1, len);
        int32_t clr_end = LV_CLAMP(clr_start, cir_x_right, len);
        if(clr_end > clr_start) {
            lv_memzero(&mask_buf[clr_start], clr_end - clr_start);
        }
    }

    return LV_DRAW_SW_MASK_RES_CHANGED;
}

int main() {
    // Setup masks for border w1
    lv_draw_sw_mask_radius_param_t outer_mask = {0};
    lv_draw_sw_mask_radius_param_t inner_mask = {0};
    
    outer_mask.rect = (lv_area_t){16, 27, 86, 97};
    outer_mask.radius = 35;
    outer_mask.outer = false;
    outer_mask.circle = malloc(sizeof(lv_draw_sw_mask_radius_circle_dsc_t));
    memset(outer_mask.circle, 0, sizeof(*outer_mask.circle));
    circ_calc_aa4(outer_mask.circle, 35);

    inner_mask.rect = (lv_area_t){19, 30, 83, 94};
    inner_mask.radius = 32;
    inner_mask.outer = true;
    inner_mask.circle = malloc(sizeof(lv_draw_sw_mask_radius_circle_dsc_t));
    memset(inner_mask.circle, 0, sizeof(*inner_mask.circle));
    circ_calc_aa4(inner_mask.circle, 32);
    
    // Apply masks at y=60 to match Rust probe
    int width = outer_mask.rect.x2 - outer_mask.rect.x1 + 1;
    for(int y = 60; y <= 64; y++) {
        uint8_t mask_buf[128];
        memset(mask_buf, 255, sizeof(mask_buf));

        uint8_t mask_after_inner[128];
        memcpy(mask_after_inner, mask_buf, sizeof(mask_after_inner));

        lv_draw_sw_mask_res_t res_inner = lv_draw_mask_radius(mask_after_inner, outer_mask.rect.x1, y,
                                                              width, &inner_mask);

        printf("y=%d inner res=%d\n", y, res_inner);
        int start = 0;
        while(start < width) {
            int val = mask_after_inner[start];
            int end = start;
            while(end + 1 < width && mask_after_inner[end + 1] == val) {
                end++;
            }
            printf("  [%d,%d]=%d\n", start, end, val);
            start = end + 1;
        }

        uint8_t mask_after_both[128];
        memcpy(mask_after_both, mask_after_inner, sizeof(mask_after_both));
        lv_draw_sw_mask_res_t res_outer = lv_draw_mask_radius(mask_after_both, outer_mask.rect.x1, y,
                                                              width, &outer_mask);

        printf("y=%d outer res=%d\n", y, res_outer);
        start = 0;
        while(start < width) {
            int val = mask_after_both[start];
            int end = start;
            while(end + 1 < width && mask_after_both[end + 1] == val) {
                end++;
            }
            printf("  [%d,%d]=%d\n", start, end, val);
            start = end + 1;
        }
        printf("\n");
    }
    
    free(outer_mask.circle->buf);
    free(outer_mask.circle);
    free(inner_mask.circle->buf);
    free(inner_mask.circle);
    return 0;
}
