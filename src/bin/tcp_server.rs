use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut buffer = vec![0; 4 * 1024 * 1024]; // 4MB buffer
    while let Ok(_) = stream.read_exact(&mut buffer) {
        // Echo the data back
        if stream.write_all(&buffer).is_err() {
            break;
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8990").expect("Could not bind");
    println!("Server listening on port 8990");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Failed: {}", e);
            }
        }
    }
}
