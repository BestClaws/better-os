// Standalone harness mirroring LVGL radius mask output for a single case.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define LV_MIN(a, b) ((a) < (b) ? (a) : (b))
#define LV_MAX(a, b) ((a) > (b) ? (a) : (b))

typedef uint8_t lv_opa_t;

typedef struct {
    int32_t x1;
    int32_t y1;
    int32_t x2;
    int32_t y2;
} lv_area_t;

typedef struct {
    int32_t x;
    int32_t y;
} lv_point_t;

typedef struct {
    int32_t radius;
    lv_opa_t *cir_opa;
    uint16_t *opa_start_on_y;
    uint16_t *x_start_on_y;
    uint8_t *buf;
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
} lv_draw_sw_mask_res_t;

static inline int32_t lv_area_get_width(const lv_area_t *area) {
    return area->x2 - area->x1 + 1;
}

static inline int32_t lv_area_get_height(const lv_area_t *area) {
    return area->y2 - area->y1 + 1;
}

static inline int32_t lv_clamp(int32_t min_v, int32_t v, int32_t max_v) {
    if(v < min_v) return min_v;
    if(v > max_v) return max_v;
    return v;
}

static inline void lv_memzero(void *buf, size_t size) {
    memset(buf, 0, size);
}

static lv_opa_t *get_next_line(lv_draw_sw_mask_radius_circle_dsc_t *c, int32_t y, int32_t *len_out, int32_t *x_start_out) {
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

static void circ_init(lv_point_t *c, int32_t *tmp, int32_t radius) {
    c->x = radius;
    c->y = 0;
    *tmp = 1 - radius;
}

static bool circ_cont(const lv_point_t *c) {
    return c->y <= c->x;
}

static void circ_next(lv_point_t *c, int32_t *tmp) {
    if(*tmp <= 0) {
        (*tmp) += 2 * c->y + 3;
    }
    else {
        (*tmp) += 2 * (c->y - c->x) + 5;
        c->x--;
    }
    c->y++;
}

static void circ_calc_aa4(lv_draw_sw_mask_radius_circle_dsc_t *c, int32_t radius) {
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
    int32_t *cir_x = calloc(1, cir_xy_size);
    int32_t *cir_y = &cir_x[(radius + 1) * 2];

    uint32_t y_8th_cnt = 0;
    lv_point_t cp;
    int32_t tmp;
    circ_init(&cp, &tmp, radius * 4);
    int32_t i;

    uint32_t x_int[4];
    uint32_t x_fract[4];
    int32_t cir_size = 0;
    x_int[0] = (uint32_t)cp.x >> 2;
    x_fract[0] = 0;

    while(circ_cont(&cp)) {
        for(i = 0; i < 4; i++) {
            circ_next(&cp, &tmp);
            if(!circ_cont(&cp)) break;
            x_int[i] = (uint32_t)cp.x >> 2;
            x_fract[i] = (uint32_t)cp.x & 0x3;
        }
        if(i != 4) break;

        if(x_int[0] == x_int[3]) {
            cir_x[cir_size] = (int32_t)x_int[0];
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((x_fract[0] + x_fract[1] + x_fract[2] + x_fract[3]) * 16);
            cir_size++;
        }
        else if(x_int[0] != x_int[1]) {
            cir_x[cir_size] = (int32_t)x_int[0];
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)(x_fract[0] * 16);
            cir_size++;

            cir_x[cir_size] = (int32_t)x_int[0] - 1;
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((4 + x_fract[1] + x_fract[2] + x_fract[3]) * 16);
            cir_size++;
        }
        else if(x_int[0] != x_int[2]) {
            cir_x[cir_size] = (int32_t)x_int[0];
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((x_fract[0] + x_fract[1]) * 16);
            cir_size++;

            cir_x[cir_size] = (int32_t)x_int[0] - 1;
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((8 + x_fract[2] + x_fract[3]) * 16);
            cir_size++;
        }
        else {
            cir_x[cir_size] = (int32_t)x_int[0];
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((x_fract[0] + x_fract[1] + x_fract[2]) * 16);
            cir_size++;

            cir_x[cir_size] = (int32_t)x_int[0] - 1;
            cir_y[cir_size] = (int32_t)y_8th_cnt;
            c->cir_opa[cir_size] = (lv_opa_t)((12 + x_fract[3]) * 16);
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
            tmp_val >>= 16;
        }
        else {
            tmp_val = 1024 - tmp_val;
            tmp_val = tmp_val * tmp_val * 2;
            tmp_val >>= 16;
            tmp_val = 15 - tmp_val;
        }

        cir_x[cir_size] = mid_int;
        cir_y[cir_size] = mid_int;
        c->cir_opa[cir_size] = (lv_opa_t)(tmp_val * 16);
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
        c->x_start_on_y[y] = (uint16_t)cir_x[i];

        while(i < cir_size && cir_y[i] == y) {
            if(cir_x[i] < (int32_t)c->x_start_on_y[y]) c->x_start_on_y[y] = (uint16_t)cir_x[i];
            i++;
        }
    }

    free(cir_x);
}

