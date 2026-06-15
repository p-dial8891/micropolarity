#![no_std]
#![no_main]

//use cortex_m_rt::entry;
//use rp235x_hal as hal;
//use embassy_rp::block::ImageDef;
use embassy_executor::Spawner;

// Use the absolute scratchpad memory window configured in memory.x
const HANDSHAKE_ADDR: *mut u32 = 0x2008_0000 as *mut u32;

// #[unsafe(link_section = ".start_block")]
// #[used]
// pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

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

//#[embassy_executor::main(executor = "embassy_rp::executor::Executor",  entry = "cortex_m_rt::entry")]
#[embassy_executor::task]
async fn core1_async_loop(spawner: Spawner) -> ! {

    // 2. Initialize the RP2350 embassy peripherals architecture 
    let peripherals = embassy_rp::init(Default::default());

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

use embassy_rp::executor::Executor;
use static_cell::StaticCell;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    unsafe {
        const SIO_BASE: u32 = 0xd0000000;
        const SPINLOCK0_PTR: *mut u32 = (SIO_BASE + 0x100) as *mut u32;
        const SPINLOCK_COUNT: usize = 32;
        for i in 0..SPINLOCK_COUNT {
            SPINLOCK0_PTR.wrapping_add(i).write_volatile(1);
        }
        // Enable the Double-Co-Pro and the GPIO Co-Pro in the CPACR register.
        // We have to do this early, before there's a chance we might call
        // any accelerated functions.
        const SCB_CPACR_PTR: *mut u32 = 0xE000_ED88 as *mut u32;
        const SCB_CPACR_FULL_ACCESS: u32 = 0b11;
        // Do a R-M-W, because the FPU enable is here and that's already been enabled
        let mut temp = SCB_CPACR_PTR.read_volatile();
        // DCP Co-Pro is 4, two-bits per entry
        temp |= SCB_CPACR_FULL_ACCESS << (4 * 2);
        // GPIO Co-Pro is 0, two-bits per entry
        temp |= SCB_CPACR_FULL_ACCESS << (0 * 2);
        SCB_CPACR_PTR.write_volatile(temp);

    }
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

    // 3. Manually spin up the Thread-Mode Executor
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        // spawner.spawn(core1_async_loop(spawner).unwrap());
    });

}
