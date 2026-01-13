#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

#define LV_MIN(a, b) ((a) < (b) ? (a) : (b))
#define LV_MAX(a, b) ((a) > (b) ? (a) : (b))
#define LV_ABS(a) ((a) < 0 ? -(a) : (a))

typedef uint8_t lv_opa_t;
typedef struct { int32_t x1, y1, x2, y2; } lv_area_t;
typedef struct { int32_t x, y; } lv_point_t;

static void lv_point_set(lv_point_t * p, int32_t x, int32_t y)
{
    p->x = x;
    p->y = y;
}

static const uint16_t sin0_90_table[] = {
    0,     572,   1144,  1715,  2286,  2856,  3425,  3993,  4560,  5126,  5690,  6252,  6813,  7371,  7927,  8481,
    9032,  9580,  10126, 10668, 11207, 11743, 12275, 12803, 13328, 13848, 14365, 14876, 15384, 15886, 16384, 16877,
    17364, 17847, 18324, 18795, 19261, 19720, 20174, 20622, 21063, 21498, 21926, 22348, 22763, 23170, 23571, 23965,
    24351, 24730, 25102, 25466, 25822, 26170, 26510, 26842, 27166, 27482, 27789, 28088, 28378, 28660, 28932, 29197,
    29452, 29698, 29935, 30163, 30382, 30592, 30792, 30983, 31164, 31336, 31499, 31651, 31795, 31928, 32052, 32166,
    32270, 32365, 32449, 32524, 32588, 32643, 32688, 32723, 32748, 32763, 32768
};

static int32_t lv_trigo_sin(int16_t angle)
{
    int32_t ret = 0;
    while(angle < 0) angle += 360;
    while(angle >= 360) angle -= 360;

    if(angle < 90) {
        ret = sin0_90_table[angle];
    }
    else if(angle >= 90 && angle < 180) {
        angle = 180 - angle;
        ret = sin0_90_table[angle];
    }
    else if(angle >= 180 && angle < 270) {
        angle = angle - 180;
        ret = -((int32_t)sin0_90_table[angle]);
    }
    else {
        angle = 360 - angle;
        ret = -((int32_t)sin0_90_table[angle]);
    }

    if(ret == 32767) return 32768;
    if(ret == -32767) return -32768;
    return ret;
}

static int32_t lv_trigo_cos(int16_t angle)
{
    return lv_trigo_sin(angle + 90);
}

static uint8_t mask_mix(uint8_t mask_act, uint8_t mask_new);

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

typedef struct {
    int32_t start_angle;
    int32_t end_angle;
    int32_t width;
    const char *label;
} arc_case_t;

typedef enum {
    LV_DRAW_SW_MASK_LINE_SIDE_LEFT = 0,
    LV_DRAW_SW_MASK_LINE_SIDE_RIGHT = 1,
    LV_DRAW_SW_MASK_LINE_SIDE_TOP = 2,
    LV_DRAW_SW_MASK_LINE_SIDE_BOTTOM = 3,
} lv_draw_sw_mask_line_side_t;

typedef struct {
    lv_point_t p1;
    lv_point_t p2;
    lv_draw_sw_mask_line_side_t side;
    lv_point_t origo;
    int32_t xy_steep;
    int32_t yx_steep;
    int32_t steep;
    int32_t spx;
    bool flat;
    bool inv;
} lv_draw_sw_mask_line_param_t;

typedef struct {
    int32_t start_angle;
    int32_t end_angle;
    int32_t delta_deg;
    lv_point_t vertex;
    lv_draw_sw_mask_line_param_t start_line;
    lv_draw_sw_mask_line_param_t end_line;
} lv_draw_sw_mask_angle_param_t;

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

