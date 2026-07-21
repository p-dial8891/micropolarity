use core::{mem, mem::MaybeUninit};
use embassy_rp::peripherals::{DMA_CH1, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_rp::{bind_interrupts, dma};
use embassy_time::{Duration, Timer, Instant};
// use embedded_io_async::Write;
use embedded_io_async::{Read, ErrorType, ErrorKind};
use static_cell::StaticCell;
// use {defmt_rtt as _, panic_probe as _};
use crate::ring_buffer::Buffer;
use crate::messaging::{SharedMessage, MessageId};
use nanomp3::Decoder;
use crate::{HANDSHAKE_ADDR, Irqs};
use embassy_net::{Stack, tcp::TcpSocket};

const SAMPLE_RATE: u32 = 44100;
const BIT_DEPTH: u32 = 16;
const READ_SIZE: usize = 32768;

struct FileReader {
    pub rb : Buffer
}

impl Read for FileReader {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut bytes_popped = 0;
        //log::info!("Sending request.");
        SharedMessage::send_message(MessageId::GET_AUDIO, &self.rb, None).await;
        while let m = SharedMessage::receive_message(&self.rb, buf).await {
            if m.0 == MessageId::NOOP {
                Timer::after_millis(5).await;
                continue;
            } else {
                bytes_popped = m.1;
                break;
            }
        }
        log::info!("{} bytes popped from core 0", bytes_popped);
        Timer::after_millis(5).await;
        Ok(bytes_popped)
    }
}

impl ErrorType for FileReader {
    type Error = ErrorKind;
}

async fn select_next_track(
    stack : embassy_net::Stack<'_>,
    rx_buffer : &mut [u8],
    tx_buffer : &mut [u8]
) {
    
    let mut buffer = [0u8; 100];
    let mut read = 0usize;
    let rb = Buffer::new();
    let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);

    socket.set_timeout(Some(Duration::from_secs(10)));
    log::info!("Listening on TCP:1234...");
    if let Err(e) = socket.accept(1234).await {
        log::warn!("accept error: {:?}", e);
        return;
    }
    log::info!("Received connection at {:?}", socket.local_endpoint());
    log::info!("Reading socket.");
    loop {
        read += socket.read(&mut buffer[read..]).await.unwrap();
        if let Some(n) = (&buffer[..read]).iter().position(|x| { *x == '\r' as u8 }) {
            let mut rxbuf = [0;0usize];
            read = 0;
            log::info!("CR found at {}", n);
            loop {
                SharedMessage::send_message(MessageId::PLAY_FILE, &rb, Some(&buffer[..n])).await;
                Timer::after_millis(1000).await;
                if let m = SharedMessage::receive_message(&rb, &mut rxbuf).await {
                    if m.0 != MessageId::PLAY_FILE {
                        continue;
                    } else {
                        return;
                    }
                }
            }
            buffer.as_mut_slice().fill(0);
        } else {
            Timer::after_millis(100).await;
        }
    }
}

