#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
// For USB
use embassy_rp::{peripherals::USB, usb};
use embassy_rp::{bind_interrupts, dma};
use cyw43::aligned_bytes;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use embassy_rp::peripherals::{DMA_CH0, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt::{self, InterruptExt};
use embassy_rp::interrupt::typelevel::{Binding, Handler};
use crate::ring_buffer::CORE1_TERMINATE;
use critical_section::Mutex;
use core::cell::RefCell;
use core::task::{Poll};

mod ring_buffer;

// Use the absolute scratchpad memory window configured in memory.x
const HANDSHAKE_ADDR: *mut u32 = 0x2008_0000 as *mut u32;

// #[unsafe(link_section = ".start_block")]
// #[used]
// pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

bind_interrupts!(struct Irqs {
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    SIO_IRQ_BELL => SioInterruptHandler;
});

#[derive(Copy, Clone)]
struct SioInterruptHandler;

impl Handler<interrupt::typelevel::SIO_IRQ_BELL> for SioInterruptHandler {
    unsafe fn on_interrupt() {
        // Clear the hardware FIFO flag by reading the raw SIO register
        let sio = rp_pac::SIO;

        // Read which doorbells are active
        let active_doorbells = sio.doorbell_in_clr().read();
        
        // Check if Doorbell 0 caused this interrupt
        if (active_doorbells.doorbell_in_clr() & (1 << 0)) != 0 {
            // Clear Doorbell 0 so the interrupt line drops back to low
            sio.doorbell_in_clr().write_value(rp_pac::sio::regs::DoorbellInClr(1 << 0));
            
            // Wake any suspended Embassy tasks awaiting buffer elements
            critical_section::with(|cs| {
                CORE1_TERMINATE.borrow(cs).replace(Some(()));
            });
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO1, 0>>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn core1_consumer_task() {
    log::info!("Starting consumer task.");
    let rb = ring_buffer::AsyncRingBufferReader::new();
    loop {
        let mut f = None;
        critical_section::with(|cs| {
            f = *CORE1_TERMINATE.borrow(cs).borrow_mut();      
        });
        if f == Some(()) {
            log::info!("Doorbell rang. Terminating receive.");
            return;
        }
        else {
            if let Some(byte) = rb.try_pop() {
                log::info!("Byte received: {}", byte);
            }
            Timer::after(Duration::from_millis(500)).await;
        }
    }
}

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

//#[embassy_executor::main(executor = "embassy_rp::executor::Executor",  entry = "cortex_m_rt::entry")]
#[embassy_executor::task]
async fn core1_wifi_loop(
    spawner: embassy_executor::Spawner,
    p_23 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_23>,
    p_25 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_25>,
    p_24 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_24>,
    p_29 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_29>,    
    pio1 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIO1>,
    dma : embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH0>
) {

    // 2. Initialize the RP2350 embassy peripherals architecture 
    //let peripherals = embassy_rp::init(Default::default());
    log::info!("wifi loop started...");

    let fw = aligned_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../../embassy/cyw43-firmware/nvram_rp2040.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p_23, Level::Low);
    let cs = Output::new(p_25, Level::High);
    let mut pio = Pio::new(pio1, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
        // See: https://github.com/embassy-rs/embassy/issues/3960.
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p_24,
        p_29,
        dma::Channel::new(dma, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let delay = Duration::from_millis(250);
    loop {
        // info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        // info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }

}


#[embassy_executor::task]
async fn core1_async_loop() {

    // 2. Initialize the RP2350 embassy peripherals architecture 
    //let peripherals = embassy_rp::init(Default::default());
    log::info!("async loop started...");
    let mut toggle = false;
    loop {
        Timer::after(Duration::from_millis(2000)).await;
        if toggle {
            unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x4EC07111); }
        } else {
            unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); }

        }
        toggle = !toggle;
        log::info!("async loop processing...");
    }
    // unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x4EC07111); }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

use embassy_executor::Executor;
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

    // Bring up the Embassy RP drivers (using existing C++ clock trees)
    // let p = unsafe { embassy_rp::Peripherals::steal() } ;
    let p = embassy_rp::init_without_clocks();
    // unsafe { 
    //     embassy_rp::time_driver::init(); 
    //     embassy_rp::dma::init();
    //     embassy_rp::gpio::init();
    // }

    // 4. SIGNAL STAGE 3: System initialized, entering execution loop
    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x3333_3333); }

    // // Modify your RAM handshake logic to output the result:
    // if is_core1_secure() {
    //     unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x5EC07111); } // Hex "SEC-1"
    // } else {
    //     unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x0045c111); } // Hex "NS-1"
    // }

    // let g = embassy_rp::gpio::Input::new(p.PIN_2, embassy_rp::gpio::Pull::Up);
    // loop {
    //     if g.is_low() {
    //         unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x6EC07111); }
    //     } else {
    //         unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x7EC07111); }
    //     }
    // }
    unsafe {
        //interrupt::SIO_IRQ_PROC1.bind(SioInterruptHandler);
        interrupt::SIO_IRQ_BELL.enable();
    }

    // 3. Manually spin up the Thread-Mode Executor
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(logger_task(p.USB).unwrap());
        spawner.spawn(core1_async_loop().unwrap());
        spawner.spawn(core1_wifi_loop(
            spawner,
            p.PIN_23, 
            p.PIN_25, 
            p.PIN_24, 
            p.PIN_29, 
            p.PIO1, 
            p.DMA_CH0)
        .unwrap());
        spawner.spawn(core1_consumer_task().unwrap());
    });

}
