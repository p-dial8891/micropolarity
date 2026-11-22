#pragma once

#include <functional>
#include <string.h>
#include <algorithm>
#include "main.h"
#include "lvgl/lvgl.h"

struct Runtime {
    std::function<bool(void)> routine;
};

template <typename T, size_t N = sizeof(T)>
struct Port {
    size_t count;
    char data[N];

    Port() = default;

    bool push(T& input, size_t length) {
        if ( count < 1 ) {
            memcpy(data, &input, std::min(N, length));
            count++;
            return true;
        } else {
            return false;
        }
    }

    bool pop(T& output, size_t length) {
        if ( count > 0 ) {
            memcpy(&output, data, std::min(N, length));
            count--;
            return true;
        } else {
            return false;
        }
    }

    bool peek(T& output, size_t length) {
        if ( count > 0 ) {
            memcpy(&output, data, std::min(N, length));
            return true;
        } else {
            return false;
        }
    }

    // The '*_ref' variant is useful in cases where input is a reference
    // or a pointer type and you want to use its referenced data 
    bool push_ptr(const T& input, size_t length) {
        if ( count < 1 ) {
            memcpy(data, input, std::min(N, length));
            count++;
            return true;
        } else {
            return false;
        }
    }

    bool pop_ptr(T& output, size_t length) {
        if ( count > 0 ) {
            memcpy(output, data, std::min(N, length));
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
extern Port<char *, MAX_FN_LENGTH> ui_track_name;
extern Port<lv_display_t*> prt_curr_disp;