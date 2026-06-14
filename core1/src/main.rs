#![no_std]
#![no_main]

//use cortex_m_rt::entry;
use rp235x_hal as hal;
//use embassy_rp::block::ImageDef;

// Use the absolute scratchpad memory window configured in memory.x
const HANDSHAKE_ADDR: *mut u32 = 0x2008_0000 as *mut u32;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[inline(always)]
fn is_core1_secure() -> bool {
    let control_reg: u32;
    unsafe {
        // Read the special Core register using inline ARM assembly
        core::arch::asm!("mrs {}, CONTROL", out(reg) control_reg);
    }
    // Check Bit 3 (SFPA - Secure Floating-Point Active)
    (control_reg & (1 << 3)) != 0
}


// --- STAGE 1 TRAP (Optional) ---
// If you want to log before even reaching main, you can track the reset handler.
// For now, we capture right inside the native main entry.

#[hal::entry]
fn main() -> ! {
    // 1. SIGNAL STAGE 1: Core 1 has successfully jumped into Rust code space!
    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x1111_1111); }

    // 2. Fix the Vector Table Offset Register (VTOR) for RP2350
    unsafe {
        let vtor = 0xE000_ED08 as *mut u32;
        core::ptr::write_volatile(vtor, 0x1020_0000); // 2MB offset origin
    }

    // 3. SIGNAL STAGE 2: Memory mapped vectors are isolated, starting system init
    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x2222_2222); }

    // Initialize your peripherals safely
    //let _peripherals = embassy_rp::init(Default::default());

    // 4. SIGNAL STAGE 3: System initialized, entering execution loop
    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x3333_3333); }

    // Modify your RAM handshake logic to output the result:
    if is_core1_secure() {
        unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); } // Hex "SEC-1"
    } else {
        unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x0045c111); } // Hex "NS-1"
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