static lv_draw_sw_mask_res_t line_mask_flat(uint8_t * mask_buf, int32_t abs_x, int32_t abs_y,
                                            int32_t len, lv_draw_sw_mask_line_param_t * p)
{
    int32_t y_at_x = (int32_t)((int32_t)p->yx_steep * abs_x) >> 10;

    if(p->yx_steep > 0) {
        if(y_at_x > abs_y) {
            return p->inv ? LV_DRAW_SW_MASK_RES_FULL_COVER : LV_DRAW_SW_MASK_RES_TRANSP;
        }
    }
    else {
        if(y_at_x < abs_y) {
            return p->inv ? LV_DRAW_SW_MASK_RES_FULL_COVER : LV_DRAW_SW_MASK_RES_TRANSP;
        }
    }

    y_at_x = (int32_t)((int32_t)p->yx_steep * (abs_x + len)) >> 10;
    if(p->yx_steep > 0) {
        if(y_at_x < abs_y) {
            return p->inv ? LV_DRAW_SW_MASK_RES_TRANSP : LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
    }
    else {
        if(y_at_x > abs_y) {
            return p->inv ? LV_DRAW_SW_MASK_RES_TRANSP : LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
    }

    int32_t xe = (p->yx_steep > 0 ? (abs_y * 256) : ((abs_y + 1) * 256)) * p->xy_steep;
    xe >>= 10;
    int32_t xei = xe >> 8;
    int32_t xef = xe & 0xFF;

    int32_t px_h = xef == 0 ? 255 : 255 - (((255 - xef) * p->spx) >> 8);
    int32_t k = xei - abs_x;
    uint8_t m;

    if(xef) {
        if(k >= 0 && k < len) {
            m = (uint8_t)(255 - (((255 - xef) * (255 - px_h)) >> 9));
            if(p->inv) m = 255 - m;
            mask_buf[k] = mask_mix(mask_buf[k], m);
        }
        k++;
    }

    while(px_h > p->spx) {
        if(k >= 0 && k < len) {
            m = (uint8_t)(px_h - (p->spx >> 1));
            if(p->inv) m = 255 - m;
            mask_buf[k] = mask_mix(mask_buf[k], m);
        }
        px_h -= p->spx;
        k++;
        if(k >= len) break;
    }

    if(k < len && k >= 0) {
        int32_t x_inters = (px_h * p->xy_steep) >> 10;
        m = (uint8_t)((x_inters * px_h) >> 9);
        if(p->yx_steep < 0) m = 255 - m;
        if(p->inv) m = 255 - m;
        mask_buf[k] = mask_mix(mask_buf[k], m);
    }

    if(p->inv) {
        k = xei - abs_x;
        if(k > len) return LV_DRAW_SW_MASK_RES_TRANSP;
        if(k >= 0) lv_memzero(mask_buf, k);
    }
    else {
        k++;
        if(k < 0) return LV_DRAW_SW_MASK_RES_TRANSP;
        if(k <= len) lv_memzero(&mask_buf[k], len - k);
    }

    return LV_DRAW_SW_MASK_RES_CHANGED;
}

static lv_draw_sw_mask_res_t line_mask_steep(uint8_t * mask_buf, int32_t abs_x, int32_t abs_y,
                                             int32_t len, lv_draw_sw_mask_line_param_t * p)
{
    int32_t x_at_y = (int32_t)((int32_t)p->xy_steep * abs_y) >> 10;
    if(p->xy_steep > 0) x_at_y++;
    if(x_at_y < abs_x) {
        return p->inv ? LV_DRAW_SW_MASK_RES_FULL_COVER : LV_DRAW_SW_MASK_RES_TRANSP;
    }

    x_at_y = (int32_t)((int32_t)p->xy_steep * abs_y) >> 10;
    if(x_at_y > abs_x + len) {
        return p->inv ? LV_DRAW_SW_MASK_RES_TRANSP : LV_DRAW_SW_MASK_RES_FULL_COVER;
    }

    int32_t xs = ((abs_y * 256) * p->xy_steep) >> 10;
    int32_t xsi = xs >> 8;
    int32_t xsf = xs & 0xFF;

    int32_t xe = (((abs_y + 1) * 256) * p->xy_steep) >> 10;
    int32_t xei = xe >> 8;
    int32_t xef = xe & 0xFF;

    int32_t k = xsi - abs_x;
    uint8_t m;

    if(xsi == xei) {
        if(k >= 0 && k < len) {
            m = (uint8_t)((xsf + xef) >> 1);
            if(p->inv) m = 255 - m;
            mask_buf[k] = mask_mix(mask_buf[k], m);
        }
        k++;

        if(p->inv) {
            k = xsi - abs_x;
            if(k >= len) return LV_DRAW_SW_MASK_RES_TRANSP;
            if(k >= 0) lv_memzero(mask_buf, k);
        }
        else {
            if(k > len) k = len;
            if(k == 0) return LV_DRAW_SW_MASK_RES_TRANSP;
            if(k > 0) lv_memzero(&mask_buf[k], len - k);
        }
    }
    else {
        if(p->xy_steep < 0) {
            int32_t y_inters = (xsf * (-p->yx_steep)) >> 10;
            if(k >= 0 && k < len) {
                m = (uint8_t)((y_inters * xsf) >> 9);
                if(p->inv) m = 255 - m;
                mask_buf[k] = mask_mix(mask_buf[k], m);
            }
            k--;

            int32_t x_inters = ((255 - y_inters) * (-p->xy_steep)) >> 10;
            if(k >= 0 && k < len) {
                m = (uint8_t)(255 - (((255 - y_inters) * x_inters) >> 9));
                if(p->inv) m = 255 - m;
                mask_buf[k] = mask_mix(mask_buf[k], m);
            }
            k += 2;

            if(p->inv) {
                k = xsi - abs_x - 1;
                if(k > len) k = len;
                else if(k > 0) lv_memzero(mask_buf, k);
            }
            else {
                if(k > len) return LV_DRAW_SW_MASK_RES_FULL_COVER;
                if(k >= 0) lv_memzero(&mask_buf[k], len - k);
            }
        }
        else {
            int32_t y_inters = ((255 - xsf) * p->yx_steep) >> 10;
            if(k >= 0 && k < len) {
                m = (uint8_t)(255 - ((y_inters * (255 - xsf)) >> 9));
                if(p->inv) m = 255 - m;
                mask_buf[k] = mask_mix(mask_buf[k], m);
            }
            k++;

            int32_t x_inters = ((255 - y_inters) * p->xy_steep) >> 10;
            if(k >= 0 && k < len) {
                m = (uint8_t)(((255 - y_inters) * x_inters) >> 9);
                if(p->inv) m = 255 - m;
                mask_buf[k] = mask_mix(mask_buf[k], m);
            }
            k++;

            if(p->inv) {
                k = xsi - abs_x;
                if(k > len) return LV_DRAW_SW_MASK_RES_TRANSP;
                if(k >= 0) lv_memzero(mask_buf, k);
            }
            else {
                if(k > len) k = len;
                if(k == 0) return LV_DRAW_SW_MASK_RES_TRANSP;
                if(k > 0) lv_memzero(&mask_buf[k], len - k);
            }
        }
    }

    return LV_DRAW_SW_MASK_RES_CHANGED;
}

static lv_draw_sw_mask_res_t lv_draw_mask_line(uint8_t * mask_buf, int32_t abs_x, int32_t abs_y,
                                               int32_t len, lv_draw_sw_mask_line_param_t * p)
{
    abs_y -= p->origo.y;
    abs_x -= p->origo.x;

    if(p->steep == 0) {
        if(p->flat) {
            if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_LEFT ||
               p->side == LV_DRAW_SW_MASK_LINE_SIDE_RIGHT) {
                return LV_DRAW_SW_MASK_RES_FULL_COVER;
            }
            else if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_TOP && abs_y < 0) {
                return LV_DRAW_SW_MASK_RES_FULL_COVER;
            }
            else if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_BOTTOM && abs_y > 0) {
                return LV_DRAW_SW_MASK_RES_FULL_COVER;
            }
            else {
                return LV_DRAW_SW_MASK_RES_TRANSP;
            }
        }
        else {
            if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_TOP ||
               p->side == LV_DRAW_SW_MASK_LINE_SIDE_BOTTOM) {
                return LV_DRAW_SW_MASK_RES_FULL_COVER;
            }
            else if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_RIGHT && abs_x > 0) {
                return LV_DRAW_SW_MASK_RES_FULL_COVER;
            }
            else if(p->side == LV_DRAW_SW_MASK_LINE_SIDE_LEFT) {
                if(abs_x + len < 0) return LV_DRAW_SW_MASK_RES_FULL_COVER;
                int32_t k = -abs_x;
                if(k < 0) return LV_DRAW_SW_MASK_RES_TRANSP;
                if(k >= 0 && k < len) lv_memzero(&mask_buf[k], len - k);
                return LV_DRAW_SW_MASK_RES_CHANGED;
            }
            else {
                if(abs_x + len < 0) return LV_DRAW_SW_MASK_RES_TRANSP;
                int32_t k = -abs_x;
                if(k < 0) k = 0;
                if(k >= len) return LV_DRAW_SW_MASK_RES_TRANSP;
                if(k >= 0 && k < len) lv_memzero(mask_buf, k);
                return LV_DRAW_SW_MASK_RES_CHANGED;
            }
        }
    }

    if(p->flat) {
        return line_mask_flat(mask_buf, abs_x, abs_y, len, p);
    }
    else {
        return line_mask_steep(mask_buf, abs_x, abs_y, len, p);
    }
}

