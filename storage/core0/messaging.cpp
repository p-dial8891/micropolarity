#pragma once
#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/multicore.h"
#include "hardware/sync.h"

#define D_TO_G_ADDR     (0x20081000)
#define G_TO_D_ADDR     (0x20081000 + sizeof(SharedMessage))

typedef struct {
    alignas(4) volatile uint32_t update; // 4-byte hardware bus alignment
    alignas(4) volatile uint32_t length;
} SharedMessage;

extern "C" {
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
}