#pragma once
#define RING_BUFFER_SIZE (2*1024) // Increased size (Must be power of two)

#ifdef __cplusplus
extern "C" {
#endif

void ring_buffer_init(void);
size_t ring_buffer_pop_burst(uint8_t* dest, const size_t length);
size_t ring_buffer_push_string(const uint8_t* source, size_t length);

#ifdef __cplusplus
}
#endif
