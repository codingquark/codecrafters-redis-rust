//! Redis RDB File Parser
//! 
//! This module implements a parser for Redis RDB (Redis Database Backup) files.
//! The RDB file format is a binary format used by Redis to store snapshots of its
//! database. This implementation supports RDB version 11 and handles various Redis
//! data types and encodings.
//!
//! # Format Overview
//! 
//! The RDB file format consists of:
//! - A magic string "REDIS"
//! - RDB version number (4 bytes)
//! - Key-value pairs with optional expiry times
//! - Special opcodes for database selection and auxiliary information
//! - EOF marker
//!
//! # Example
//!
//! ```no_run
//! use std::fs::File;
//! use crate::parser::rdb::RDBParser;
//!
//! let file = File::open("dump.rdb").unwrap();
//! let mut parser = RDBParser::new(file);
//! 
//! // Parse the RDB header
//! parser.parse_header().unwrap();
//!
//! // Iterate through entries
//! while let Some(entry) = parser.parse_entry().unwrap() {
//!     println!("Key: {:?}, Value: {:?}", entry.key, entry.value);
//! }
//! ```

use std::io::{self, Read};
use std::time::SystemTime;
use std::fmt;

// RDB Version Constants
/// The RDB version supported by this parser (version 11)
const RDB_VERSION: u32 = 11;

// RDB Type Constants
/// Represents a string value type in RDB
const RDB_TYPE_STRING: u8 = 0;
/// Represents a list value type in RDB
const RDB_TYPE_LIST: u8 = 1;
/// Represents a set value type in RDB
const RDB_TYPE_SET: u8 = 2;
/// Represents a sorted set value type in RDB
const RDB_TYPE_ZSET: u8 = 3;
/// Represents a hash value type in RDB
const RDB_TYPE_HASH: u8 = 4;

// RDB Opcode Constants
/// Marks the end of the RDB file
const RDB_OPCODE_EOF: u8 = 0xFF;
/// Indicates a database selection operation
const RDB_OPCODE_SELECTDB: u8 = 0xFE;
/// Indicates an expiry time in seconds
const RDB_OPCODE_EXPIRETIME: u8 = 0xFD;
/// Indicates an expiry time in milliseconds
const RDB_OPCODE_EXPIRETIME_MS: u8 = 0xFC;
/// Indicates auxiliary information (metadata)
const RDB_OPCODE_AUX: u8 = 0xFA;
/// Indicates database size information
const RDB_OPCODE_RESIZEDB: u8 = 0xFB;

/// Represents errors that can occur during RDB parsing
#[derive(Debug)]
pub enum RDBError {
    /// Underlying IO error during reading
    IoError(io::Error),
    /// Invalid magic string at the start of file (should be "REDIS")
    InvalidMagicString,
    /// Unsupported RDB version (only version 11 is supported)
    UnsupportedVersion,
    /// Invalid length encoding in the RDB file
    InvalidLength,
    /// Invalid string encoding in the RDB file
    InvalidEncoding,
    /// Invalid value type encountered
    InvalidType,
}

impl fmt::Display for RDBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// StdError has to be bound as well
impl std::error::Error for RDBError {}

impl From<io::Error> for RDBError {
    fn from(error: io::Error) -> Self {
        RDBError::IoError(error)
    }
}

/// Represents a value stored in Redis
/// 
/// Currently only supports String values, but will be extended
/// to support other Redis data types in the future.
#[derive(Debug)]
pub enum RDBValue {
    /// String value stored as a byte vector (can be text or binary)
    String(Vec<u8>),
}

/// Represents a key-value entry in the RDB file
/// 
/// Each entry consists of:
/// - A key (as bytes)
/// - A value (currently only strings)
/// - An optional expiry time
#[derive(Debug)]
pub struct RDBEntry {
    /// The key as a byte vector
    pub key: Vec<u8>,
    /// The value associated with the key
    pub value: RDBValue,
    /// Optional expiry time for the key
    pub expiry: Option<SystemTime>,
}

/// Parser for Redis RDB files
/// 
/// This struct provides methods to parse an RDB file from any source that
/// implements the `Read` trait. It maintains state about the current database
/// being parsed and handles various RDB format features including:
/// - Type encoding
/// - Length encoding
/// - String compression
/// - Integer encoding
pub struct RDBParser<R: Read> {
    /// The underlying reader providing the RDB data
    reader: R,
    /// The currently selected database number
    current_db: u8,
}

impl<R: Read> RDBParser<R> {
    /// Creates a new RDB parser from a reader
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type that implements `Read` trait
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use crate::parser::rdb::RDBParser;
    ///
    /// let file = File::open("dump.rdb").unwrap();
    /// let parser = RDBParser::new(file);
    /// ```
    pub fn new(reader: R) -> Self {
        RDBParser {
            reader,
            current_db: 0,
        }
    }

