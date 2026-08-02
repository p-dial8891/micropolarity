#include <string.h>
#include <iostream>
#include <algorithm>
#include "pico/stdlib.h"
#include "hardware/spi.h"

#if 1
#include "lvgl/lvgl.h"
#include "lvgl/examples/lv_examples.h"
#include "hardware/dma.h"
// #include "lvgl/src/tick/lv_tick.h"
#endif
#include "main.h"

#define SPI_PORT     spi1
#define PIN_MISO     12
#define PIN_MOSI     11
#define PIN_SCK      10
#define PIN_CS       9
#define PIN_DC       8
#define PIN_RST      15

#define SCREEN_WIDTH  320
#define SCREEN_HEIGHT 480

// Fast control macros
#define DC_CMD()   gpio_put(PIN_DC, 0)
#define DC_DATA()  gpio_put(PIN_DC, 1)
#define CS_LOW()   gpio_put(PIN_CS, 0)
#define CS_HIGH()  gpio_put(PIN_CS, 1)

#ifdef __cplusplus
extern "C" {
#endif

using namespace std;

static dma_channel_config dma_config;
static int dma_channel = 33;
static int sem = 0;

void write_cmd(uint8_t cmd) {
    DC_CMD(); CS_LOW();
    spi_write_blocking(SPI_PORT, &cmd, 1);
    CS_HIGH();
}

void write_data(uint8_t data) {
    DC_DATA(); CS_LOW();
    spi_write_blocking(SPI_PORT, &data, 1);
    CS_HIGH();
}

void ili9488_init() {
    // Initialise SPI at 30MHz (Safe ceiling for ILI9488 over standard wire)
    spi_init(SPI_PORT, 30 * 1000 * 1000);
    gpio_set_function(PIN_MISO, GPIO_FUNC_SPI);
    gpio_set_function(PIN_MOSI, GPIO_FUNC_SPI);
    gpio_set_function(PIN_SCK, GPIO_FUNC_SPI);
    
    gpio_init(PIN_CS);  gpio_set_dir(PIN_CS, GPIO_OUT);
    gpio_init(PIN_DC);  gpio_set_dir(PIN_DC, GPIO_OUT);
    gpio_init(PIN_RST); gpio_set_dir(PIN_RST, GPIO_OUT);

    // Hardware Reset
    gpio_put(PIN_RST, 0); sleep_ms(50);
    gpio_put(PIN_RST, 1); sleep_ms(120);

    write_cmd(0x11); // Sleep Out
    sleep_ms(120);

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

#define BYTES_PER_PIXEL (LV_COLOR_FORMAT_GET_SIZE(LV_COLOR_FORMAT_RGB565))
#define NUM_TX_ROW (2)

// Native LVGL Flush Callback with RGB565 to RGB888 streaming conversion
void my_disp_flush(lv_display_t *disp, const lv_area_t *area, uint8_t *px_map) {
    uint32_t w = (area->x2 - area->x1 + 1);
    uint32_t h = (area->y2 - area->y1 + 1);
    uint32_t total_pixels = w * h;

    ili9488_set_window(area->x1, area->y1, area->x2, area->y2);

    DC_DATA();
    CS_LOW();

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

    dma_channel_configure(dma_channel, &dma_config, &spi1_hw->dr, tx_buf, 
        dma_encode_transfer_count(total_pixels_lim * sizeof(lv_color16_t)), 
        true);
    dma_channel_wait_for_finish_blocking(dma_channel);

    CS_HIGH();
    lv_display_flush_ready(disp);
}

/*! \brief  Enable single DMA channel's interrupt via DMA_IRQ_2
 *  \ingroup hardware_dma
 *
 * \param channel DMA channel
 * \param enabled true to enable interrupt 2 on specified channel, false to disable.
 */
static inline void dma_channel_set_irq2_enabled(uint channel, bool enabled) {
    check_dma_channel_param(channel);
    check_hw_layout(dma_hw_t, inte2, DMA_INTE2_OFFSET);
    if (enabled)
        hw_set_bits(&dma_hw->inte2, 1u << channel);
    else
        hw_clear_bits(&dma_hw->inte2, 1u << channel);
}

// void display_dma_handler(void) {
//     // Is this channel requesting interrupt?
//     io_rw_32 * dma_hw_ints_p = &dma_hw->ints2;
//     if ((*dma_hw_ints_p & (1 << dma_channel))) {
//         *dma_hw_ints_p = 1 << dma_channel;  // Clear it.

//         /* Take semaphore */
//         sem = 1;
//     }
// }

#if 1
void display_init() {
    lv_init();
    ili9488_init();

    /* Set up DMA handler */
    cout<<"Requesting dma channel."<<endl;
    dma_channel = dma_claim_unused_channel(true);
    //irq_add_shared_handler(DMA_IRQ_2,display_dma_handler,PICO_SHARED_IRQ_HANDLER_DEFAULT_ORDER_PRIORITY);
    dma_config = dma_channel_get_default_config(dma_channel);
    channel_config_set_dreq(&dma_config, DREQ_SPI1_TX);
    channel_config_set_transfer_data_size(&dma_config, DMA_SIZE_8);
    //dma_channel_set_irq2_enabled(dma_channel, true);
    //irq_set_enabled(DMA_IRQ_2, true);

    // Allocate 1/10th buffer sizing
    static uint8_t buf[(SCREEN_WIDTH * SCREEN_HEIGHT) / 10 * BYTES_PER_PIXEL];
    //static uint8_t buf[SCREEN_WIDTH * 40 * sizeof(lv_color16_t)];
    
    lv_display_t *disp = lv_display_create(SCREEN_WIDTH, SCREEN_HEIGHT);
    lv_display_set_default(disp);
    lv_display_set_resolution(disp, 480, 320);
    lv_display_set_rotation(disp, LV_DISPLAY_ROTATION_0);
    lv_display_set_buffers(disp, buf, NULL, sizeof(buf), LV_DISPLAY_RENDER_MODE_PARTIAL);
    lv_display_set_flush_cb(disp, my_disp_flush);

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

    /* Create a style */
    static lv_style_t style;
    lv_style_init(&style);
    lv_style_set_text_font(&style, &lv_font_montserrat_28);  /* Set a larger font */
    /*Create a white label, set its text and align it to the center*/
    lv_obj_t * label = lv_label_create(lv_screen_active());
    lv_label_set_text(label, "");
    lv_obj_set_style_text_color(lv_screen_active(), lv_color_hex(0xffffff), LV_PART_MAIN);
    lv_obj_set_size(label, 300, 200);
    lv_obj_align(label, LV_ALIGN_CENTER, 0, 0);
    lv_obj_add_style(label, &style, 0);
    lv_label_set_long_mode(label, LV_LABEL_LONG_MODE_SCROLL_CIRCULAR);
    return label;
}

#ifdef __cplusplus
}
#endif
