use tokio::net::TcpListener;
use tokio::signal;
use std::sync::Arc;
use crate::{handle_connection, store::redis::Store, store::datatype::DataType};
use crate::config::AppConfig;

pub struct Server {
    listener: TcpListener,
    store: Arc<Store>,
    config: AppConfig,
}

impl Server {
    pub async fn new(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let server = Self {
            listener: TcpListener::bind(format!("{}:{}", config.server.address, config.server.port)).await?,
            store: Arc::new(Store::new().await?),
            config,
        };

        Ok(server)
    }

    async fn init_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize config
        self.store.set("dir", DataType::String(self.config.dir.clone())).await?;
        self.store.set("dbfilename", DataType::String(self.config.dbfilename.clone())).await?;

        Ok(())
    }

    async fn init_db(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Initialize the database
        Ok(())
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize the database
        Self::init_config(&self).await?;
        Self::init_db(&self).await?;

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