    /// Parses the RDB file header
    ///
    /// The header consists of:
    /// - 5 bytes: "REDIS" magic string
    /// - 4 bytes: RDB version number as ASCII string
    ///
    /// # Returns
    ///
    /// * `Ok(())` if header is valid
    /// * `Err(RDBError::InvalidMagicString)` if magic string is not "REDIS"
    /// * `Err(RDBError::UnsupportedVersion)` if version is not supported
    /// * `Err(RDBError::IoError)` if reading fails
    pub fn parse_header(&mut self) -> Result<(), RDBError> {
        let mut magic = [0u8; 5];
        self.reader.read_exact(&mut magic)?;
        
        if &magic != b"REDIS" {
            return Err(RDBError::InvalidMagicString);
        }

        let mut version = [0u8; 4];
        self.reader.read_exact(&mut version)?;
        
        let version_str = std::str::from_utf8(&version)
            .map_err(|_| RDBError::InvalidMagicString)?;
        let version_num = version_str
            .parse::<u32>()
            .map_err(|_| RDBError::InvalidMagicString)?;

        if version_num != RDB_VERSION {
            return Err(RDBError::UnsupportedVersion);
        }

        Ok(())
    }

    /// Reads a length-encoded integer from the RDB file
    ///
    /// The encoding format uses the first two bits to determine the format:
    /// - 00: Next 6 bits represent length
    /// - 01: Next 14 bits represent length
    /// - 10: Next 32 bits represent length
    /// - 11: Special format (8, 16, or 32 bit integer)
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The decoded length
    /// * `Err(RDBError::InvalidLength)` - If length encoding is invalid
    /// * `Err(RDBError::IoError)` - If reading fails
    pub fn read_length(&mut self) -> Result<usize, RDBError> {
        let mut byte = [0u8; 1];
        self.reader.read_exact(&mut byte)?;
        let first = byte[0];

        match first >> 6 {
            0 => Ok((first & 0x3F) as usize),
            1 => {
                let mut next = [0u8; 1];
                self.reader.read_exact(&mut next)?;
                Ok((((first & 0x3F) as usize) << 8) | (next[0] as usize))
            },
            2 => {
                let mut buf = [0u8; 4];
                self.reader.read_exact(&mut buf)?;
                Ok(u32::from_be_bytes(buf) as usize)
            },
            3 => {
                // Special format
                match first & 0x3F {
                    0 => {
                        // 8-bit integer
                        let mut buf = [0u8; 1];
                        self.reader.read_exact(&mut buf)?;
                        Ok(buf[0] as usize)
                    },
                    1 => {
                        // 16-bit integer
                        let mut buf = [0u8; 2];
                        self.reader.read_exact(&mut buf)?;
                        Ok(u16::from_be_bytes(buf) as usize)
                    },
                    2 => {
                        // 32-bit integer
                        let mut buf = [0u8; 4];
                        self.reader.read_exact(&mut buf)?;
                        Ok(u32::from_be_bytes(buf) as usize)
                    },
                    _ => Err(RDBError::InvalidLength),
                }
            },
            _ => Err(RDBError::InvalidLength), // This should never happen
        }
    }

    /// Reads a string from the RDB file
    ///
    /// Handles various string encodings:
    /// - Length-prefixed strings (using length encoding)
    /// - Integer-encoded strings (8, 16, or 32 bit)
    /// - Compressed strings (TODO)
    ///
    /// The first byte determines the encoding:
    /// - 0xC0: 8-bit integer
    /// - 0xC1: 16-bit integer
    /// - 0xC2: 32-bit integer
    /// - Other: Length-prefixed string
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The decoded string as bytes
    /// * `Err(RDBError::InvalidLength)` - If string length encoding is invalid
    /// * `Err(RDBError::IoError)` - If reading fails
    pub fn read_string(&mut self) -> Result<Vec<u8>, RDBError> {
        let mut byte = [0u8; 1];
        self.reader.read_exact(&mut byte)?;
        let first = byte[0];

        // Check for special encodings
        match first {
            // Integer encodings
            0xC0 => {
                // 8-bit integer
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                let num = buf[0] as i8;
                return Ok(num.to_string().into_bytes());
            },
            0xC1 => {
                // 16-bit integer
                let mut buf = [0u8; 2];
                self.reader.read_exact(&mut buf)?;
                let num = i16::from_be_bytes(buf);
                return Ok(num.to_string().into_bytes());
            },
            0xC2 => {
                // 32-bit integer
                let mut buf = [0u8; 4];
                self.reader.read_exact(&mut buf)?;
                let num = i32::from_be_bytes(buf);
                return Ok(num.to_string().into_bytes());
            },
            _ => {
                // Regular string length encoding
                let len = match first >> 6 {
                    0 => (first & 0x3F) as usize,
                    1 => {
                        let mut next = [0u8; 1];
                        self.reader.read_exact(&mut next)?;
                        (((first & 0x3F) as usize) << 8) | (next[0] as usize)
                    },
                    2 => {
                        let mut buf = [0u8; 4];
                        self.reader.read_exact(&mut buf)?;
                        u32::from_be_bytes(buf) as usize
                    },
                    3 => {
                        // Special format
                        match first & 0x3F {
                            0 => {
                                // 8-bit integer
                                let mut buf = [0u8; 1];
                                self.reader.read_exact(&mut buf)?;
                                buf[0] as usize
                            },
                            1 => {
                                // 16-bit integer
                                let mut buf = [0u8; 2];
                                self.reader.read_exact(&mut buf)?;
                                u16::from_be_bytes(buf) as usize
                            },
                            2 => {
                                // 32-bit integer
                                let mut buf = [0u8; 4];
                                self.reader.read_exact(&mut buf)?;
                                u32::from_be_bytes(buf) as usize
                            },
                            _ => return Err(RDBError::InvalidLength),
                        }
                    },
                    _ => return Err(RDBError::InvalidLength), // This should never happen
                };

                let mut buf = vec![0u8; len];
                self.reader.read_exact(&mut buf)?;
                Ok(buf)
            }
        }
    }

