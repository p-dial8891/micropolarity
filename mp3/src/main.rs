//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to specified Wifi network and creates a TCP endpoint on port 1234.

#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::str::from_utf8;
use core::{mem, mem::MaybeUninit};

use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_sync::pipe::{Pipe, Reader, Writer};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0, DMA_CH1, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_rp::{bind_interrupts, dma};
use embassy_time::{Duration, Timer, Instant};
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use nanomp3::Decoder;

// For USB
use embassy_rp::{peripherals::USB, usb};

// Bring your own wifi credentials and store it in auth.rs
// pub const WIFI_NETWORK: &str = "AAAAAAA"; // change to your network SSID
// pub const WIFI_PASSWORD: &str = "XXXXXXXX"; // change to your network password
mod auth;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;    
});

const SAMPLE_RATE: u32 = 16000;
const BIT_DEPTH: u32 = 16;
const READ_SIZE: usize = 32768*2;

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

async fn player_task(
    pio : embassy_rp::Peri<'static, embassy_rp::peripherals::PIO1>,
    dma : embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH1>,
    bit_clock_pin : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_27>,
    left_right_clock_pin : embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_28>,
    data_pin :  embassy_rp::Peri<'static, embassy_rp::peripherals::PIN_3>,
    mut socket: TcpSocket<'_>
) {
    // Setup pio state machine for i2s output
    let Pio { mut common, sm0, .. } = Pio::new(pio, Irqs);

    let program = PioI2sOutProgram::new(&mut common);
    let mut i2s = PioI2sOut::new(
        &mut common,
        sm0,
        dma,
        Irqs,
        data_pin,
        bit_clock_pin,
        left_right_clock_pin,
        SAMPLE_RATE,
        BIT_DEPTH,
        &program,
    );
    i2s.start();
    let mut read_buf : [u8;READ_SIZE] = [0u8;READ_SIZE];
    let mut used : usize = 0;   // bytes ( multiples of 4 )
    let mut read : usize = 0;
    let mut decoder = Decoder::new();
    let mut frame_info : Option<nanomp3::FrameInfo> = None;
    let mut decoded : usize = 0;
    let mut samples : usize = 0;
    let mut profile_start = 0;
    let mut profile_end = 0;

    const BUFFER_SIZE : usize = 218*1024; // bytes
    const THRESHOLD : usize = (BUFFER_SIZE / 2) - (nanomp3::MAX_SAMPLES_PER_FRAME*4); // bytes
    static READ_BUF_POOL: StaticCell<[MaybeUninit<u8>;BUFFER_SIZE]> = StaticCell::new();
    let mut buffer_static_unaligned = READ_BUF_POOL.init_with(|| [MaybeUninit::zeroed(); BUFFER_SIZE] );
    let (prefix, mut buffer_static, suffix) = unsafe { buffer_static_unaligned.align_to_mut::<MaybeUninit<f32>>() };
    let (mut front_buffer, mut back_buffer) = buffer_static.split_at_mut(BUFFER_SIZE/(2*4));

    log::info!("Player started with {}, {}, {}, {}, {} samples/frame ...", 
        prefix.len(), suffix.len(), front_buffer.len(), back_buffer.len(), nanomp3::MAX_SAMPLES_PER_FRAME);

    while used < ( front_buffer.len()*4 - (nanomp3::MAX_SAMPLES_PER_FRAME*4) ) {
        // READ MP3 DATA
        profile_start = Instant::now().as_millis();
        read = (read-decoded) + match socket.read(&mut read_buf[(read-decoded)..]).await {
            Ok(0) => {
                log::warn!("read EOF");
                return;
            }
            Ok(n) => { log::warn!("read {} bytes, used {} bytes", n, used); n },
            Err(e) => {
                log::warn!("read error: {:?}", e);
                return;
            }
        };
        
        // DECODE MP3 DATA
        let mut buffer_initialised : &mut [f32] = unsafe { mem::transmute(&mut *front_buffer) };
        let (decoded_t, frame_info_t) = decoder.decode(&read_buf[..read],&mut buffer_initialised[(used/4)..]);
        frame_info = frame_info_t; decoded = decoded_t;
        if let Some(f) = frame_info {
            samples = f.samples_produced;
        } else {
            samples = 0;
        }
        let mut i = 0;
        for mut s in &mut buffer_initialised[(used/4)..((used/4)+samples)] {
            let mut f = *s;
            let s_scaled = f * 32767f32;
            let s_scaled_floor = s_scaled as i16;
            let s_scaled_floor_udword = ( s_scaled_floor as u16 as u32 ) * 0x10001;
            let mut pcm : &mut u32 = unsafe { mem::transmute(s) };
            *pcm = s_scaled_floor_udword;
            i += 1;
        }
        used += (i*4);
        read_buf.copy_within(decoded..read, 0);
    }

    loop{
        // PLAY SAMPLES
        let dma_buffer : &mut [u32] = unsafe { mem::transmute(&mut *front_buffer) };
        profile_end = Instant::now().as_millis();
        log::info!("Playing {} bytes @ {:.3}Kbps with {} bytes queued", used,
            (used as f32)/((profile_end - profile_start) as f32), socket.recv_queue(),);
        if let Some(f) = frame_info {
            log::info!("{:?} with {decoded} bytes decoded and {} bytes buffered",f, read);
        } else {
            log::info!("No decoder info.");
        }
        let dma_future = i2s.write(&dma_buffer[0..(used/4)]);

        profile_start = Instant::now().as_millis();

        used = 0;
        // let mut retry_count = 5;

        while used < ( back_buffer.len()*4 - (nanomp3::MAX_SAMPLES_PER_FRAME*4) ) {
            // READ MP3 DATA
            read = (read-decoded) + match socket.read(&mut read_buf[(read-decoded)..]).await {
                Ok(0) => {
                    log::warn!("read EOF");
                    return;
                }
                Ok(n) => { log::warn!("read {} bytes, used {} bytes", n, used); n },
                Err(e) => {
                    log::warn!("read error: {:?}", e);
                    return;
                }
            };

            // DECODE MP3 DATA
            let mut buffer_initialised : &mut [f32] = unsafe { mem::transmute(&mut *back_buffer) };
            let (decoded_t, frame_t) = decoder.decode(&read_buf[..read],&mut buffer_initialised[(used/4)..]);
            frame_info = frame_t; decoded = decoded_t;
            if let Some(f) = frame_info {
                samples = f.samples_produced;
            } else {
                samples = 0;
            }
            let mut i = 0;
            for mut s in &mut buffer_initialised[(used/4)..((used/4)+samples)] {
                let mut f = *s;
                let s_scaled = f * 32767f32;
                let s_scaled_floor = s_scaled as i16;
                let s_scaled_floor_udword = ( s_scaled_floor as u16 as u32 ) * 0x10001;
                let mut pcm : &mut u32 = unsafe { mem::transmute(s) };
                *pcm = s_scaled_floor_udword;
                i += 1;
            }
            used += (i*4);
            read_buf.copy_within(decoded..read, 0);
        }

        dma_future.await;
        mem::swap(&mut back_buffer, &mut front_buffer);

    }

}

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    let p = embassy_rp::init(Default::default());
    spawner.must_spawn(logger_task(p.USB));
    log::info!("Hello World!");
    
    let mut rng = RoscRng;

    let fw = aligned_bytes!("../../embassy/cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../../embassy/cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../../embassy/cyw43-firmware/nvram_rp2040.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner));

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

    spawner.spawn(net_task(runner));

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

    let mut rx_buffer = [0; READ_SIZE*2];
    let mut tx_buffer = [0; READ_SIZE/4];

    log::info!("Setting up player at {}Hz", SAMPLE_RATE);

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    control.gpio_set(0, false).await;
    log::info!("Listening on TCP:1234...");
    if let Err(e) = socket.accept(1234).await {
        log::warn!("accept error: {:?}", e);
        return;
    }

    log::info!("Received connection at {:?}", socket.local_endpoint());
    control.gpio_set(0, true).await;

    player_task(p.PIO1, p.DMA_CH1, p.PIN_27, p.PIN_28, p.PIN_3 ,socket).await;

    loop {
        Timer::after_millis(500).await;
    }

}