static void lv_draw_sw_mask_line_points_init(lv_draw_sw_mask_line_param_t * param, int32_t p1x, int32_t p1y,
                                             int32_t p2x, int32_t p2y, lv_draw_sw_mask_line_side_t side)
{
    lv_memzero(param, sizeof(*param));

    if(p1y == p2y && side == LV_DRAW_SW_MASK_LINE_SIDE_BOTTOM) {
        p1y--;
        p2y--;
    }

    if(p1y > p2y) {
        int32_t t = p2x;
        p2x = p1x;
        p1x = t;

        t = p2y;
        p2y = p1y;
        p1y = t;
    }

    lv_point_set(&param->p1, p1x, p1y);
    lv_point_set(&param->p2, p2x, p2y);
    param->side = side;
    lv_point_set(&param->origo, p1x, p1y);
    param->flat = LV_ABS(p2x - p1x) > LV_ABS(p2y - p1y);
    param->yx_steep = 0;
    param->xy_steep = 0;

    int32_t dx = p2x - p1x;
    int32_t dy = p2y - p1y;

    if(param->flat) {
        if(dx) {
            int32_t m = (int32_t)((1L << 20) / dx);
            param->yx_steep = (m * dy) >> 10;
        }
        if(dy) {
            int32_t m = (int32_t)((1L << 20) / dy);
            param->xy_steep = (m * dx) >> 10;
        }
        param->steep = param->yx_steep;
    }
    else {
        if(dy) {
            int32_t m = (int32_t)((1L << 20) / dy);
            param->xy_steep = (m * dx) >> 10;
        }
        if(dx) {
            int32_t m = (int32_t)((1L << 20) / dx);
            param->yx_steep = (m * dy) >> 10;
        }
        param->steep = param->xy_steep;
    }

    if(param->side == LV_DRAW_SW_MASK_LINE_SIDE_LEFT) param->inv = false;
    else if(param->side == LV_DRAW_SW_MASK_LINE_SIDE_RIGHT) param->inv = true;
    else if(param->side == LV_DRAW_SW_MASK_LINE_SIDE_TOP) param->inv = param->steep > 0;
    else if(param->side == LV_DRAW_SW_MASK_LINE_SIDE_BOTTOM) param->inv = !(param->steep > 0);

    param->spx = param->steep >> 2;
    if(param->steep < 0) param->spx = -param->spx;
}

