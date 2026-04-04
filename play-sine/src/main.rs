//! This example shows generating audio and sending it to a connected i2s DAC using the PIO
//! module of the RP235x.
//!
//! Connect the i2s DAC as follows:
//!   bclk : GPIO 27
//!   lrc  : GPIO 28
//!   din  : GPIO 3
//! Then short GPIO 2 to GND to trigger a rising triangle waveform.

#![no_std]
#![no_main]

mod sine;

use core::mem;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_rp::{bind_interrupts, dma};
use embassy_futures::select::{select, Either};
use embassy_time::Timer;
// use embassy_sync::channel::{Channel, Sender, Receiver};
// use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

// For USB
use embassy_rp::{peripherals::USB, usb};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const SAMPLE_RATE: u32 = 48_000;
const BIT_DEPTH: u32 = 16;

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);

    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

// #[embassy_executor::task]
// async fn front_task(
//     mut i2s : PioI2sOut<'static, embassy_rp::peripherals::PIO0,0>, 
//     buf: &'static [u32],
//     sender: Sender<'static, NoopRawMutex, u32, 1>
// ) {
//     i2s.write(buf).await;
//     sender.send(1).await;
// }

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    _spawner.must_spawn(logger_task(p.USB));
    
    // Setup pio state machine for i2s output
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);

    let bit_clock_pin = p.PIN_27;
    let left_right_clock_pin = p.PIN_28;
    let data_pin = p.PIN_3;

    let program = PioI2sOutProgram::new(&mut common);
    let mut i2s = PioI2sOut::new(
        &mut common,
        sm0,
        p.DMA_CH0,
        Irqs,
        data_pin,
        bit_clock_pin,
        left_right_clock_pin,
        SAMPLE_RATE,
        BIT_DEPTH,
        &program,
    );
    i2s.start();
    let input = Input::new(p.PIN_2, Pull::Up);
/*

    // create two audio buffers (back and front) which will take turns being
    // filled with new audio data and being sent to the pio fifo using dma
    const BUFFER_SIZE: usize = 960;
    static DMA_BUFFER: StaticCell<[u32; BUFFER_SIZE * 2]> = StaticCell::new();
    let dma_buffer = DMA_BUFFER.init_with(|| [0u32; BUFFER_SIZE * 2]);
    let (mut back_buffer, mut front_buffer) = dma_buffer.split_at_mut(BUFFER_SIZE);

    // start pio state machine
    let mut fade_value: i32 = 0;
    let mut phase: i32 = 0;

    loop {
        // trigger transfer of front buffer data to the pio fifo
        // but don't await the returned future, yet
        let dma_future = i2s.write(front_buffer);

        // fade in audio when GPIO 0 pin is shorted to GND
        let fade_target = if fade_input.is_low() { 
            log::info!("Key pressed ...");
            i32::MAX 
        } else { 0 };

        // fill back buffer with fresh audio samples before awaiting the dma future
        for s in back_buffer.iter_mut() {
            // exponential approach of fade_value => fade_target
            fade_value += (fade_target - fade_value) >> 14;
            // generate triangle wave with amplitude and frequency based on fade value
            phase = (phase + (fade_value >> 22)) & 0xffff;
            let triangle_sample = (phase as i16 as i32).abs() - 16384;
            let sample = (triangle_sample * (fade_value >> 15)) >> 16;
            // duplicate mono sample into lower and upper half of dma word
            *s = (sample as u16 as u32) * 0x10001;
        }
        // now await the dma future. once the dma finishes, the next buffer needs to be queued
        // within DMA_DEPTH / SAMPLE_RATE = 8 / 48000 seconds = 166us
        dma_future.await;
        mem::swap(&mut back_buffer, &mut front_buffer);
    }
*/
    const BUFFER_SIZE: usize = 438;
    static DMA_BUFFER: StaticCell<[u32; BUFFER_SIZE * 2]> = StaticCell::new();
    let dma_buffer = DMA_BUFFER.init_with(|| {
        let mut a = [0u32; BUFFER_SIZE * 2];
        for i in 0..BUFFER_SIZE {
            a[i] = sine::SINE_440_48KHZ[i];
        }
        for i in BUFFER_SIZE..(BUFFER_SIZE*2) {
            a[i] = sine::SINE_440_48KHZ[i-BUFFER_SIZE];
        }
        a
    });
    let (mut back_buffer, mut front_buffer) = dma_buffer.split_at_mut(BUFFER_SIZE);

    loop {
        match select(
            async {
                while input.is_low() { 
                    Timer::after_millis(2000).await;
                }
                let dma_future = i2s.write(front_buffer);
                dma_future.await;
                mem::swap(&mut back_buffer, &mut front_buffer);                
            },
            async {
                Timer::after_millis(2000).await;
            }
        ).await {
            Either::First(_) => { },
            Either::Second(_) => {
                log::info!("Silence ...");
            }
        }
    }
}
