// mem_layout.rs
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll, Waker};
use core::cell::RefCell;
use critical_section::Mutex;

pub const BUFFER_SIZE: usize = 2*1024;
const BUFFER_MASK: u32 = (BUFFER_SIZE - 1) as u32;
const SHARED_BUFFER_ADDR: usize = 0x20080800;

#[repr(C)]
struct RawRingBuffer {
    head: AtomicU32,
    tail: AtomicU32,
    data: [u8; BUFFER_SIZE],
}

pub struct Buffer;

impl Buffer {
    pub fn new() -> Self {
        Self
    }

    // Reads as many bytes as possible out of the ring buffer at once
    pub fn pop_burst(&self, out_buf: &mut [u8]) -> usize {
        unsafe {
            let rb = &*(SHARED_BUFFER_ADDR as *const RawRingBuffer);
            
            // Acquire ordering forces a hardware fence matching C++'s __dmb()
            let current_head = rb.head.load(Ordering::Acquire);
            let current_tail = rb.tail.load(Ordering::Relaxed);

            if current_head == current_tail {
                return 0; // Buffer is completely empty
            }

            let mut tail = current_tail;
            let mut bytes_copied = 0;

            // Empty the buffer in a tight local loop
            while tail != current_head && bytes_copied < out_buf.len() {
                out_buf[bytes_copied] = rb.data[tail as usize];
                tail = (tail + 1) & BUFFER_MASK;
                bytes_copied += 1;
            }

            // Release ordering flushes the new tail pointer back to Core 0 safely
            rb.tail.store(tail, Ordering::Release);
            bytes_copied
        }
    }

    // Writes as many bytes as possible into the ring buffer at once
    pub fn push_string(&self, in_buf: &[u8]) -> usize {
        unsafe {
            let mut bytes_written = 0;
            let rb = &mut *(SHARED_BUFFER_ADDR as *mut RawRingBuffer);
            
            // Acquire ordering forces a hardware fence matching C++'s __dmb()
            let mut current_head = rb.head.load(Ordering::Relaxed);
            let current_tail = rb.tail.load(Ordering::Relaxed);

            for i in 0..in_buf.len() {
                if ((current_head + 1) & BUFFER_MASK) == current_tail {
                    break; //buffer full
                }
                rb.data[current_head as usize] = in_buf[i];
                current_head = ( current_head + 1 ) & BUFFER_MASK;
                bytes_written += 1;
            }

            let head = current_head;

            // Release ordering flushes the new tail pointer back to Core 0 safely
            rb.head.store(head, Ordering::Relaxed);
            bytes_written
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe {
            let rb = &*(SHARED_BUFFER_ADDR as *const RawRingBuffer);
            return rb.head.load(Ordering::Relaxed) == rb.tail.load(Ordering::Relaxed);
        }
    }
}