//use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "spinlock")]
use embassy_rp::spinlock_mutex::blocking_mutex::*;

#[cfg(feature = "fifo")]
use rp_pac::SIO;

use core::mem;

const D_TO_G_ADDR : usize = (0x20081000);
const G_TO_D_ADDR : usize = (0x20081000 + mem::size_of::<SharedMessage>());

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SharedMessage {
    update: u32,
    length: u32,
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
    pub fn receive() -> (bool, usize) {
        let fifo = SIO.fifo();
        if fifo.st().read().vld() {
            (true, fifo.rd().read() as usize)
        } else {
            (false, 0)
        }
    }
}