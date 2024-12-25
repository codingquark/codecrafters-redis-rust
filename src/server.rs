use tokio::net::TcpListener;
use tokio::signal;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{handle_connection, store::Store};

pub struct Server {
    listener: TcpListener,
    address: String,
    port: u16,
    store: Arc<Mutex<Store>>,
}

impl Server {
    pub async fn new(address: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", address, port)).await?;
        let store = Arc::new(Mutex::new(Store::new()));
        Ok(Self { 
            listener, 
            address: address.to_string(), 
            port,
            store,
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((socket, _)) => {
                            let store = Arc::clone(&self.store);
                            tokio::spawn(async move {
                                let mut store = store.lock().await;
                                if let Err(e) = handle_connection(socket, &mut store).await {
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