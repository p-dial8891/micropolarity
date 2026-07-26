//#define SPINLOCK
#define FIFO

#ifdef __cplusplus
extern "C" {
#endif

#ifdef __cplusplus
enum MessageId : uint32_t {
    NOOP = 0,
    PLAY_FILE = 1,
    GET_AUDIO = 2
};
enum MidErrorCode : uint32_t {
    NO_ERROR = 0,
    FIFO_BLOCKED
};

#else
typedef enum {
    MID_NOOP = 0,
    MID_PLAY_FILE = 1,
    MID_GET_AUDIO = 2
} MessageId;
typedef enum {
    MID_NO_ERROR = 0,
    MID_FIFO_BLOCKED
} MidErrorCode;
#endif

void send(uint32_t length);
bool receive(void);
#ifdef FIFO
static const int FIFO_RETRY_COUNT = 3;
void send_string(uint32_t* data);
size_t send_message(MessageId cmd, uint8_t* data, size_t len, uint32_t * error);
bool receive_string(void);
MessageId receive_message(uint8_t* data, size_t * length, uint32_t * error);
#endif

#ifdef __cplusplus
}
#endif