pub async fn player_task(
    player_p : crate::player_per,
    stack : embassy_net::Stack<'_>,
    rx_buffer : &mut [u8],
    tx_buffer : &mut [u8]
) {

    // Setup pio state machine for i2s output
    let Pio { mut common, sm0, .. } = Pio::new(player_p.pio, Irqs);

    let program = PioI2sOutProgram::new(&mut common);
    let mut i2s = PioI2sOut::new(
        &mut common,
        sm0,
        player_p.dma,
        Irqs,
        player_p.data_pin,
        player_p.bit_clock_pin,
        player_p.left_right_clock_pin,
        SAMPLE_RATE,
        BIT_DEPTH,
        &program,
    );
    i2s.start();

    unsafe { core::ptr::write_volatile(HANDSHAKE_ADDR, 0x4EC07111); }
    
    let mut read_buf : [u8;READ_SIZE] = [0u8;READ_SIZE];
    let mut used : usize = 0;   // bytes ( multiples of 4 )
    let mut read : usize = 0;
    let mut decoder = Decoder::new();
    let mut frame_info : Option<nanomp3::FrameInfo> = None;
    let mut decoded : usize = 0;
    let mut samples : usize = 0;
    let mut stream_end = false;
    let mut profile_start = 0;
    let mut profile_end = 0;

    const BUFFER_SIZE : usize = 300*1024; // bytes
    static READ_BUF_POOL: StaticCell<[MaybeUninit<u8>;BUFFER_SIZE]> = StaticCell::new();
    let mut buffer_static_unaligned = READ_BUF_POOL.init_with(|| [MaybeUninit::zeroed(); BUFFER_SIZE] );
    let (prefix, mut buffer_static, suffix) = unsafe { buffer_static_unaligned.align_to_mut::<MaybeUninit<f32>>() };
    let (mut front_buffer, mut back_buffer) = buffer_static.split_at_mut(BUFFER_SIZE/(2*4));
    let mut disk = FileReader{ rb : Buffer::new() };

    log::info!("Player started with {}, {}, {}, {}, {} samples/frame ...", 
        prefix.len(), suffix.len(), front_buffer.len(), back_buffer.len(), nanomp3::MAX_SAMPLES_PER_FRAME);

    'outer: loop {
        used = 0;
        log::info!("Selecting next track.");
        select_next_track(stack, rx_buffer, tx_buffer).await;

        while used < ( front_buffer.len()*4 - (nanomp3::MAX_SAMPLES_PER_FRAME*4) ) {
            // READ MP3 DATA
            profile_start = Instant::now().as_millis();
            read = (read-decoded) + match disk.read(&mut read_buf[(read-decoded)..]).await {
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
                if f.sample_rate != SAMPLE_RATE {
                    log::warn!("Incompatible sample rate! Exiting...");
                    return;
                }
            } else {
                samples = 0;
            }
            let mut i = 0;
            for mut s in &mut buffer_initialised[(used/4)..((used/4)+(samples*2))] {
                let mut f = *s;
                let s_scaled = f * 32767f32;
                let s_scaled_floor = s_scaled as i16;
                let s_scaled_floor_udword = ( s_scaled_floor as u16 as u32 ) ;
                let mut pcm : &mut u32 = unsafe { mem::transmute(s) };
                *pcm = s_scaled_floor_udword;
                i += 1;
            }
            used += (i*4);
            read_buf.copy_within(decoded..read, 0);
        }
        let mut dma_buffer : &mut [u32] = unsafe { mem::transmute(&mut *front_buffer) };
        //// Collect every 2nd sample
        for i in 0..(used/(4*2)) {
            dma_buffer[i] = ( dma_buffer[i*2] * 0x10000u32) | ( dma_buffer[(i*2)+1] & 0xFFFF ) ;
        }
        used /= 2;

        'inner: loop{
            // PLAY SAMPLES
            let mut dma_buffer : &mut [u32] = unsafe { mem::transmute(&mut *front_buffer) };
            let dma_future = i2s.write(&dma_buffer[0..(used/4)]);

            profile_start = Instant::now().as_millis();

            used = 0;

            while used < ( back_buffer.len()*4 - (nanomp3::MAX_SAMPLES_PER_FRAME*4) ) {
                // READ MP3 DATA
                read = (read-decoded) + match disk.read(&mut read_buf[(read-decoded)..]).await {
                    Ok(0) => {
                        log::warn!("read EOF");
                        stream_end = true;
                        0
                    }
                    Ok(n) => { log::warn!("read {} bytes, used {} bytes", n, used); n },
                    Err(e) => {
                        log::warn!("read error: {:?}", e);
                        continue 'outer; 
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
                for mut s in &mut buffer_initialised[(used/4)..((used/4)+(samples*2))] {
                    let mut f = *s;
                    let s_scaled = f * 32767f32;
                    let s_scaled_floor = s_scaled as i16;
                    let s_scaled_floor_udword = ( s_scaled_floor as u16 as u32 ) ;
                    let mut pcm : &mut u32 = unsafe { mem::transmute(s) };
                    *pcm = s_scaled_floor_udword;
                    i += 1;
                }
                used += (i*4);
                read_buf.copy_within(decoded..read, 0);
                if stream_end && decoded == 0 {
                    log::warn!("Ending playback.");
                    // stream_end = false;
                    // socket.write(&[1]).await.unwrap();
                    // socket.close();
                    // socket.flush().await.unwrap();
                    continue 'outer;
                }
            }
            let mut dma_buffer : &mut [u32] = unsafe { mem::transmute(&mut *back_buffer) };
            //// Collect every 2nd sample
            for i in 0..(used/(4*2)) {
                dma_buffer[i] = ( dma_buffer[i*2] * 0x10000u32) | ( dma_buffer[(i*2)+1] & 0xFFFF ) ;
            }
            used /= 2;
            profile_end = Instant::now().as_millis();        
            log::info!("Playing {} bytes @ {:.3}Kbps ", used, 
                (used as f32)/((profile_end - profile_start) as f32));
            if let Some(f) = frame_info {
                log::info!("{:?} with {decoded} bytes decoded and {} bytes buffered",f, read);
            } else {
                log::info!("No decoder info.");
            }
            dma_future.await;
            mem::swap(&mut back_buffer, &mut front_buffer);
        }
    }

}
