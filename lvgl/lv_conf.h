#ifndef LV_CONF_H
#define LV_CONF_H

/* Minimal LVGL configuration for sprite generator builds */
#define LV_CONF_H

#define LV_DISABLE_API_MAPPING 1

#define LV_USE_SPAN 0

#define LV_USE_THEME_DEFAULT 0
#define LV_USE_THEME_SIMPLE 0
#define LV_USE_THEME_MONO 0
#define LV_USE_THEME_BASIC 0

/* Leave vector graphics disabled by default; enable externally when backend is available */
/* #define LV_USE_VECTOR_GRAPHIC 1 */

#endif /* LV_CONF_H */
