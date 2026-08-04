#pragma once

#include <functional>
#include <string.h>
#include <algorithm>
#include "main.h"

struct Runtime {
    std::function<bool(void)> routine;
};

template <typename T, size_t N>
struct Port {
    size_t count;
    char data[N];

    bool push(T& input, size_t length) {
        if ( count < 1 ) {
            memcpy((void*)data, (void*)&input, std::min(N, length));
            count++;
            return true;
        } else {
            return false;
        }
    }

    bool pop(T& output, size_t length) {
        if ( count > 0 ) {
            memcpy((void*)&output, (void*)data, std::min(N, length));
            count--;
            return true;
        } else {
            return false;
        }
    }

    bool push_deref(T& input, size_t length) {
        if ( count < 1 ) {
            memcpy((void*)data, (void*)input, std::min(N, length));
            count++;
            return true;
        } else {
            return false;
        }
    }

    bool pop_deref(T& output, size_t length) {
        if ( count > 0 ) {
            memcpy((void*)output, (void*)data, std::min(N, length));
            count--;
            return true;
        } else {
            return false;
        }
    }

    bool is_available() {
        return count > 0;
    }
    
};

extern struct Runtime runtime;
extern Port<char [], MAX_FN_LENGTH> ui_track_name;