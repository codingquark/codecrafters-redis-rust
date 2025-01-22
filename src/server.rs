use tokio::net::TcpListener;
use tokio::signal;
use std::sync::Arc;
use crate::{handle_connection, store::redis::Store, store::datatype::DataType};

pub struct Server {
    listener: TcpListener,
    // address: String,
    // port: u16,
    store: Arc<Store>,
}

impl Server {
    pub async fn new(address: String, port: u16, dir: String, dbfilename: String) -> Result<Self, Box<dyn std::error::Error>> {
        let server = Self {
            listener: TcpListener::bind(format!("{}:{}", address, port)).await?,
            // address,
            // port,
            store: Arc::new(Store::new().await?),
        };
        
        // Initialize the database
        Self::init_db(&server.store, dir, dbfilename).await?;

        Ok(server)
    }

    async fn init_db(store: &Arc<Store>, dir: String, dbfilename: String) -> Result<(), Box<dyn std::error::Error>> {
        // Store config in RAM
        store.set("dir", DataType::String(dir)).await?;
        store.set("dbfilename", DataType::String(dbfilename)).await?;

        Ok(())
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((socket, _)) => {
                            let store = Arc::clone(&self.store);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(socket, &store).await {
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