#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "hardware/spi.h"
#include "runtime.hpp"
#include "board/spi.hpp"
#include "hardware/dma.h"
#include "touch.hpp"
#include "main.h"

void touch_init() {
    uint8_t command[2] = { 0x80, 0x0 };
    uint8_t response;

    printf("Initialising touch.\n");
    gpio_init(TOUCH_PIN_IRQ);
    gpio_set_dir(TOUCH_PIN_IRQ, GPIO_IN);

    spi::touch_cs(spi::LOW);
    spi_write_read_blocking(spi1, (uint8_t*)&command[0], (uint8_t*)&response, 1);
    sleep_us(200);
    spi_write_read_blocking(spi1, (uint8_t*)&command[1], (uint8_t*)&response, 1);
    spi_write_read_blocking(spi1, (uint8_t*)&command[1], (uint8_t*)&response, 1);
    sleep_us(200);
    spi::touch_cs(spi::HIGH); 

}

uint16_t touch_get_x_pos() {
    int count = 3;
    uint8_t response[3] = {};
    uint8_t command[3] =
    {
        {
            // ( 1 << 7 ) | // START
            // ( 5 << 4 ) | // CONTROL
            // ( 0 << 3 ) | // MODE
            // ( 1 << 2 ) | // SER/DFR
            // ( 0 << 0 )   // POWER DOWN
            0xD0
        },
        {
            0
        },
        {
            0
        }
    };
    spi::touch_cs(spi::LOW);
    //sleep_ms(50);
    //spi::dma_tx_start(&command, sizeof(command));
    //spi::wait_for_completion(TOUCH_PID);
    //spi::dma_rx_start((uint8_t*)&response, sizeof(response)/sizeof(uint8_t));
    //spi::wait_for_completion(TOUCH_PID);
    // while ( count > 0 ){
    //     count -= spi_write_read_blocking(spi1, (uint8_t*)&command[0], (uint8_t*)&response[0], 3);
    spi_write_read_blocking(spi1, (uint8_t*)&command[0], (uint8_t*)&response[0], 1);
    sleep_ms(1);
    spi_write_read_blocking(spi1, (uint8_t*)&command[1], (uint8_t*)&response[1], 1);
    spi_write_read_blocking(spi1, (uint8_t*)&command[2], (uint8_t*)&response[2], 1);
    //spi_write_blocking(spi1, (uint8_t*)&command, 1);
    //spi_read_blocking(spi1, command, (uint8_t*)&response, 2);
    spi::touch_cs(spi::HIGH); 

    return ( ( response[1] << 8 ) | response[2] ) >> 3 ;
}