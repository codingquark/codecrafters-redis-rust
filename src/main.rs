use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        // Support multiple connections
        match stream {
            // If the stream is OK, respond to PINGs with PONG
            Ok(stream) => {
                thread::spawn(|| handle_connection(stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = stream
            .read(&mut buffer)
            .expect("Failed to read from stream");

        if bytes_read == 0 {
            break;
        }

        stream.write(b"+PONG\r\n").expect("Failed to write to stream");
    }
}
