// mem_layout.rs
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll, Waker};
use core::cell::RefCell;
use critical_section::Mutex;

const BUFFER_SIZE: usize = 256;
const BUFFER_MASK: u32 = (BUFFER_SIZE - 1) as u32;
const SHARED_BUFFER_ADDR: usize = 0x2007F000;

// Global thread-safe flag
pub static CORE1_TERMINATE: Mutex<RefCell<Option<()>>> = Mutex::new(RefCell::new(None));

#[repr(C)]
struct RawRingBuffer {
    head: AtomicU32,
    tail: AtomicU32,
    data: [u8; BUFFER_SIZE],
}

pub struct AsyncRingBufferReader;

impl AsyncRingBufferReader {
    pub fn new() -> Self {
        Self
    }

    // Raw atomic pop operation
    pub fn try_pop(&self) -> Option<u8> {
        unsafe {
            let rb = &*(SHARED_BUFFER_ADDR as *const RawRingBuffer);
            let current_head = rb.head.load(Ordering::Acquire);
            let current_tail = rb.tail.load(Ordering::Relaxed);

            if current_head == current_tail {
                return None; // Buffer is empty
            }

            let byte = rb.data[current_tail as usize];
            
            // Increment tail safely
            rb.tail.store((current_tail + 1) & BUFFER_MASK, Ordering::Release);
            Some(byte)
        }
    }
}
