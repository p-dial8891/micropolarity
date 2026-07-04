use core::sync::atomic::{AtomicU32, Ordering};
use embassy_rp::spinlock_mutex::blocking_mutex::*;
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

    pub fn send(length : u32) {
        let mut message = unsafe { &mut *(G_TO_D_ADDR as *mut SharedMessage) };
        cortex_m::asm::dmb();
        let s = SpinlockMutex::<1, &mut SharedMessage>::new(message);
        let mut sync_m = unsafe { s.lock_mut(|_| {}) };
        message.length = length;
        message.update = 1u32;
    }

    pub fn receive() -> usize {
        let mut message = unsafe { &mut *(D_TO_G_ADDR as *mut SharedMessage) };
        let s = SpinlockMutex::<0, &mut SharedMessage>::new(message);
        let mut sync_m = unsafe { s.lock_mut(|_| {}) };
        if message.update == 1 {
            message.update = 0;
            message.length as usize
        } else {
            0usize
        }
    }

}