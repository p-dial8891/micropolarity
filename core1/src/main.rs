#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
// For USB
use embassy_rp::{peripherals::USB, usb};
use embassy_rp::{bind_interrupts, dma};
use embassy_rp::peripherals::{DMA_CH10, DMA_CH11, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt::{self, InterruptExt};
use embassy_rp::interrupt::typelevel::{Binding, Handler};
use critical_section::Mutex;
use core::cell::RefCell;
use core::task::{Poll};

mod ring_buffer;
mod messaging;
mod player;

// Use the absolute scratchpad memory window configured in memory.x
pub const HANDSHAKE_ADDR: *mut u32 = 0x2008_0000 as *mut u32;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH11>;
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

// #[embassy_executor::task]
// async fn core1_consumer_task(
//     p_2 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_2>
// ) {
//     log::info!("Starting consumer task.");
//     let mut data = [0u8; 256];
//     let rb = ring_buffer::AsyncBurstReader::new();
//     let g = embassy_rp::gpio::Input::new(p_2, embassy_rp::gpio::Pull::Up);
//     loop {
//         if g.is_low() {
//             messaging::SharedMessage::send(1);
//         }
//         let (_, recv_len) = messaging::SharedMessage::receive();
//         if recv_len > 0 {
//             let bytes_popped = rb.pop_burst(&mut data[0..recv_len]);
//             for i in 0u8..(recv_len as u8) {
//                 if data[i as usize] != i {
//                     log::warn!("Error in transmission: {} vs {}", data[i as usize], i);
//                 }
//                 data[i as usize] = 0;
//             }
//             log::info!("Received {} bytes and popped {} bytes.", recv_len, bytes_popped);
//         }
//         Timer::after(Duration::from_millis(50)).await;
//     }
// }

// //#[embassy_executor::main(executor = "embassy_rp::executor::Executor",  entry = "cortex_m_rt::entry")]
// #[embassy_executor::task]
// async fn core1_wifi_loop(
//     spawner: embassy_executor::Spawner,
//     p_23 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_23>,
//     p_25 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_25>,
//     p_24 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_24>,
//     p_29 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_29>,    
//     pio1 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIO1>,
//     dma : embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH0>
// ) {

//     // 2. Initialize the RP2350 embassy peripherals architecture 
//     //let peripherals = embassy_rp::init(Default::default());
//     log::info!("wifi loop started...");

//     let fw = aligned_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
//     let clm = aligned_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");
//     let nvram = aligned_bytes!("../../embassy/cyw43-firmware/nvram_rp2040.bin");

//     // To make flashing faster for development, you may want to flash the firmwares independently
//     // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
//     //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
//     //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
//     //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
//     //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

//     let pwr = Output::new(p_23, Level::Low);
//     let cs = Output::new(p_25, Level::High);
//     let mut pio = Pio::new(pio1, Irqs);
//     let spi = PioSpi::new(
//         &mut pio.common,
//         pio.sm0,
//         // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
//         // See: https://github.com/embassy-rs/embassy/issues/3960.
//         RM2_CLOCK_DIVIDER,
//         pio.irq0,
//         cs,
//         p_24,
//         p_29,
//         dma::Channel::new(dma, Irqs),
//     );

//     static STATE: StaticCell<cyw43::State> = StaticCell::new();
//     let state = STATE.init(cyw43::State::new());
//     let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
//     spawner.spawn(cyw43_task(runner).unwrap());

//     control.init(clm).await;
//     control
//         .set_power_management(cyw43::PowerManagementMode::PowerSave)
//         .await;

//     let delay = Duration::from_millis(250);
//     loop {
//         // info!("led on!");
//         control.gpio_set(0, true).await;
//         Timer::after(delay).await;

//         // info!("led off!");
//         control.gpio_set(0, false).await;
//         Timer::after(delay).await;
//     }

// }


#[embassy_executor::task]
async fn core1_async_loop() {

    // 2. Initialize the RP2350 embassy peripherals architecture 
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
    // unsafe {
    //     const SIO_BASE: u32 = 0xd0000000;
    //     const SPINLOCK0_PTR: *mut u32 = (SIO_BASE + 0x100) as *mut u32;
    //     const SPINLOCK_COUNT: usize = 32;
    //     for i in 0..SPINLOCK_COUNT {
    //         SPINLOCK0_PTR.wrapping_add(i).write_volatile(1);
    //     }
    //     // Enable the Double-Co-Pro and the GPIO Co-Pro in the CPACR register.
    //     // We have to do this early, before there's a chance we might call
    //     // any accelerated functions.
    //     const SCB_CPACR_PTR: *mut u32 = 0xE000_ED88 as *mut u32;
    //     const SCB_CPACR_FULL_ACCESS: u32 = 0b11;
    //     // Do a R-M-W, because the FPU enable is here and that's already been enabled
    //     let mut temp = SCB_CPACR_PTR.read_volatile();
    //     // DCP Co-Pro is 4, two-bits per entry
    //     temp |= SCB_CPACR_FULL_ACCESS << (4 * 2);
    //     // GPIO Co-Pro is 0, two-bits per entry
    //     temp |= SCB_CPACR_FULL_ACCESS << (0 * 2);
    //     SCB_CPACR_PTR.write_volatile(temp);

    // }
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
    let p = embassy_rp::init_without_clocks();

    // 4. SIGNAL STAGE 3: System initialized, entering execution loop
    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x3333_3333); }

    // 3. Manually spin up the Thread-Mode Executor
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(logger_task(p.USB).unwrap());
        //spawner.spawn(core1_async_loop().unwrap());
        // spawner.spawn(core1_wifi_loop(
        //     spawner,
        //     p.PIN_23, 
        //     p.PIN_25, 
        //     p.PIN_24, 
        //     p.PIN_29, 
        //     p.PIO1, 
        //     p.DMA_CH0)
        // .unwrap());
        //spawner.spawn(core1_consumer_task().unwrap());

        spawner.spawn(player::player_task(
           p.PIO0, p.DMA_CH11, p.PIN_27, p.PIN_28, p.PIN_3
        //    p.PIO0, p.DMA_CH11, p.PIN_23, p.PIN_25, p.PIN_24
        ).unwrap());
        //spawner.spawn(core1_consumer_task(p.PIN_2).unwrap());
    });

}
