use redis_starter_rust::{config::load_config, server::Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let server = Server::new(config.server.address, 
        config.server.port, 
        config.dir.unwrap(), 
        config.dbfilename.unwrap()).await?;
    server.start().await?;

    Ok(())
}
