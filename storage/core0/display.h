#ifndef DISPLAY_H
#define DISPLAY_H

#ifdef __cplusplus
extern "C" {
#endif

void display_init();
void display_tick(const uint32_t tick_period);

#ifdef __cplusplus
}
#endif

#endif