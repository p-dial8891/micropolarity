#include <string.h>
#include <iostream>
#include "pico/stdlib.h"
#include "hardware/spi.h"
#include "runtime.hpp"
#include "board/spi.hpp"
#if 1
#include "lvgl/lvgl.h"
#include "lvgl/examples/lv_examples.h"
#include "hardware/dma.h"
// #include "lvgl/src/tick/lv_tick.h"
#include "ST7789_Defines.h"
#include "board/bsp_lcd_brightness.h"
#endif
#include "main.h"

#define SPI_PORT     spi1
#define PIN_MISO     12
#define PIN_MOSI     11
#define PIN_SCK      10
#define PIN_DC       14
#define PIN_RST      15
#define TFT_BL       16

#define SCREEN_WIDTH  240
#define SCREEN_HEIGHT 320

// Fast control macros
#define DC_CMD()   gpio_put(PIN_DC, 0)
#define DC_DATA()  gpio_put(PIN_DC, 1)
// #define CS_LOW()   gpio_put(PIN_CS, 0)
// #define CS_HIGH()  gpio_put(PIN_CS, 1)

#ifdef __cplusplus
extern "C" {
#endif

using namespace std;


void write_cmd(uint8_t cmd) {
    DC_CMD(); spi::display_cs(spi::LOW);
    spi_write_blocking(SPI_PORT, &cmd, 1);
    spi::display_cs(spi::HIGH);
}

void write_data(uint8_t data) {
    DC_DATA(); spi::display_cs(spi::LOW);
    spi_write_blocking(SPI_PORT, &data, 1);
    spi::display_cs(spi::HIGH);
}
#if 0
void ili9488_init() {
    gpio_set_function(PIN_MISO, GPIO_FUNC_SPI);
    gpio_set_function(PIN_MOSI, GPIO_FUNC_SPI);
    gpio_set_function(PIN_SCK, GPIO_FUNC_SPI);
    
    gpio_init(PIN_DC);  gpio_set_dir(PIN_DC, GPIO_OUT);
    gpio_init(PIN_RST); gpio_set_dir(PIN_RST, GPIO_OUT);

    // Hardware Reset
    gpio_put(PIN_RST, 0); sleep_ms(50);
    gpio_put(PIN_RST, 1); sleep_ms(120);

    write_cmd(0x11); // Sleep Out
    sleep_ms(120);

    write_cmd(0xB0);  // Interface control register
    write_data(0x80); // SDA_EN = 1 , SDO is not used

    write_cmd(0xE0); // Positive Gamma Control
    write_data(0x00);
    write_data(0x03);
    write_data(0x09);
    write_data(0x08);
    write_data(0x16);
    write_data(0x0A);
    write_data(0x3F);
    write_data(0x78);
    write_data(0x4C);
    write_data(0x09);
    write_data(0x0A);
    write_data(0x08);
    write_data(0x16);
    write_data(0x1A);
    write_data(0x0F);

    write_cmd(0XE1); // Negative Gamma Control
    write_data(0x00);
    write_data(0x16);
    write_data(0x19);
    write_data(0x03);
    write_data(0x0F);
    write_data(0x05);
    write_data(0x32);
    write_data(0x45);
    write_data(0x46);
    write_data(0x04);
    write_data(0x0E);
    write_data(0x0D);
    write_data(0x35);
    write_data(0x37);
    write_data(0x0F);

    write_cmd(0x36); // Memory Access Control
    write_data(0x00 | (1<<5) | (1<<6) | (1<<7));          // MV, MX, MY, RGB

    write_cmd(0x3A); write_data(0x55); // Interface Pixel Format: 16-bit/pixel

    write_cmd(0x29); // Display ON
}

void ili9488_set_window(uint16_t x1, uint16_t y1, uint16_t x2, uint16_t y2) {
    write_cmd(0x2A); // Column Addr
    write_data(x1 >> 8); write_data(x1 & 0xFF);
    write_data(x2 >> 8); write_data(x2 & 0xFF);

    write_cmd(0x2B); // Row Addr
    write_data(y1 >> 8); write_data(y1 & 0xFF);
    write_data(y2 >> 8); write_data(y2 & 0xFF);

    write_cmd(0x2C); // Memory Write
}
#endif

void st7789_init() {
    gpio_set_function(PIN_MISO, GPIO_FUNC_SPI);
    gpio_set_function(PIN_MOSI, GPIO_FUNC_SPI);
    gpio_set_function(PIN_SCK, GPIO_FUNC_SPI);
    gpio_init(TFT_BL);  gpio_set_dir(TFT_BL, GPIO_OUT);
    
    gpio_init(PIN_DC);  gpio_set_dir(PIN_DC, GPIO_OUT);
    gpio_init(PIN_RST); gpio_set_dir(PIN_RST, GPIO_OUT);

    // Hardware Reset
    gpio_put(PIN_RST, 0); sleep_ms(50);
    gpio_put(PIN_RST, 1); sleep_ms(120);
    gpio_put(TFT_BL, 1);

    write_cmd(ST7789_SLPOUT);   // Sleep out
    sleep_ms(120);

    write_cmd(ST7789_NORON);    // Normal display mode on

    //------------------------------display and color format setting--------------------------------//
    write_cmd(ST7789_MADCTL);
    //write_data(0x00);
    write_data(0x00);

    // // JLX240 display datasheet
    // write_cmd(0xB6);
    // write_data(0x0A);
    // write_data(0x82);

    write_cmd(ST7789_RAMCTRL);
    write_data(0x00);
    write_data(0xE0); // 5 to 6-bit conversion: r0 = r5, b0 = b5

    write_cmd(ST7789_COLMOD);
    write_data(0x05);
    sleep_ms(10);

    //--------------------------------ST7789V Frame rate setting----------------------------------//
    write_cmd(ST7789_PORCTRL);
    write_data(0x0c);
    write_data(0x0c);
    write_data(0x00);
    write_data(0x33);
    write_data(0x33);

    write_cmd(ST7789_GCTRL);    // VGH=14.97V,VGL=-7.67V
    write_data(0x75);

    //---------------------------------ST7789V Power setting--------------------------------------//
    write_cmd(ST7789_VCOMS);
    write_data(0x1A);

    write_cmd(ST7789_LCMCTRL);
    write_data(0x2C);

    write_cmd(ST7789_VDVVRHEN);
    write_data(0x01);
    write_data(0xFF);

    write_cmd(ST7789_VRHS);       // voltage VRHS
    write_data(0x13);

    write_cmd(ST7789_VDVSET);
    write_data(0x20);

    write_cmd(ST7789_FRCTR2);
    write_data(0x0f);

    write_cmd(ST7789_PWCTRL1);
    write_data(0xa4);
    write_data(0xa1);

    write_cmd(0xD6);
    write_data(0xA1);

    //--------------------------------ST7789V gamma setting---------------------------------------//
    write_cmd(ST7789_PVGAMCTRL);
    write_data(0xd0);
    write_data(0x0D);
    write_data(0x14);
    write_data(0x0D);
    write_data(0x0D);
    write_data(0x09);
    write_data(0x38);
    write_data(0x44);
    write_data(0x4E);
    write_data(0x3A);
    write_data(0x17);
    write_data(0x18);
    write_data(0x2F);
    write_data(0x30);

    write_cmd(ST7789_NVGAMCTRL);
    write_data(0xd0);
    write_data(0x09);
    write_data(0x0F);
    write_data(0x08);
    write_data(0x07);
    write_data(0x14);
    write_data(0x37);
    write_data(0x44);
    write_data(0x4D);
    write_data(0x38);
    write_data(0x15);
    write_data(0x16);
    write_data(0x2C);
    write_data(0x2E);

    write_cmd(ST7789_INVON);

    write_cmd(ST7789_CASET);    // Column address set
    write_data(0x00);
    write_data(0x00);
    write_data(0x00);
    write_data(0xEF);    // 239

    write_cmd(ST7789_RASET);    // Row address set
    write_data(0x00);
    write_data(0x00);
    write_data(0x01);
    write_data(0x3F);    // 319
    
    sleep_ms(120);
    write_cmd(ST7789_DISPON);
}

void st7789_set_window(uint16_t x1, uint16_t y1, uint16_t x2, uint16_t y2) {
    write_cmd(0x2A); // Column Addr
    write_data(x1 >> 8); write_data(x1 & 0xFF);
    write_data(x2 >> 8); write_data(x2 & 0xFF);

    write_cmd(0x2B); // Row Addr
    write_data(y1 >> 8); write_data(y1 & 0xFF);
    write_data(y2 >> 8); write_data(y2 & 0xFF);

    write_cmd(0x2C); // Memory Write
}

#define BYTES_PER_PIXEL (LV_COLOR_FORMAT_GET_SIZE(LV_COLOR_FORMAT_RGB565))
#define NUM_TX_ROW (2)

// Native LVGL Flush Callback with RGB565 to RGB888 streaming conversion
void my_disp_flush(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
    uint32_t w = (area->x2 - area->x1 + 1);
    uint32_t h = (area->y2 - area->y1 + 1);
    uint32_t total_pixels = w * h;

#if 0
    ili9488_set_window(area->x1, area->y1, area->x2, area->y2);
#endif
    st7789_set_window(area->x1, area->y1, area->x2, area->y2);

    DC_DATA();
    spi::display_cs(spi::LOW);

    lv_color16_t *colors = (lv_color16_t *)px_map;
    //uint8_t rgb565_buf[2];
    static uint8_t tx_buf[(SCREEN_WIDTH * SCREEN_HEIGHT) / 10 * BYTES_PER_PIXEL];
    //static lv_color16_t tx_buf[SCREEN_WIDTH * NUM_TX_ROW];

    //Stream 16-bit pixels straight into the SPI FIFO
    // for (uint32_t i = 0; i < (h/NUM_TX_ROW); i++) {
        uint32_t total_pixels_lim = std::min( total_pixels , (uint32_t)(SCREEN_WIDTH * SCREEN_HEIGHT / 10) );
        for(uint32_t u = 0; u < total_pixels_lim; u++) {
            // tx_buf[u] = ((*(uint16_t*)colors) >> 8) & 0xFF;
            // tx_buf[u+1] = ((*(uint16_t*)colors) & 0xFF);
            tx_buf[(u*2)] = ((*(uint16_t*)colors) >> 8) & 0xFF;
            tx_buf[(u*2)+1] = ((*(uint16_t*)colors) & 0xFF);
            colors++;
        }
#if 0
    //     //memcpy(tx_buf, colors, (SCREEN_WIDTH * NUM_TX_ROW * sizeof(lv_color16_t)));
    //     // rgb565_buf[0] = ((*(uint16_t*)colors) >> 8) & 0xFF;
    //     // rgb565_buf[1] = ((*(uint16_t*)colors) & 0xFF);
    //     //spi_write_blocking(SPI_PORT, rgb565_buf, 2);
    //     dma_channel_configure(dma_channel, &dma_config, &spi1_hw->dr, colors, 
    //         dma_encode_transfer_count(w * h * sizeof(lv_color16_t)), 
    //         true);
    //     // while ( sem == 0 ) {
    //     //     sleep_ms(TICK_PERIOD);
    //     // }
    //     // /* Release semaphore */
    //     // sem = 0;
    //     dma_channel_wait_for_finish_blocking(dma_channel);
    // }
#endif
    spi::dma_tx_start(tx_buf, total_pixels_lim * sizeof(lv_color16_t));
    //dma_channel_wait_for_finish_blocking(dma_channel);
    // spi::wait_for_completion(DISP_PID);

    // spi::display_cs(spi::HIGH);
    // lv_display_flush_ready(disp);
}

#if 1
void display_init() {
    lv_init();
#if 0
    ili9488_init();
#endif
    st7789_init();
    // bsp_lcd_brightness_init();
    // bsp_lcd_brightness_set(50);

    // Allocate 1/10th buffer sizing
    static uint8_t buf[(SCREEN_WIDTH * SCREEN_HEIGHT) / 10 * BYTES_PER_PIXEL];
    //static uint8_t buf[SCREEN_WIDTH * 40 * sizeof(lv_color16_t)];
    
    lv_display_t *disp = lv_display_create(SCREEN_WIDTH, SCREEN_HEIGHT);
    lv_display_set_default(disp);
    lv_display_set_resolution(disp, 320, 240);
    lv_display_set_rotation(disp, LV_DISPLAY_ROTATION_90);
    lv_display_set_buffers(disp, buf, NULL, sizeof(buf), LV_DISPLAY_RENDER_MODE_PARTIAL);
    lv_display_set_flush_cb(disp, my_disp_flush);
    curr_disp = disp;
}

void display_tick(const uint32_t tick_period) {
    lv_timer_handler();
    //sleep_ms(10);
    lv_tick_inc(tick_period); // Update timing directly via SDK
}

#endif

lv_obj_t* display_create_file_label()
{
    /*Change the active screen's background color*/
    lv_obj_set_style_bg_color(lv_screen_active(), lv_color_hex(0x003a57), LV_PART_MAIN);
    // lv_obj_set_style_bg_color(lv_screen_active(), lv_color_white(), LV_PART_MAIN);

    /* Create a style */
    static lv_style_t style;
    lv_style_init(&style);
    lv_style_set_text_font(&style, &lv_font_montserrat_28);  /* Set a larger font */
    /*Create a white label, set its text and align it to the center*/
    lv_obj_t * label = lv_label_create(lv_screen_active());
    lv_label_set_text(label, "");
    lv_obj_set_style_text_color(lv_screen_active(), lv_color_hex(0xffffff), LV_PART_MAIN);
    lv_obj_set_size(label, 100, 50);
    lv_obj_align(label, LV_ALIGN_CENTER, 0, 0);
    lv_obj_add_style(label, &style, 0);
    lv_label_set_long_mode(label, LV_LABEL_LONG_MODE_SCROLL_CIRCULAR);
    return label;
}

#ifdef __cplusplus
}
#endif
