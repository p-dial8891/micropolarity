#pragma once
#include "pico/stdlib.h"

#define DISPLAY_PIN_CS      9
#define TOUCH_PIN_CS        16

namespace spi {

    enum Command {
        LOW = 0,
        HIGH
    };

    static inline void display_cs(Command command) {
        switch (command) {
            case LOW : {
                gpio_put(DISPLAY_PIN_CS, 0);
                gpio_put(TOUCH_PIN_CS, 1);
                break;
            }
            case HIGH : {
                gpio_put(DISPLAY_PIN_CS, 1);
                break;
            }
        }
    }

    static inline void touch_cs(Command command) {
        switch (command) {
            case LOW : {
                gpio_put(TOUCH_PIN_CS, 0);
                gpio_put(DISPLAY_PIN_CS, 1);
                break;
            }
            case HIGH : {
                gpio_put(TOUCH_PIN_CS, 1);
                break;
            }
        }
    }
    void dma_tx_start(uint8_t* source, size_t tx_count);
    void dma_rx_start(uint8_t* destination, size_t rx_count);
    void wait_for_completion(int pid);
    void dma_init();
    void spi_init();
}
