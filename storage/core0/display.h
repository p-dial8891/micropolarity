#ifndef DISPLAY_H
#define DISPLAY_H

#include "lvgl.h"

#ifdef __cplusplus
extern "C" {
#endif

void display_init();
void display_tick(const uint32_t tick_period);
lv_obj_t* display_create_file_label();

#ifdef __cplusplus
}
#endif

#endif