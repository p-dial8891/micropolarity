//use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "spinlock")]
use embassy_rp::spinlock_mutex::blocking_mutex::*;

#[cfg(feature = "fifo")]
use rp_pac::SIO;

use core::mem;
use crate::HANDSHAKE_ADDR;
use crate::ring_buffer::Buffer;

#[cfg(feature = "spinlock")]
const D_TO_G_ADDR : usize = (0x20081000);
#[cfg(feature = "spinlock")]
const G_TO_D_ADDR : usize = (0x20081000 + mem::size_of::<SharedMessage>());

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SharedMessage {
    update: u32,
    length: u32,
}

#[repr(u32)]
#[derive(PartialEq)]
pub enum MessageId {
    NOOP = 0,
    PLAY_FILE = 1,
    GET_AUDIO = 2
}

impl SharedMessage {

    #[cfg(feature = "spinlock")]
    pub fn send(length : u32) {
        let mut message = unsafe { &mut *(G_TO_D_ADDR as *mut SharedMessage) };
        cortex_m::asm::dmb();
        let s = SpinlockMutex::<1, &mut SharedMessage>::new(message);
        unsafe { s.lock_mut(|ref mut m| {
            m.length = length;
            m.update = 1u32;
        }) };

    }

    #[cfg(feature = "spinlock")]
    pub fn receive() -> (bool, usize) {
        let mut message = unsafe { &mut *(D_TO_G_ADDR as *mut SharedMessage) };
        let s = SpinlockMutex::<0, &mut SharedMessage>::new(message);
        unsafe { s.lock_mut(|ref mut m| {
            if m.update == 1 {
                m.update = 0;
                (true, m.length as usize)
            } else {
                (false, 0usize)
            }
        }) }
    }

    #[cfg(feature = "fifo")]
    pub fn send(length : u32) {
        let fifo = SIO.fifo();
        cortex_m::asm::dmb();
        if fifo.st().read().rdy() {
            fifo.wr().write_value(length);
        }
    }

    #[cfg(feature = "fifo")]
    pub fn send_string(data : [u32;2]) {
        let fifo = SIO.fifo();
        cortex_m::asm::dmb();
        for i in 0..2 {
            if fifo.st().read().rdy() {
                fifo.wr().write_value(data[i]);
            } else {
                if i == 1 {
                    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); }
                    panic!("FIFO blocked.")
                } else {
                    break; 
                }
            }
        }
    }

    #[cfg(feature = "fifo")]
    pub fn send_message(cmd : MessageId, rb : &Buffer, data : Option<&[u8]>) {
        let fifo = SIO.fifo();
        if !fifo.st().read().rdy() {
            return;
        }
        if data.is_some() {
            rb.push_string(data.unwrap_or(&[0u8;0]));
        }
        cortex_m::asm::dmb();
        fifo.wr().write_value(cmd as u32);
        if !fifo.st().read().rdy() {
            unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); }
            panic!("FIFO blocked.")
        } else {
            fifo.wr().write_value(data.unwrap_or(&[0u8;0]).len() as u32);
        }
    }

    #[cfg(feature = "fifo")]
    pub fn receive() -> (bool, usize) {
        let fifo = SIO.fifo();
        if fifo.st().read().vld() {
            (true, fifo.rd().read() as usize)
        } else {
            (false, 0)
        }
    }

    #[cfg(feature = "fifo")]
    pub fn receive_string() -> (bool, usize) {
        let fifo = SIO.fifo();
        let mut ret = (false, 0usize);
        for i in 0..2 {
            if fifo.st().read().vld() {
                ret = (true, fifo.rd().read() as usize)
            } else {
                if i == 1 {
                    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); }
                    panic!("FIFO blocked.")
                } else {
                    break;
                }
            }
        }
        ret
    }

    #[cfg(feature = "fifo")]
    pub fn receive_message(rb : &Buffer, data : &mut [u8]) -> (MessageId, usize) {
        let fifo = SIO.fifo();
        if !fifo.st().read().vld() {
            return (MessageId::NOOP, 0usize);
        }
        let mid = match fifo.rd().read() {
            0 => { MessageId::NOOP },
            1 => { MessageId::PLAY_FILE },
            2 => { MessageId::GET_AUDIO },
            _ => { MessageId::NOOP }
        };
        if !fifo.st().read().vld() {
            unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); }
            panic!("FIFO blocked.");
            return (MessageId::NOOP, 0usize);
        }
        let _ = fifo.rd().read();
        if !rb.is_empty() && mid != MessageId::NOOP {
            let bytes = rb.pop_burst(data);
            (mid, bytes)
        } else {
            (mid, 0)
        }
    }
}