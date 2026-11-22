#include "runtime.hpp"
#include "main.h"

Port<char [], MAX_FN_LENGTH> ui_track_name;
Runtime runtime;
Port<lv_display_t *,sizeof(lv_display_t*)> prt_curr_disp;