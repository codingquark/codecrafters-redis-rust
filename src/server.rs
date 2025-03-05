use tokio::net::TcpListener;
use tokio::signal;
use std::sync::Arc;
use crate::{handle_connection, store::redis::Store, store::datatype::DataType};
use crate::config::AppConfig;
use crate::parser::RDBParser;
use std::fs::File;
use std::io;

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
        // Try to open the RDB file, if it doesn't exist, that's fine
        let rdb_file = match File::open(self.config.dbfilename.clone()) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                println!("No RDB file found, starting with empty database");
                return Ok(());
            }
            Err(e) => return Err(Box::new(e)),
        };

        println!("Loading RDB file: {}", self.config.dbfilename);
        let mut rdb_parser = RDBParser::new(rdb_file);
        
        // Parse the RDB header
        rdb_parser.parse_header()?;

        // Parse and load entries
        let mut entry_count = 0;
        while let Some(entry) = rdb_parser.parse_entry()? {
            entry_count += 1;
            let key = String::from_utf8_lossy(&entry.key).to_string();
            
            match entry.value {
                crate::parser::rdb::RDBValue::String(data) => {
                    let value = String::from_utf8_lossy(&data).to_string();
                    self.store.set(&key, DataType::String(value)).await?;
                    
                    // If there's an expiry, set it
                    if let Some(_expiry) = entry.expiry {
                        // TODO: Implement expiry handling
                    }
                }
            }
        }

        println!("RDB file loaded successfully, loaded {} entries", entry_count);
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