static lv_draw_sw_mask_res_t lv_draw_mask_radius(uint8_t *mask_buf, int32_t abs_x, int32_t abs_y,
                                                 int32_t len, lv_draw_sw_mask_radius_param_t *p) {
    const int32_t radius = p->radius;
    const lv_area_t rect = p->rect;
    const bool outer = p->outer;

    if(radius == 0) {
        int32_t first = rect.x1 - abs_x;
        int32_t last = rect.x2 - abs_x + 1;
        if(first >= len || last <= 0) {
            return outer ? LV_DRAW_SW_MASK_RES_TRANSP : LV_DRAW_SW_MASK_RES_FULL_COVER;
        }

        first = lv_clamp(0, first, len);
        last = lv_clamp(0, last, len);

        if(!outer) {
            if(first > 0) lv_memzero(mask_buf, first);
            if(last < len) lv_memzero(&mask_buf[last], len - last);
            return LV_DRAW_SW_MASK_RES_CHANGED;
        }
        else {
            if(first < len) {
                int32_t span = last - first;
                if(span > 0) lv_memzero(&mask_buf[first], span);
            }
            return LV_DRAW_SW_MASK_RES_CHANGED;
        }
    }

    lv_draw_sw_mask_radius_circle_dsc_t *circle = p->circle;
    if(!circle) return LV_DRAW_SW_MASK_RES_CHANGED;

    int32_t k = rect.x1 - abs_x;
    int32_t w = lv_area_get_width(&rect);
    int32_t h = lv_area_get_height(&rect);
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
    lv_opa_t *aa_opa = get_next_line(circle, cir_y, &aa_len, &x_start);
    int32_t cir_x_right = k + w - radius + x_start;
    int32_t cir_x_left = k + radius - x_start - 1;

    if(!outer) {
        for(int32_t i = 0; i < aa_len; i++) {
            lv_opa_t opa = aa_opa[aa_len - i - 1];
            int32_t right_idx = cir_x_right + i;
            if(right_idx >= 0 && right_idx < len) mask_buf[right_idx] = mask_mix(mask_buf[right_idx], opa);
            int32_t left_idx = cir_x_left - i;
            if(left_idx >= 0 && left_idx < len) mask_buf[left_idx] = mask_mix(mask_buf[left_idx], opa);
        }

        cir_x_right = lv_clamp(0, cir_x_right + aa_len, len);
        if(cir_x_right < len) lv_memzero(&mask_buf[cir_x_right], len - cir_x_right);

        cir_x_left = lv_clamp(0, cir_x_left - aa_len + 1, len);
        if(cir_x_left > 0) lv_memzero(mask_buf, cir_x_left);
    }
    else {
        for(int32_t i = 0; i < aa_len; i++) {
            lv_opa_t opa = (lv_opa_t)(255 - aa_opa[aa_len - 1 - i]);
            int32_t right_idx = cir_x_right + i;
            if(right_idx >= 0 && right_idx < len) mask_buf[right_idx] = mask_mix(mask_buf[right_idx], opa);
            int32_t left_idx = cir_x_left - i;
            if(left_idx >= 0 && left_idx < len) mask_buf[left_idx] = mask_mix(mask_buf[left_idx], opa);
        }

        int32_t clr_start = lv_clamp(0, cir_x_left + 1, len);
        int32_t clr_end = lv_clamp(clr_start, cir_x_right, len);
        if(clr_end > clr_start) lv_memzero(&mask_buf[clr_start], (size_t)(clr_end - clr_start));
    }

    return LV_DRAW_SW_MASK_RES_CHANGED;
}