static void lv_draw_sw_mask_line_angle_init(lv_draw_sw_mask_line_param_t * param, int32_t p1x, int32_t p1y,
                                            int16_t angle, lv_draw_sw_mask_line_side_t side)
{
    if(angle > 180) angle -= 180;

    int32_t p2x = (lv_trigo_sin(angle + 90) >> 5) + p1x;
    int32_t p2y = (lv_trigo_sin(angle) >> 5) + p1y;

    lv_draw_sw_mask_line_points_init(param, p1x, p1y, p2x, p2y, side);
}

static void lv_draw_sw_mask_angle_init(lv_draw_sw_mask_angle_param_t * param, int32_t vertex_x, int32_t vertex_y,
                                       int32_t start_angle, int32_t end_angle)
{
    if(start_angle < 0) start_angle = 0;
    else if(start_angle > 359) start_angle = 359;

    if(end_angle < 0) end_angle = 0;
    else if(end_angle > 359) end_angle = 359;

    if(end_angle < start_angle) param->delta_deg = 360 - start_angle + end_angle;
    else param->delta_deg = LV_ABS(end_angle - start_angle);

    param->start_angle = start_angle;
    param->end_angle = end_angle;
    lv_point_set(&param->vertex, vertex_x, vertex_y);

    lv_draw_sw_mask_line_side_t start_side =
        (start_angle >= 0 && start_angle < 180) ? LV_DRAW_SW_MASK_LINE_SIDE_LEFT : LV_DRAW_SW_MASK_LINE_SIDE_RIGHT;

    lv_draw_sw_mask_line_side_t end_side;
    if(end_angle >= 0 && end_angle < 180) end_side = LV_DRAW_SW_MASK_LINE_SIDE_RIGHT;
    else end_side = LV_DRAW_SW_MASK_LINE_SIDE_LEFT;

    lv_draw_sw_mask_line_angle_init(&param->start_line, vertex_x, vertex_y, start_angle, start_side);
    lv_draw_sw_mask_line_angle_init(&param->end_line, vertex_x, vertex_y, end_angle, end_side);
}

