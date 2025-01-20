use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod parser;
pub mod error;
pub mod config;
pub mod server;
pub mod store;

use parser::RESPOutput;
use error::{RedisError, Result};
use store::redis::Store;
use store::datatype::DataType;

use crate::parser::Parser;

pub async fn handle_connection(mut stream: TcpStream, store: &Store) -> Result<()> {
    let mut buffer = [0; 512];

    loop {
        let size = stream.read(&mut buffer).await?;
        if size == 0 {
            return Ok(());
        }

        let command = Parser::parse(&buffer[..size])
            .map_err(|e| RedisError::Parser(e))
            .and_then(|(output, _)| Command::from_resp(output))?;

        let response = command.execute(store).await?;
        stream.write_all(response.as_bytes()).await?;
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo(String),
    Get(String),
    Set(String, DataType),
}

impl Command {
    pub fn from_resp(resp: RESPOutput) -> Result<Self> {
        match resp {
            RESPOutput::Array(elements) => Self::parse_command(elements),
            RESPOutput::SimpleString(s) => Ok(Command::Echo(s)),
            RESPOutput::Error(_) => Err(RedisError::InvalidArguments),
            RESPOutput::Integer(i) => Ok(Command::Get(i.to_string())),
            RESPOutput::Double(d) => Ok(Command::Get(d.to_string())),
            RESPOutput::Boolean(b) => Ok(Command::Get(b.to_string())),
            RESPOutput::Null => Err(RedisError::InvalidArguments),
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
                            RESPOutput::BulkString(s) => Some(DataType::String(s.clone())),
                            RESPOutput::Integer(i) => Some(DataType::Integer(*i)),
                            RESPOutput::Double(d) => Some(DataType::Double(*d)),
                            RESPOutput::Boolean(b) => Some(DataType::Boolean(*b)),
                            RESPOutput::Null => Some(DataType::Null),
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

    pub async fn execute(&self, store: &Store) -> Result<String> {
        Ok(match self {
            Command::Ping => "+PONG\r\n".to_string(),
            Command::Echo(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Command::Set(key, value) => {
                store.set(key, value.clone()).await?;
                "+OK\r\n".to_string()
            }
            Command::Get(key) => {
                match store.get(key).await? {
                    Some(value) => match value {
                        DataType::String(s) => format!("${}\r\n{}\r\n", s.len(), s),
                        DataType::Integer(i) => format!(":{}\r\n", i),
                        DataType::Double(d) => format!(",{}\r\n", d),
                        DataType::Boolean(b) => format!("#{}\r\n", if b { "t" } else { "f" }),
                        DataType::Null => "$-1\r\n".to_string(),
                    },
                    None => "$-1\r\n".to_string(),
                }
            }
        })
    }
}
