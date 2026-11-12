#include <string.h>
#include <iostream>
#include "hardware/spi.h"
#include "hardware/dma.h"
#include "runtime.hpp"
#include "board/spi.hpp"

namespace spi {

    static bool initialised = false;
    static dma_channel_config dma_config;
    static int dma_channel = 33;
    static volatile int sem = 0;

    static void dma_handler(void) {
        // Is this channel requesting interrupt?
        io_rw_32 * dma_hw_ints_p = &dma_hw->ints2;
        if ((*dma_hw_ints_p & (1 << dma_channel))) {
            *dma_hw_ints_p = 1 << dma_channel;  // Clear it.

            /* Release semaphore */
            sem = 1;
        }
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

    void dma_tx_start(uint8_t* source, size_t tx_count) {
        /* Acquire semaphore */
        sem = 0;
        channel_config_set_dreq(&dma_config, DREQ_SPI1_TX);
        dma_channel_configure(dma_channel, &dma_config, &spi1_hw->dr, source, 
            dma_encode_transfer_count(tx_count), true);
    }

    void dma_rx_start(uint8_t* destination, size_t rx_count) {
        /* Acquire semaphore */
        sem = 0;
        channel_config_set_dreq(&dma_config, DREQ_SPI1_RX);
        dma_channel_configure(dma_channel, &dma_config, destination, &spi1_hw->dr,  
            dma_encode_transfer_count(rx_count), true);
    }

    void wait_for_completion(int pid) {
        std::cout<<"Process "<<pid<<" requested wait."<<std::endl;
        while ( sem == 0 ) {
            sleep_ms(TICK_PERIOD);
            runtime.routine();
        }
    }

    void dma_init() {

        if ( initialised == false ) {
            /* Set up DMA handler */
            std::cout<<"Requesting dma channel."<<std::endl;
            dma_channel = dma_claim_unused_channel(true);
            irq_add_shared_handler(DMA_IRQ_2,dma_handler,PICO_SHARED_IRQ_HANDLER_DEFAULT_ORDER_PRIORITY);
            dma_config = dma_channel_get_default_config(dma_channel);
            channel_config_set_transfer_data_size(&dma_config, DMA_SIZE_8);
            dma_channel_set_irq2_enabled(dma_channel, true);
            irq_set_enabled(DMA_IRQ_2, true);
            initialised = true;
        }
    }

    void spi_init() {
        
        gpio_init(DISPLAY_PIN_CS);
        gpio_set_dir(DISPLAY_PIN_CS, GPIO_OUT);

        gpio_init(TOUCH_PIN_CS);  
        gpio_set_dir(TOUCH_PIN_CS, GPIO_OUT);

        // Initialise SPI at 30MHz (Safe ceiling for ILI9488 over standard wire)
        spi_init(spi1, 1 * 1000 * 1000);

    }

}