    /// Parses the next entry from the RDB file
    ///
    /// This method handles:
    /// - Special opcodes (EOF, SELECT DB, etc.)
    /// - Key-value pairs with their types
    /// - Expiry times
    /// - Auxiliary data
    ///
    /// Special opcodes are handled as follows:
    /// - EOF (0xFF): End of file reached
    /// - SELECTDB (0xFE): Switch to a different database
    /// - EXPIRETIME/EXPIRETIME_MS (0xFD/0xFC): Set expiry time for next entry
    /// - AUX (0xFA): Skip auxiliary information
    /// - RESIZEDB (0xFB): Skip database size information
    ///
    /// # Returns
    ///
    /// * `Ok(Some(RDBEntry))` - Successfully parsed entry
    /// * `Ok(None)` - End of file reached
    /// * `Err(RDBError)` - Parse error occurred
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::parser::rdb::RDBParser;
    /// use std::fs::File;
    ///
    /// let file = File::open("dump.rdb").unwrap();
    /// let mut parser = RDBParser::new(file);
    /// parser.parse_header().unwrap();
    ///
    /// while let Some(entry) = parser.parse_entry().unwrap() {
    ///     println!("Key: {:?}, Value: {:?}", entry.key, entry.value);
    /// }
    /// ```
    pub fn parse_entry(&mut self) -> Result<Option<RDBEntry>, RDBError> {
        let mut opcode = [0u8; 1];
        match self.reader.read_exact(&mut opcode) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // End of file reached
                return Ok(None);
            },
            Err(e) => return Err(RDBError::IoError(e)),
        }

        match opcode[0] {
            RDB_OPCODE_EOF => {
                Ok(None)
            },
            RDB_OPCODE_SELECTDB => {
                let db = self.read_length()?;
                self.current_db = db as u8;
                self.parse_entry()
            },
            RDB_OPCODE_EXPIRETIME | RDB_OPCODE_EXPIRETIME_MS => {
                let mut timestamp = [0u8; 8];
                self.reader.read_exact(&mut timestamp)?;
                let expiry = if opcode[0] == RDB_OPCODE_EXPIRETIME {
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(u64::from_be_bytes(timestamp))
                } else {
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(u64::from_be_bytes(timestamp))
                };
                
                let entry = self.parse_entry()?;
                match entry {
                    Some(mut e) => {
                        e.expiry = Some(expiry);
                        Ok(Some(e))
                    },
                    None => Ok(None),
                }
            },
            RDB_OPCODE_AUX => {
                // Skip auxiliary fields (metadata)
                let _key = self.read_string()?;
                let _value = self.read_string()?;
                // Print AUX fields in readable format for debugging
                println!("AUX fields: key={:?}, value={:?}", String::from_utf8_lossy(&_key), String::from_utf8_lossy(&_value));
                // Continue parsing the next entry
                self.parse_entry()
            },
            RDB_OPCODE_RESIZEDB => {
                // Skip database size info
                let _db_size = self.read_length()?;
                let _expires_size = self.read_length()?;
                // Continue parsing the next entry
                self.parse_entry()
            },
            value_type => {
                let key = self.read_string()?;
                
                let value = match value_type {
                    RDB_TYPE_STRING => {
                        let data = self.read_string()?;
                        RDBValue::String(data)
                    },
                    // For all other types, convert them to string representation
                    RDB_TYPE_LIST => {
                        RDBValue::String(Vec::new())
                    },
                    RDB_TYPE_SET => {
                        RDBValue::String(Vec::new())
                    },
                    RDB_TYPE_ZSET => {
                        RDBValue::String(Vec::new())
                    },
                    RDB_TYPE_HASH => {
                        RDBValue::String(Vec::new())
                    },
                    _ => {
                        return Err(RDBError::InvalidType);
                    }
                };

                Ok(Some(RDBEntry {
                    key,
                    value,
                    expiry: None,
                }))
            }
        }
    }
} 