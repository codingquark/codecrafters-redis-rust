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
use std::time::Duration;
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
    Set(String, DataType, Option<Duration>),
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

                    let expiry = Self::parse_expiry(&args)?;
                    Ok(Command::Set(key, value, expiry))
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

    fn parse_expiry(args: &[RESPOutput]) -> Result<Option<Duration>> {
        if args.len() <= 3 {
            return Ok(None);
        }

        let [opt, duration] = match (args.get(2), args.get(3)) {
            (Some(RESPOutput::BulkString(opt)), Some(RESPOutput::BulkString(duration))) => [opt, duration],
            _ => return Err(RedisError::InvalidArguments),
        };

        let duration_val = duration.parse::<u64>()
            .map_err(|_| RedisError::InvalidArguments)?;

        match opt.to_uppercase().as_str() {
            "EX" => Ok(Some(Duration::from_secs(duration_val))),
            "PX" => Ok(Some(Duration::from_millis(duration_val))),
            _ => Ok(None),
        }
    }

    pub async fn execute(&self, store: &Store) -> Result<String> {
        Ok(match self {
            Command::Ping => "+PONG\r\n".to_string(),
            Command::Echo(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Command::Set(key, value, expiry) => {
                match expiry {
                    Some(duration) => store.set_ex(key, value.clone(), *duration).await?,
                    None => store.set(key, value.clone()).await?,
                }
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
