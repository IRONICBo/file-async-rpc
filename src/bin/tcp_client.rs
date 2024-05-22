use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Instant;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8990")?;
    let num_files = 10_000;
    let file_size = 4 * 1024 * 1024; // 4MB

    let file_content = vec![0u8; file_size];
    let mut buffer = vec![0u8; file_size];
    let start = Instant::now();

    for _ in 0..num_files {
        stream.write_all(&file_content)?;
        stream.read_exact(&mut buffer)?;
    }

    let duration = start.elapsed();
    println!("Time taken to send and receive responses for 10,000 files: {:?}, speed: {:?}MB/s", duration.as_secs_f64(), ((num_files * 4) as f64) / duration.as_secs_f64());

    Ok(())
}
