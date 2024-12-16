use tokio::net::TcpListener;
use tokio::signal;
use crate::handle_connection;
pub struct Server {
    listener: TcpListener,
    address: String,
    port: u16,
}

impl Server {
    pub async fn new(address: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", address, port)).await?;
        Ok(Self { listener, address: address.to_string(), port })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((socket, _)) => {
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(socket).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Accept error: {}", e);
                        }
                    }
                }
                _ = signal::ctrl_c() => {
                    println!("Ctrl+C pressed, shutting down...");
                    break;
                }
            }
        }

        Ok(())
    }
}