#pragma once
#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/multicore.h"
//#include "mem_layout.h"
#include "ring_buffer.h"

#define BUFFER_MASK (RING_BUFFER_SIZE - 1)
//#define WATERMARK_THRESHOLD 64 // Trigger doorbell every 64 bytes
#define SHARED_BUFFER_ADDR 0x20080800

typedef struct {
    alignas(4) volatile uint32_t head; // 4-byte hardware bus alignment
    alignas(4) volatile uint32_t tail;
    uint8_t data[RING_BUFFER_SIZE];
} SharedRingBuffer;

inline SharedRingBuffer* get_shared_buffer() {
    return (SharedRingBuffer*)SHARED_BUFFER_ADDR;
}

extern "C" {
    
    void ring_buffer_init(void) {
        SharedRingBuffer* rb = get_shared_buffer();
        rb->head = 0;
        rb->tail = 0;
    }

    size_t ring_buffer_pop_burst(uint8_t* dest, const size_t length) {
        SharedRingBuffer* rb = get_shared_buffer();
        uint32_t current_head = rb->head;
        uint32_t current_tail = rb->tail;
        size_t bytes_copied = 0;

        if (current_head == current_tail) {
            return 0; // Buffer is completely empty
        }

        for (size_t i = 0; i < length; i++) {
            if (current_head == current_tail) {
                break; // buffer empty
            }
            dest[i] = rb->data[current_tail];
            current_tail = (current_tail + 1) & BUFFER_MASK;
            bytes_copied++;
        }

        if (bytes_copied > 0) {
            // // Data Memory Barrier: Flushes the CPU write-buffer out to actual SRAM
            // // before updating the head pointer.
            // __dmb(); 
            rb->tail = current_tail;
        }

        return bytes_copied;
    }

    // Optimized push that minimizes doorbell frequency
    size_t ring_buffer_push_string(const uint8_t* source, size_t length) {
        SharedRingBuffer* rb = get_shared_buffer();
        uint32_t current_head = rb->head;
        uint32_t current_tail = rb->tail;
        size_t bytes_written = 0;

        for (size_t i = 0; i < length; i++) {
            if (((current_head + 1) & BUFFER_MASK) == current_tail) {
                break; // Buffer full
            }
            rb->data[current_head] = source[i];
            current_head = (current_head + 1) & BUFFER_MASK;
            bytes_written++;
        }

        if (bytes_written > 0) {
            // // Data Memory Barrier: Flushes the CPU write-buffer out to actual SRAM
            // // before updating the head pointer.
            // __dmb(); 
            rb->head = current_head;
    #if 0
            // Calculate currently queued bytes
            uint32_t queued = (current_head >= current_tail) ? 
                            (current_head - current_tail) : 
                            (RING_BUFFER_SIZE - (current_tail - current_head));

            // Only interrupt Core 1 if we crossed the threshold
            if (queued >= WATERMARK_THRESHOLD) {
                sio_hw->doorbell_out_set = (1UL << 0);
            }
    #endif
        }

        return bytes_written;
    }
}
