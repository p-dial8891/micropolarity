/* main.c
Copyright 2021 Carl John Kugler III

Licensed under the Apache License, Version 2.0 (the License); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at

   http://www.apache.org/licenses/LICENSE-2.0
Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an AS IS BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
*/

#include <stdio.h>
#include <stdint.h>
//
#include "pico/stdlib.h"
//
#include "f_util.h"
#include "ff.h"
#include "hw_config.h"
#include "pico/multicore.h"
/*

This file should be tailored to match the hardware design.

See
https://github.com/carlk3/no-OS-FatFS-SD-SDIO-SPI-RPi-Pico/tree/main#customizing-for-the-hardware-configuration

*/

#include "hw_config.h"

extern bool ring_buffer_push(uint8_t byte);

#define HANDSHAKE_ADDR      ((volatile uint32_t*)0x20080000)
// RP2350 updated origins based on the new memory partition
#define RUST_FLASH_ORIGIN   0x10200000
#define RUST_RAM_END        (0x20010000 + (448 * 1024)) // 0x20080000

/* SDIO Interface */
static sd_sdio_if_t sdio_if = {
    /*
    Pins CLK_gpio, D1_gpio, D2_gpio, and D3_gpio are at offsets from pin D0_gpio.
    The offsets are determined by sd_driver\SDIO\rp2040_sdio.pio.
        CLK_gpio = (D0_gpio + SDIO_CLK_PIN_D0_OFFSET) % 32;
        As of this writing, SDIO_CLK_PIN_D0_OFFSET is 18,
            which is -14 in mod32 arithmetic, so:
        CLK_gpio = D0_gpio -14.
        D1_gpio = D0_gpio + 1;
        D2_gpio = D0_gpio + 2;
        D3_gpio = D0_gpio + 3;
    */
    .DMA_IRQ_num = DMA_IRQ_1,
    .SDIO_PIO = pio2,
    .CMD_gpio = 18,
    .D0_gpio = 19,
    .baud_rate = 125 * 1000 * 1000 / 6  // 20833333 Hz
};

/* Hardware Configuration of the SD Card socket "object" */
static sd_card_t sd_card = {.type = SD_IF_SDIO, .sdio_if_p = &sdio_if};

/**
 * @brief Get the number of SD cards.
 *
 * @return The number of SD cards, which is 1 in this case.
 */
size_t sd_get_num() { return 1; }

/**
 * @brief Get a pointer to an SD card object by its number.
 *
 * @param[in] num The number of the SD card to get.
 *
 * @return A pointer to the SD card object, or @c NULL if the number is invalid.
 */
sd_card_t* sd_get_by_num(size_t num) {
    if (0 == num) {
        // The number 0 is a valid SD card number.
        // Return a pointer to the sd_card object.
        return &sd_card;
    } else {
        // The number is invalid. Return @c NULL.
        return NULL;
    }
}

#define SIO_BASE            0xd0000000
#define SIO_CORE1_MPU_BASE  ((volatile uint32_t*)(SIO_BASE + 0x2b0))
#define SIO_CORE1_MPU_CTRL  ((volatile uint32_t*)(SIO_BASE + 0x2bc))

void direct_boot_core1(uint32_t entry_addr, uint32_t stack_ptr, uint32_t vtor) {
    // Reset Core 1 to ensure it is sitting in the clean ROM sleep loop
    multicore_reset_core1();
    sleep_ms(10);

    // Write the raw ARM boot parameters straight into the SIO registers
    // The RP2350 ROM polls these registers natively when awoken by an event.
    SIO_CORE1_MPU_BASE[0] = vtor;       // Register Vector Table Address
    SIO_CORE1_MPU_BASE[1] = stack_ptr;  // Register Stack Pointer
    SIO_CORE1_MPU_BASE[2] = entry_addr; // Register Reset Vector (with thumb bit!)
    
    // Fire a Send-Event instruction to wake Core 1 up out of its low-power sleep
    __asm volatile("sev");
}

/**
 * @brief The main function of the program.
 *
 * @details This function initializes the stdio interface, prints a greeting to the
 * console, mounts the SD card, writes a message to a file, and unmounts the SD card.
 *
 */
