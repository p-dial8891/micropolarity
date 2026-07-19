#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
// For USB
use embassy_rp::{peripherals::USB, usb};
use embassy_rp::{bind_interrupts, dma};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH11, PIO0, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt::{self, InterruptExt};
use embassy_rp::interrupt::typelevel::{Binding, Handler};
use critical_section::Mutex;
use core::cell::RefCell;
use core::task::{Poll};

use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, StackResources, Stack};
use embassy_rp::clocks::RoscRng;
use cyw43::{aligned_bytes, SpiBus, JoinOptions};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};

mod ring_buffer;
mod messaging;
mod player;
mod auth;

// Use the absolute scratchpad memory window configured in memory.x
pub const HANDSHAKE_ADDR: *mut u32 = 0x2008_0000 as *mut u32;
const READ_SIZE : usize = 100;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH11>, dma::InterruptHandler<DMA_CH0>;
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

pub struct wifi_per {
    p_23 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_23>,
    p_25 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_25>,
    p_24 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_24>,
    p_29 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_29>,    
    pio1 : embassy_rp::Peri<'static, embassy_rp::peripherals::PIO1>,
    dma : embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH0>
}

pub struct player_per {
    pio : embassy_rp::Peri<'static, embassy_rp::peripherals::PIO0>,
    dma : embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH11>,
    bit_clock_pin : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_27>,
    left_right_clock_pin : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_28>,
    data_pin :  embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_3>,
}

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO1, 0>>>) -> ! {
    runner.run().await
}

//#[embassy_executor::main(executor = "embassy_rp::executor::Executor",  entry = "cortex_m_rt::entry")]
#[embassy_executor::task]
async fn core1_main_loop(
    spawner: embassy_executor::Spawner,
    wifi_p : wifi_per,
    player_p : player_per
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

    let pwr = Output::new(wifi_p.p_23, Level::Low);
    let cs = Output::new(wifi_p.p_25, Level::High);
    let mut pio = Pio::new(wifi_p.pio1, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
        // See: https://github.com/embassy-rs/embassy/issues/3960.
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        wifi_p.p_24,
        wifi_p.p_29,
        dma::Channel::new(wifi_p.dma, Irqs),
    );
    
    let mut rng = RoscRng;
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) : 
        (cyw43::NetDriver<'_>, cyw43::Control<'_>, cyw43::Runner<'_, SpiBus<Output<'_>, _>>) = 
            cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    spawner.spawn(net_task(runner).unwrap());

    while let Err(err) = control
        .join(auth::WIFI_NETWORK, JoinOptions::new(auth::WIFI_PASSWORD.as_bytes()))
        .await
    {
        log::info!("join failed: {:?}", err);
    }

    log::info!("waiting for link...");
    stack.wait_link_up().await;

    log::info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // And now we can use it!
    log::info!("Stack is up!");
    log::info!("Network address is {:x?}", stack.hardware_address());

    let mut rx_buffer = [0; READ_SIZE];
    let mut tx_buffer = [0; READ_SIZE];

    log::info!("Setting up player");
    
    player::player_task(player_p, stack, &mut rx_buffer, &mut tx_buffer).await;

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
        spawner.spawn(core1_main_loop(
            spawner,
            wifi_per { 
                p_23 : p.PIN_23, p_25 : p.PIN_25, p_24 : p.PIN_24, 
                p_29 : p.PIN_29, pio1 : p.PIO1, dma : p.DMA_CH0 
            },
            player_per {
                pio : p.PIO0, dma : p.DMA_CH11, 
                bit_clock_pin : p.PIN_27, left_right_clock_pin : p.PIN_28, 
                data_pin : p.PIN_3
            }
        )
        .unwrap());
        //spawner.spawn(core1_consumer_task().unwrap());
 
        // spawner.spawn(player::player_task(
        //    p.PIO0, p.DMA_CH11, p.PIN_27, p.PIN_28, p.PIN_3
        // ).unwrap());
        //spawner.spawn(core1_consumer_task(p.PIN_2).unwrap());
    });

}
