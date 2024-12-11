use std::net::TcpListener;
use threadpool::ThreadPool;
use redis_starter_rust::handle_connection;

fn main() {
    // Handle concurrent connections
    let pool = ThreadPool::new(4);

    let listener = TcpListener::bind("127.0.0.1:6379")
        .expect("Failed to bind");

    println!("Server is running on port 6379");

    for stream in listener.incoming() {
        println!("Connection established");
        pool.execute(move ||{
            match stream {
                Ok(stream) => handle_connection(stream),
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        });
    }
}