static lv_draw_sw_mask_res_t lv_draw_mask_angle(uint8_t * mask_buf, int32_t abs_x, int32_t abs_y, int32_t len,
                                                lv_draw_sw_mask_angle_param_t * p)
{
    int32_t rel_y = abs_y - p->vertex.y;
    int32_t rel_x = abs_x - p->vertex.x;

    if(p->start_angle < 180 && p->end_angle < 180 && p->start_angle != 0 && p->end_angle != 0 &&
       p->start_angle > p->end_angle) {

        if(abs_y < p->vertex.y) return LV_DRAW_SW_MASK_RES_FULL_COVER;

        int32_t end_angle_first = (rel_y * p->end_line.xy_steep) >> 10;
        int32_t start_angle_last = ((rel_y + 1) * p->start_line.xy_steep) >> 10;

        if(p->start_angle > 270 && p->start_angle <= 359 && start_angle_last < 0) start_angle_last = 0;
        else if(p->start_angle > 0 && p->start_angle <= 90 && start_angle_last < 0) start_angle_last = 0;
        else if(p->start_angle > 90 && p->start_angle < 270 && start_angle_last > 0) start_angle_last = 0;

        if(p->end_angle > 270 && p->end_angle <= 359 && start_angle_last < 0) start_angle_last = 0;
        else if(p->end_angle > 0 && p->end_angle <= 90 && start_angle_last < 0) start_angle_last = 0;
        else if(p->end_angle > 90 && p->end_angle < 270 && start_angle_last > 0) start_angle_last = 0;

        int32_t dist = (end_angle_first - start_angle_last) >> 1;

        lv_draw_sw_mask_res_t res1 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        lv_draw_sw_mask_res_t res2 = LV_DRAW_SW_MASK_RES_FULL_COVER;

        int32_t tmp = start_angle_last + dist - rel_x;
        if(tmp > len) tmp = len;
        if(tmp > 0) {
            res1 = lv_draw_mask_line(&mask_buf[0], abs_x, abs_y, tmp, &p->start_line);
            if(res1 == LV_DRAW_SW_MASK_RES_TRANSP) lv_memzero(&mask_buf[0], tmp);
        }

        if(tmp > len) tmp = len;
        if(tmp < 0) tmp = 0;
        res2 = lv_draw_mask_line(&mask_buf[tmp], abs_x + tmp, abs_y, len - tmp, &p->end_line);
        if(res2 == LV_DRAW_SW_MASK_RES_TRANSP) lv_memzero(&mask_buf[tmp], len - tmp);

        return (res1 == res2) ? res1 : LV_DRAW_SW_MASK_RES_CHANGED;
    }
    else if(p->start_angle > 180 && p->end_angle > 180 && p->start_angle > p->end_angle) {

        if(abs_y > p->vertex.y) return LV_DRAW_SW_MASK_RES_FULL_COVER;

        int32_t end_angle_first = (rel_y * p->end_line.xy_steep) >> 10;
        int32_t start_angle_last = ((rel_y + 1) * p->start_line.xy_steep) >> 10;

        if(p->start_angle > 270 && p->start_angle <= 359 && start_angle_last < 0) start_angle_last = 0;
        else if(p->start_angle > 0 && p->start_angle <= 90 && start_angle_last < 0) start_angle_last = 0;
        else if(p->start_angle > 90 && p->start_angle < 270 && start_angle_last > 0) start_angle_last = 0;

        if(p->end_angle > 270 && p->end_angle <= 359 && start_angle_last < 0) start_angle_last = 0;
        else if(p->end_angle > 0 && p->end_angle <= 90 && start_angle_last < 0) start_angle_last = 0;
        else if(p->end_angle > 90 && p->end_angle < 270 && start_angle_last > 0) start_angle_last = 0;

        int32_t dist = (end_angle_first - start_angle_last) >> 1;

        lv_draw_sw_mask_res_t res1 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        lv_draw_sw_mask_res_t res2 = LV_DRAW_SW_MASK_RES_FULL_COVER;

        int32_t tmp = start_angle_last + dist - rel_x;
        if(tmp > len) tmp = len;
        if(tmp > 0) {
            res1 = lv_draw_mask_line(&mask_buf[0], abs_x, abs_y, tmp, &p->end_line);
            if(res1 == LV_DRAW_SW_MASK_RES_TRANSP) lv_memzero(&mask_buf[0], tmp);
        }

        if(tmp > len) tmp = len;
        if(tmp < 0) tmp = 0;
        res2 = lv_draw_mask_line(&mask_buf[tmp], abs_x + tmp, abs_y, len - tmp, &p->start_line);
        if(res2 == LV_DRAW_SW_MASK_RES_TRANSP) lv_memzero(&mask_buf[tmp], len - tmp);

        return (res1 == res2) ? res1 : LV_DRAW_SW_MASK_RES_CHANGED;
    }
    else {
        lv_draw_sw_mask_res_t res1 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        lv_draw_sw_mask_res_t res2 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        bool res1_unknown = false;
        bool res2_unknown = false;

        if(p->start_angle == 180) {
            if(abs_y < p->vertex.y) res1 = LV_DRAW_SW_MASK_RES_FULL_COVER;
            else res1_unknown = true;
        }
        else if(p->start_angle == 0) {
            if(abs_y < p->vertex.y) res1_unknown = true;
            else res1 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
        else if((p->start_angle < 180 && abs_y < p->vertex.y) ||
                (p->start_angle > 180 && abs_y >= p->vertex.y)) {
            res1_unknown = true;
        }
        else {
            res1 = lv_draw_mask_line(mask_buf, abs_x, abs_y, len, &p->start_line);
        }

        if(p->end_angle == 180) {
            if(abs_y < p->vertex.y) res2_unknown = true;
            else res2 = LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
        else if(p->end_angle == 0) {
            if(abs_y < p->vertex.y) res2 = LV_DRAW_SW_MASK_RES_FULL_COVER;
            else res2_unknown = true;
        }
        else if((p->end_angle < 180 && abs_y < p->vertex.y) ||
                (p->end_angle > 180 && abs_y >= p->vertex.y)) {
            res2_unknown = true;
        }
        else {
            res2 = lv_draw_mask_line(mask_buf, abs_x, abs_y, len, &p->end_line);
        }

        if(res1 == LV_DRAW_SW_MASK_RES_TRANSP || res2 == LV_DRAW_SW_MASK_RES_TRANSP) return LV_DRAW_SW_MASK_RES_TRANSP;
        if(res1_unknown && res2_unknown) return LV_DRAW_SW_MASK_RES_TRANSP;
        if(!res1_unknown && !res2_unknown &&
           res1 == LV_DRAW_SW_MASK_RES_FULL_COVER && res2 == LV_DRAW_SW_MASK_RES_FULL_COVER) {
            return LV_DRAW_SW_MASK_RES_FULL_COVER;
        }
        return LV_DRAW_SW_MASK_RES_CHANGED;
    }
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

static const arc_case_t ARC_CASES[] = {
    {0, 90, 3, "arc_quarter_w3_a0"},
    {90, 180, 3, "arc_quarter_w3_a90"},
    {180, 270, 3, "arc_quarter_w3_a180"},
    {270, 360, 3, "arc_quarter_w3_a270"},
    {0, 90, 8, "arc_quarter_w8_a0"},
    {90, 180, 8, "arc_quarter_w8_a90"},
    {180, 270, 8, "arc_quarter_w8_a180"},
    {270, 360, 8, "arc_quarter_w8_a270"},
    {0, 90, 15, "arc_quarter_w15_a0"},
    {90, 180, 15, "arc_quarter_w15_a90"},
    {180, 270, 15, "arc_quarter_w15_a180"},
    {270, 360, 15, "arc_quarter_w15_a270"},
    {0, 45, 8, "arc_span45"},
    {0, 90, 8, "arc_span90"},
    {0, 180, 8, "arc_span180"},
    {0, 270, 8, "arc_span270"},
};

static const int ARC_CASE_COUNT = (int)(sizeof(ARC_CASES) / sizeof(ARC_CASES[0]));

static const char * mask_res_to_str(lv_draw_sw_mask_res_t res)
{
    switch(res) {
        case LV_DRAW_SW_MASK_RES_TRANSP: return "Transparent";
        case LV_DRAW_SW_MASK_RES_FULL_COVER: return "FullCover";
        case LV_DRAW_SW_MASK_RES_CHANGED: return "Changed";
        default: return "Unknown";
    }
}

static void print_compact_ranges(const char * label, const char * case_label, int y,
                                 lv_draw_sw_mask_res_t res, const uint8_t * buf,
                                 int width, int32_t x0)
{
    printf("%s case=%s y=%d res=%s ranges=[", label, case_label, y, mask_res_to_str(res));
    bool first = true;
    for(int idx = 0; idx < width;) {
        int val = buf[idx];
        int end = idx;
        while(end + 1 < width && buf[end + 1] == val) {
            end++;
        }
        if(!first) printf(", ");
        printf("(%d, %d, %d)", x0 + idx, x0 + end, val);
        first = false;
        idx = end + 1;
    }
    printf("]\n");
}

static void print_circle_debug(const char * label, const char * case_label, int rel_y,
                               const lv_draw_sw_mask_radius_param_t * mask)
{
    if(rel_y < 0 || !mask->circle || mask->radius <= 0) {
        return;
    }

    int32_t radius = mask->radius;
    int32_t h = lv_area_get_height(&mask->rect);
    int32_t cir_y;
    if(rel_y < radius) {
        cir_y = radius - rel_y - 1;
    }
    else {
        cir_y = rel_y - (h - radius);
    }

    int32_t len = 0;
    int32_t x_start = 0;
    if(cir_y < 0 || cir_y >= radius) {
        return;
    }

    lv_opa_t * line = get_next_line(mask->circle, cir_y, &len, &x_start);
    if(!line || len <= 0) {
        return;
    }

    printf("%s case=%s rel_y=%d x_start=%d opa=[", label, case_label, rel_y, x_start);
    for(int i = 0; i < len; i++) {
        if(i > 0) printf(", ");
        printf("%d", line[i]);
    }
    printf("]\n");
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
    // Precompute outer mask common to all cases
    lv_draw_sw_mask_radius_param_t outer_mask = {0};
    outer_mask.rect = (lv_area_t){16, 27, 86, 97};
    outer_mask.radius = 35;
    outer_mask.outer = false;
    outer_mask.circle = malloc(sizeof(lv_draw_sw_mask_radius_circle_dsc_t));
    memset(outer_mask.circle, 0, sizeof(*outer_mask.circle));
    circ_calc_aa4(outer_mask.circle, outer_mask.radius);

    int32_t center_x = (outer_mask.rect.x1 + outer_mask.rect.x2) / 2;
    int32_t center_y = (outer_mask.rect.y1 + outer_mask.rect.y2) / 2;
    int width = outer_mask.rect.x2 - outer_mask.rect.x1 + 1;
    int32_t x0 = outer_mask.rect.x1;

    for(int case_idx = 0; case_idx < ARC_CASE_COUNT; case_idx++) {
        const arc_case_t *c = &ARC_CASES[case_idx];
        if(case_idx > 0) {
            printf("\n");
        }
        printf("case=%s start=%d end=%d width=%d\n", c->label, c->start_angle, c->end_angle, c->width);

        lv_draw_sw_mask_angle_param_t angle_mask;
        lv_draw_sw_mask_angle_init(&angle_mask, center_x, center_y, c->start_angle, c->end_angle);

        int32_t inner_radius = outer_mask.radius - c->width;
        if(inner_radius < 0) inner_radius = 0;
        lv_area_t inner_rect = {
            center_x - inner_radius,
            center_y - inner_radius,
            center_x + inner_radius,
            center_y + inner_radius,
        };

        lv_draw_sw_mask_radius_param_t inner_mask = {0};
        inner_mask.rect = inner_rect;
        inner_mask.radius = inner_radius;
        inner_mask.outer = true;
        if(inner_radius > 0) {
            inner_mask.circle = malloc(sizeof(lv_draw_sw_mask_radius_circle_dsc_t));
            memset(inner_mask.circle, 0, sizeof(*inner_mask.circle));
            circ_calc_aa4(inner_mask.circle, inner_radius);
        }

        for(int y = outer_mask.rect.y1; y <= outer_mask.rect.y2; y++) {
            uint8_t buf_combined[128];
            memset(buf_combined, 255, sizeof(buf_combined));
            lv_draw_sw_mask_res_t res_combined = LV_DRAW_SW_MASK_RES_FULL_COVER;
            lv_draw_sw_mask_res_t step_res = lv_draw_mask_angle(buf_combined, x0, y, width, &angle_mask);
            if(step_res == LV_DRAW_SW_MASK_RES_TRANSP) {
                memset(buf_combined, 0, width);
                res_combined = LV_DRAW_SW_MASK_RES_TRANSP;
            }
            else {
                if(step_res == LV_DRAW_SW_MASK_RES_CHANGED) {
                    res_combined = LV_DRAW_SW_MASK_RES_CHANGED;
                }
                step_res = lv_draw_mask_radius(buf_combined, x0, y, width, &outer_mask);
                if(step_res == LV_DRAW_SW_MASK_RES_TRANSP) {
                    memset(buf_combined, 0, width);
                    res_combined = LV_DRAW_SW_MASK_RES_TRANSP;
                }
                else {
                    if(step_res == LV_DRAW_SW_MASK_RES_CHANGED) {
                        res_combined = LV_DRAW_SW_MASK_RES_CHANGED;
                    }
                    if(inner_mask.circle) {
                        step_res = lv_draw_mask_radius(buf_combined, x0, y, width, &inner_mask);
                        if(step_res == LV_DRAW_SW_MASK_RES_TRANSP) {
                            memset(buf_combined, 0, width);
                            res_combined = LV_DRAW_SW_MASK_RES_TRANSP;
                        }
                        else if(step_res == LV_DRAW_SW_MASK_RES_CHANGED) {
                            res_combined = LV_DRAW_SW_MASK_RES_CHANGED;
                        }
                    }
                }
            }
            print_compact_ranges("combined", c->label, y, res_combined, buf_combined, width, x0);

            if(case_idx == 0 && y >= 60 && y <= 64) {
                uint8_t buf_angle[128];
                memset(buf_angle, 255, sizeof(buf_angle));
                lv_draw_sw_mask_res_t res_angle = lv_draw_mask_angle(buf_angle, x0, y, width, &angle_mask);
                if(res_angle == LV_DRAW_SW_MASK_RES_TRANSP) {
                    memset(buf_angle, 0, width);
                }
                print_compact_ranges("angle-only", c->label, y, res_angle, buf_angle, width, x0);

                uint8_t buf_outer_only[128];
                memset(buf_outer_only, 255, sizeof(buf_outer_only));
                lv_draw_sw_mask_res_t res_outer_only = lv_draw_mask_radius(buf_outer_only, x0, y, width, &outer_mask);
                if(res_outer_only == LV_DRAW_SW_MASK_RES_TRANSP) {
                    memset(buf_outer_only, 0, width);
                }
                print_compact_ranges("outer-only", c->label, y, res_outer_only, buf_outer_only, width, x0);

                uint8_t buf_inner_only[128];
                memset(buf_inner_only, 255, sizeof(buf_inner_only));
                lv_draw_sw_mask_res_t res_inner_only = LV_DRAW_SW_MASK_RES_FULL_COVER;
                if(inner_mask.circle) {
                    res_inner_only = lv_draw_mask_radius(buf_inner_only, x0, y, width, &inner_mask);
                    if(res_inner_only == LV_DRAW_SW_MASK_RES_TRANSP) {
                        memset(buf_inner_only, 0, width);
                    }
                }
                print_compact_ranges("inner-only", c->label, y, res_inner_only, buf_inner_only, width, x0);

                int rel_y_outer = y - outer_mask.rect.y1;
                print_circle_debug("outer", c->label, rel_y_outer, &outer_mask);
                if(inner_mask.circle) {
                    int rel_y_inner = y - inner_rect.y1;
                    print_circle_debug("inner", c->label, rel_y_inner, &inner_mask);
                }
            }
        }

        if(inner_mask.circle) {
            free(inner_mask.circle->buf);
            free(inner_mask.circle);
        }
    }

    free(outer_mask.circle->buf);
    free(outer_mask.circle);
    return 0;
}