int main() {
    stdio_init_all();
    sleep_ms(2000); // Wait for serial monitor to connect
#if 1
    puts("Hello, world!");

    // See FatFs - Generic FAT Filesystem Module, "Application Interface",
    // http://elm-chan.org/fsw/ff/00index_e.html
    FATFS fs;
    FRESULT fr = f_mount(&fs, "", 1);
    if (FR_OK != fr) {
        panic("f_mount error: %s (%d)\n", FRESULT_str(fr), fr);
        return -1;
    }

    FIL fil;
    const char* const filename = "filename.txt";
    fr = f_open(&fil, filename, FA_OPEN_APPEND | FA_WRITE);
    if (FR_OK != fr && FR_EXIST != fr) {
        panic("f_open(%s) error: %s (%d)\n", filename, FRESULT_str(fr), fr);
        return -1;
    }

    if (f_printf(&fil, "Hello, world!\n") < 0) {
        printf("f_printf failed\n");
    }

    fr = f_close(&fil);
    if (FR_OK != fr) {
        printf("f_close error: %s (%d)\n", FRESULT_str(fr), fr);
    }

    f_unmount("");
#if 0
    puts("Goodbye, world!");
    for (;;) {
        puts("Goodbye, world!");
        sleep_ms(1000);
    }
#endif
#endif
    printf("\n--- Starting Multicore Handshake Monitor ---\n");

    // 1. Force state to RESET
    *HANDSHAKE_ADDR = 0x00000000;

    // 2. Unpack the vector table array
    uint32_t* rust_vector_table = (uint32_t*)RUST_FLASH_ORIGIN;
    uint32_t rust_stack_pointer = rust_vector_table[0]; 
    uint32_t rust_entry_address = rust_vector_table[1]; 

    // MANDATORY FIX: Force thumb mode bit high for Cortex-M33
    //rust_entry_address |= 1;

    printf("Rust Vector Table Address: 0x%08X\n", RUST_FLASH_ORIGIN);
    printf("Extracted Core 1 Stack Pointer: 0x%08X\n", rust_stack_pointer);
    printf("Extracted Core 1 Entry Point: 0x%08X\n", rust_entry_address);

    // 3. Fire the launch sequence
    // multicore_reset_core1(); // Clear any debugger stalls
    // sleep_ms(10);
    multicore_launch_core1_raw((void (*)())rust_entry_address, (uint32_t*)rust_stack_pointer, RUST_FLASH_ORIGIN);
    //direct_boot_core1(rust_entry_address, rust_stack_pointer, RUST_FLASH_ORIGIN);

    ring_buffer_init();
    gpio_init(2);
    gpio_pull_up(2);
    gpio_set_dir(2, GPIO_IN);

    bool latch = false;
    // 4. Trace the handshake transitions
    uint32_t last_state = 0xFFFFFFFF;
    while (1) {
        uint32_t current_state = *HANDSHAKE_ADDR;

        if (current_state != last_state) {
            last_state = current_state;

            switch (current_state) {
                case 0x00000000:
                    printf("[Core 0 Log]: Core 1 hasn't responded yet.\n");
                    break;
                case 0x11111111:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 reached Rust main entry.\n");
                    break;
                case 0x22222222:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 configured its VTOR registers.\n");
                    break;
                case 0x33333333:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 completed init and is running.\n");
                    break;
                // case 0x5EC07111:
                //     printf("[Core 1 Sync]: SUCCESS! Core 1 is running in secure mode.\n");
                //     break;
                // case 0x0045c111:
                //     printf("[Core 1 Sync]: SUCCESS! Core 1 is running in non-secure mode.\n");
                //     break;
                case 0x4EC07111:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 toggle on.\n");
                    break;
                case 0x5EC07111:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 toggle off.\n");
                    break;
                case 0x6EC07111:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 detected key press.\n");
                    break;
                case 0x7EC07111:
                    printf("[Core 1 Sync]: SUCCESS! Core 1 detected key release.\n");
                    break;
                default:
                    // If you see a completely random address value here, Core 1 hard-faulted
                    // and printed its stack trace memory markers over the handshake window.
                    printf("[CRITICAL FAULT]: Core 1 crashed! Handshake register state: 0x%08X\n", current_state);
                    break;
            }
        }

        if ((gpio_get(2) == 0)) {
            // Fire Doorbell 0 to alert Core 1.
            // Writing a 1 to bit 0 sets the doorbell flag for the opposite core.
            latch = true;
            ring_buffer_push(0x42);
            sio_hw->doorbell_out_set = (1UL << 0); 
        }
        if (latch) {
            ring_buffer_push(0x42);
        }

        sleep_ms(50);
    }
}


