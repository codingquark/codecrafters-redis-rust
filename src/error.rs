use std::io;
use thiserror::Error;

use crate::parser::ParserError;
#[derive(Error, Debug)]
pub enum RedisError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),

    #[error("Unknown command")]
    UnknownCommand,
    
    #[error("Invalid command arguments")]
    InvalidArguments,
}

pub type Result<T> = std::result::Result<T, RedisError>; 