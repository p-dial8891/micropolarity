use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::net::TcpStream;
use std::thread::{sleep};
use std::time::{Duration, Instant};
use std::io::Write;

fn main() {
	let mut fp = String::from("../audacity/");
    fp.extend([std::env::args().nth(1).unwrap()]);
    let mut input = File::open(fp).unwrap();

    let mut stream = TcpStream::connect("192.168.1.100:1234").unwrap();
    let mut buffer = [0u8; 90112/(2)];

    let mut retry_count = 5;
    let mut profile_count = 4;
    let mut offset = 0;
    let mut written = 0;
    let mut written_sum = 0;
    let mut profile_duration = 0;
    let mut profile_start = Instant::now();
    let mut read = input.read(&mut buffer).unwrap();

    println!("TCP Stream read timmeout : {:?} and write timeout : {:?}", 
        stream.read_timeout(), stream.write_timeout());

    while read > 0 {
        written = match stream.write(&buffer[..(read + offset)]) {
            Ok(n) => { retry_count = 5; n},
            Err(e) => {
                println!("Error {:?}", e);
                if retry_count > 0 {
                    println!("Retrying {}/5", 5 - retry_count + 1);
                    retry_count -= 1;
                    continue;
                } else {
                    break;
                }
            }
        };
        written_sum += written;
        //sleep(Duration::from_millis(5632/(2*2*2*2)));
        buffer.copy_within(written..read,0);
        offset = read - written;
        if profile_count == 0 {
            profile_duration = Instant::now().duration_since(profile_start).as_millis();
            println!("Read : {} , Written : {} , Offset : {} @ {:.3}Kbps", read, written, offset,
                (written_sum as f32)*2f32/(profile_duration as f32));
            profile_count = 4;
            written_sum = 0;
            profile_duration = Duration::ZERO.as_millis();
            profile_start = Instant::now();
        }
        read = input.read(&mut buffer[offset..]).unwrap();
        profile_count -= 1;
    }

}
