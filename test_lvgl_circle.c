#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <math.h>

typedef uint8_t lv_opa_t;

typedef struct {
    int32_t x;
    int32_t y;
} lv_point_t;

typedef struct {
    int32_t radius;
    lv_opa_t * cir_opa;
    uint16_t * opa_start_on_y;
    uint16_t * x_start_on_y;
    uint8_t * buf;
} lv_draw_sw_mask_radius_circle_dsc_t;

static void circ_init(lv_point_t * c, int32_t * tmp, int32_t radius)
{
    c->x = radius;
    c->y = 0;
    *tmp = 1 - radius;
}

static bool circ_cont(lv_point_t * c)
{
    return c->y <= c->x;
}

static void circ_next(lv_point_t * c, int32_t * tmp)
{
    if(*tmp <= 0) {
        (*tmp) += 2 * c->y + 3;
    }
    else {
        (*tmp) += 2 * (c->y - c->x) + 5;
        c->x--;
    }
    c->y++;
}

static void circ_calc_aa4(lv_draw_sw_mask_radius_circle_dsc_t * c, int32_t radius)
{
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

int main(int argc, char **argv) {
    lv_draw_sw_mask_radius_circle_dsc_t circle = {0};
    int radius = 10;
    if(argc > 1) radius = atoi(argv[1]);
    circ_calc_aa4(&circle, radius);

    printf("Circle cache for radius=%d:\n", radius);
    for(int y = 0; y <= radius; y++) {
        int start = circle.opa_start_on_y[y];
        int end = circle.opa_start_on_y[y + 1];
        printf("y=%d: x_start=%d, opa=[", y, circle.x_start_on_y[y]);
        for(int i = start; i < end; i++) {
            printf("%d", circle.cir_opa[i]);
            if(i < end - 1) printf(", ");
        }
        printf("] (start=%d end=%d)\n", start, end);
    }

    int rect_h = radius * 2 + 1;
    printf("\nDerived cir_y values for abs_y 0..%d:\n", rect_h - 1);
    for(int abs_y = 0; abs_y < rect_h; abs_y++) {
        int cir_y;
        if(abs_y < radius) {
            cir_y = radius - abs_y - 1;
        } else {
            cir_y = abs_y - (rect_h - radius);
        }
        printf("abs_y=%2d -> cir_y=%2d\n", abs_y, cir_y);
    }

    free(circle.buf);
    return 0;
}
