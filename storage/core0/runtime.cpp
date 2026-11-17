#include "runtime.hpp"
#include "main.h"

Port<char [], MAX_FN_LENGTH> ui_track_name;
Runtime runtime;
lv_display_t * curr_disp = NULL;