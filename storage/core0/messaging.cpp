#pragma once
#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/multicore.h"
#include "hardware/sync.h"
#include "messaging.h"
#include "ring_buffer.h"
#include <stdio.h>

#define D_TO_G_ADDR     (0x20081000)
#define G_TO_D_ADDR     (0x20081000 + sizeof(SharedMessage))

typedef struct {
    alignas(4) volatile uint32_t update; // 4-byte hardware bus alignment
    alignas(4) volatile uint32_t length;
} SharedMessage;

extern "C" {
#ifdef SPINLOCK
    void send(uint32_t length) {
        auto s = spin_lock_init(0);
        SharedMessage * message = reinterpret_cast<SharedMessage*>(D_TO_G_ADDR);

        __dmb();
        auto irq = spin_lock_blocking(s);
        message->length = length;
        message->update = 1;
        spin_unlock(s, irq);
    }

    bool receive(void) {
        //uint32_t length = 0;
        bool result = false;
        auto s = spin_lock_init(1);
        SharedMessage * message = reinterpret_cast<SharedMessage*>(G_TO_D_ADDR);

        auto irq = spin_lock_blocking(s);
        if (message->update == 1) {
            //length = message->length;
            message->update = 0;
            result = true;
        }
        spin_unlock(s, irq);

        return result;
    }
#elif defined(FIFO)
    void send(uint32_t length) {
        __dmb();
        if (multicore_fifo_wready()) {
            multicore_fifo_push_blocking(length);
        }
    }

    void send_string(uint32_t* data) {
        __dmb();
        for ( int i = 0; i < 2; i++ ) {
            if (multicore_fifo_wready()) {
                multicore_fifo_push_blocking(data[i]);
            } else {
                if ( i == 1 ) {
                    printf("FIFO blocked.");
                    while (true) {};
                } else {
                    break;
                }
            }
        }
    }

    size_t send_message(MessageId cmd, uint8_t* data, size_t len) {
        int count = FIFO_RETRY_COUNT;
        if (!multicore_fifo_wready())
            return 0;
        size_t ret = ring_buffer_push_string(const_cast<uint8_t*>(data), len);
        __dmb();
        multicore_fifo_push_blocking(cmd);
        if (!multicore_fifo_wready() && (count > 0)) {
            sleep_ms(1);
            count--;
        }
        if (count == 0) {
            printf("FIFO blocked.");
            while (true) {};
        }
        multicore_fifo_push_blocking((uint32_t)len);

        return ret;
    }

    bool receive(void) {
        if (multicore_fifo_rvalid()) {
            (void)multicore_fifo_pop_blocking();
            return true;
        } else {
            return false;
        }
    }

    bool receive_string(void) {
        bool ret = false;
        for ( int i = 0; i < 2; i++ ) {
            if (multicore_fifo_rvalid()) {
                (void)multicore_fifo_pop_blocking();
                ret = true;
            } else {
                if ( i == 1 ) {
                    printf("FIFO blocked.");
                    while (true) {};
                } else {
                    break;
                }
                ret = false;
            }
        }
        return ret;
    }

    MessageId receive_message(uint8_t* data, size_t *length) {
        int count = FIFO_RETRY_COUNT;
        if (!multicore_fifo_rvalid()) {
            return MessageId::NOOP;
        }
        MessageId mid = static_cast<MessageId>(multicore_fifo_pop_blocking());
        while (!multicore_fifo_rvalid() && (count > 0)) {
            sleep_ms(1);
            count--;
        }
        if (count == 0) {
            printf("FIFO blocked.");
            while (true) {};
        }
        size_t len = static_cast<size_t>(multicore_fifo_pop_blocking());
        if ( ( mid != MessageId::NOOP ) && ( len != 0 ) )
            *length = ring_buffer_pop_burst(data, *length);

        return mid;
    }
    
#endif
}