static void mask_radius_init(lv_draw_sw_mask_radius_param_t *param, const lv_area_t *rect, int32_t radius, bool outer) {
    param->rect = *rect;
    param->radius = radius;
    param->outer = outer;
    if(radius > 0) {
        param->circle = calloc(1, sizeof(*param->circle));
        circ_calc_aa4(param->circle, radius);
    }
    else {
        param->circle = NULL;
    }
}

static void mask_radius_free(lv_draw_sw_mask_radius_param_t *param) {
    if(param->circle) {
        free(param->circle->buf);
        free(param->circle);
    }
}

static void apply_masks(lv_draw_sw_mask_radius_param_t **masks, uint8_t *buf, int32_t abs_x, int32_t abs_y, int32_t len) {
    for(int idx = 0; masks[idx] != NULL; idx++) {
        lv_draw_sw_mask_res_t res = lv_draw_mask_radius(buf, abs_x, abs_y, len, masks[idx]);
        if(res == LV_DRAW_SW_MASK_RES_TRANSP) {
            memset(buf, 0, len);
            return;
        }
    }
}

int main(void) {
    lv_area_t outer = {20, 30, 82, 95};
    lv_area_t inner = {21, 31, 81, 94};

    lv_draw_sw_mask_radius_param_t inner_param = {0};
    lv_draw_sw_mask_radius_param_t outer_param = {0};
    mask_radius_init(&inner_param, &inner, 9, true);
    mask_radius_init(&outer_param, &outer, 10, false);

    int len = outer.x2 - outer.x1 + 1;
    uint8_t buf[128];

    lv_draw_sw_mask_radius_param_t *masks[] = {&inner_param, &outer_param, NULL};

    memset(buf, 255, (size_t)len);
        lv_draw_mask_radius(buf, outer.x1, 30, len, &inner_param);
        printf("mask y=30 after inner:\n");
        for(int i = 0; i < 16 && i < len; i++) printf("%d ", buf[i]);
        printf("\n");

        if(outer_param.circle) {
         int32_t len_i32 = len;
            int32_t w = lv_area_get_width(&outer_param.rect);
            int32_t radius = outer_param.radius;
            int32_t rel_y = 30 - outer_param.rect.y1;
            int32_t h = lv_area_get_height(&outer_param.rect);
         int32_t cir_y = rel_y < radius ? radius - rel_y - 1 : rel_y - (h - radius);
         int32_t aa_len = 0;
         int32_t x_start = 0;
            lv_opa_t *aa_line = get_next_line(outer_param.circle, cir_y, &aa_len, &x_start);
         (void)aa_line;
            int32_t k = outer_param.rect.x1 - outer.x1;
            int32_t cir_x_right = k + w - radius + x_start;
            int32_t cir_x_left = k + radius - x_start - 1;
         printf("debug outer y=30: k=%d w=%d radius=%d aa_len=%d cir_x_left=%d cir_x_right=%d\n",
             k, w, radius, aa_len, cir_x_left, cir_x_right);
         printf("right_clear=%d left_clear=%d\n",
             (cir_x_right + aa_len) < 0 ? 0 : (cir_x_right + aa_len > len_i32 ? len_i32 : cir_x_right + aa_len),
             (cir_x_left - aa_len + 1) < 0 ? 0 : (cir_x_left - aa_len + 1 > len_i32 ? len_i32 : cir_x_left - aa_len + 1));
        }

        lv_draw_mask_radius(buf, outer.x1, 30, len, &outer_param);
        printf("mask y=30:\n");
        for(int i = 0; i < 16 && i < len; i++) printf("%d ", buf[i]);
        printf("\n");

        memset(buf, 255, (size_t)len);
        lv_draw_mask_radius(buf, outer.x1, 31, len, &inner_param);
        printf("mask y=31 after inner:\n");
        for(int i = 0; i < 16 && i < len; i++) printf("%d ", buf[i]);
        printf("\n");

        lv_draw_mask_radius(buf, outer.x1, 31, len, &outer_param);
    printf("mask y=31:\n");
    for(int i = 0; i < 16 && i < len; i++) printf("%d ", buf[i]);
    printf("\n");

    mask_radius_free(&inner_param);
    mask_radius_free(&outer_param);
    return 0;
}
