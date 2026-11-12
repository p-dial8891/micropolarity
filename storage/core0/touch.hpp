#include <stdint.h>

#define TOUCH_PIN_IRQ   17

void touch_init();
uint16_t touch_get_x_pos();