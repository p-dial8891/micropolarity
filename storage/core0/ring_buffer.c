#pragma once
#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/multicore.h"
//#include "mem_layout.h"

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
