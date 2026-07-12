//#define SPINLOCK
#define FIFO

#ifdef __cplusplus
extern "C" {
#endif

void send(uint32_t length);
bool receive(void);
#ifdef FIFO
void send_string(uint32_t* data);
bool receive_string(void);
#endif

#ifdef __cplusplus
}
#endif
