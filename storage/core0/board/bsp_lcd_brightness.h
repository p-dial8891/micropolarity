#ifndef __BSP_LCD_BRIGHTNESS_H__
#define __BSP_LCD_BRIGHTNESS_H__

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pico/stdlib.h"

#define BSP_LCD_BL_PIN    16

#ifdef __cplusplus
extern "C" {
#endif

void bsp_lcd_brightness_init(void);
void bsp_lcd_brightness_set(uint8_t percent);

#ifdef __cplusplus
}
#endif

#endif //__BSP_LCD_BRIGHTNESS_H__

