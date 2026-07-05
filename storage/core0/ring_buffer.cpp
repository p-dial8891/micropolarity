#pragma once
#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/multicore.h"
//#include "mem_layout.h"

#if 0
#define BUFFER_SIZE 256 // Must be a power of two
#define BUFFER_MASK (BUFFER_SIZE - 1)

// Packed structure shared between C++ and Rust
typedef struct {
    volatile uint32_t head;            // Written by Producer
    volatile uint32_t tail;            // Written by Consumer
    uint8_t data[BUFFER_SIZE];         // Array block
} SharedRingBuffer;

// Point directly to an unallocated high SRAM zone
#define SHARED_BUFFER_ADDR 0x2007F000
inline SharedRingBuffer* get_shared_buffer() {
    return (SharedRingBuffer*)SHARED_BUFFER_ADDR;
}

void ring_buffer_init() {
    SharedRingBuffer* rb = get_shared_buffer();
    rb->head = 0;
    rb->tail = 0;
}

bool ring_buffer_push(uint8_t byte) {
    SharedRingBuffer* rb = get_shared_buffer();
    
    uint32_t current_head = rb->head;
    uint32_t current_tail = rb->tail;

    // Check if buffer is completely full
    if (((current_head + 1) & BUFFER_MASK) == current_tail) {
        return false; // Buffer overflow
    }

    rb->data[current_head] = byte;
    
    // Ensure memory write finishes before index is incremented (Memory Barrier)
    __dmb(); 
    rb->head = (current_head + 1) & BUFFER_MASK;
    return true;
}

#include "pico/stdlib.h"
#include "hardware/structs/sio.h"
#endif

#define BUFFER_SIZE 256 // Increased size (Must be power of two)
#define BUFFER_MASK (BUFFER_SIZE - 1)
//#define WATERMARK_THRESHOLD 64 // Trigger doorbell every 64 bytes
#define SHARED_BUFFER_ADDR 0x20080800

typedef struct {
    alignas(4) volatile uint32_t head; // 4-byte hardware bus alignment
    alignas(4) volatile uint32_t tail;
    uint8_t data[BUFFER_SIZE];
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
            // Data Memory Barrier: Flushes the CPU write-buffer out to actual SRAM
            // before updating the head pointer.
            __dmb(); 
            rb->head = current_head;
    #if 0
            // Calculate currently queued bytes
            uint32_t queued = (current_head >= current_tail) ? 
                            (current_head - current_tail) : 
                            (BUFFER_SIZE - (current_tail - current_head));

            // Only interrupt Core 1 if we crossed the threshold
            if (queued >= WATERMARK_THRESHOLD) {
                sio_hw->doorbell_out_set = (1UL << 0);
            }
    #endif
        }

        return bytes_written;
    }

}
