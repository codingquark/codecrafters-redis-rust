use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod parser;
pub mod error;
pub mod config;
pub mod server;
pub mod store;

use parser::RESPOutput;
use error::{RedisError, Result};
use store::Store;

use crate::parser::Parser;

pub async fn handle_connection(mut stream: TcpStream, store: &mut Store) -> Result<()> {
    let mut buffer = [0; 512];

    loop {
        let size = stream.read(&mut buffer).await?;
        if size == 0 {
            return Ok(());
        }

        let command = Parser::parse(&buffer[..size])
            .map_err(|e| RedisError::Parser(e))
            .and_then(|(output, _)| Command::from_resp(output))?;

        let response = command.execute(store)?;
        stream.write_all(response.as_bytes()).await?;
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo(String),
    Set(String, String),
    Get(String),
}

impl Command {
    pub fn from_resp(resp: RESPOutput) -> Result<Self> {
        match resp {
            RESPOutput::Array(elements) => Self::parse_command(elements),
            _ => Err(RedisError::InvalidArguments),
        }
    }

    fn parse_command(elements: Vec<RESPOutput>) -> Result<Self> {
        let (command, args) = elements.split_first()
            .ok_or(RedisError::InvalidArguments)?;

        match command {
            RESPOutput::BulkString(cmd) => match cmd.to_uppercase().as_str() {
                "PING" => Ok(Command::Ping),
                "ECHO" => {
                    let arg = args.first()
                        .and_then(|arg| match arg {
                            RESPOutput::BulkString(s) => Some(s.clone()),
                            _ => None,
                        })
                        .ok_or(RedisError::InvalidArguments)?;
                    Ok(Command::Echo(arg))
                }
                "SET" => {
                    let key = args.first()
                        .and_then(|arg| match arg {
                            RESPOutput::BulkString(s) => Some(s.clone()),
                            _ => None,
                        })
                        .ok_or(RedisError::InvalidArguments)?;
                    let value = args.get(1)
                        .and_then(|arg| match arg {
                            RESPOutput::BulkString(s) => Some(s.clone()),
                            _ => None,
                        })
                        .ok_or(RedisError::InvalidArguments)?;
                    Ok(Command::Set(key, value))
                }
                "GET" => {
                    let key = args.first()
                        .and_then(|arg| match arg {
                            RESPOutput::BulkString(s) => Some(s.clone()),
                            _ => None,
                        })
                        .ok_or(RedisError::InvalidArguments)?;
                    Ok(Command::Get(key))
                }
                _ => Err(RedisError::UnknownCommand),
            },
            _ => Err(RedisError::InvalidArguments),
        }
    }

    pub fn execute(&self, store: &mut Store) -> Result<String> {
        Ok(match self {
            Command::Ping => "+PONG\r\n".to_string(),
            Command::Echo(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Command::Set(key, value) => {
                store.set(key, value)?;
                "+OK\r\n".to_string()
            }
            Command::Get(key) => {
                match store.get(key)? {
                    Some(value) => format!("${}\r\n{}\r\n", value.len(), value),
                    None => "$-1\r\n".to_string(),
                }
            }
        })
    }
}
