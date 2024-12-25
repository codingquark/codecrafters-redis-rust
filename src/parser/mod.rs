use std::fmt;

#[derive(Debug)]
pub enum RESPOutput {
    Array(Vec<RESPOutput>),
    BulkString(String),
    // TODO: Add other types
    // SimpleString(String),
    // Error(String),
    // Integer(i64),
    // Double(f64),
    // Boolean(bool),
    // Null,
}

#[derive(Debug)]
pub enum RESPCommand {
    Ping,
    Echo(String),
    Set(String, String),
}

#[derive(Debug)]
pub enum ParserError {
    IncompleteInput,
    UnsupportedCommand,
    CRLFNotFound,
    InvalidInput,
}

// Implement Display for ParserError
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::IncompleteInput => write!(f, "Incomplete input"),
            ParserError::UnsupportedCommand => write!(f, "Unsupported command"),
            ParserError::CRLFNotFound => write!(f, "CRLF not found"),
            ParserError::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

// Implement Error for ParserError
impl std::error::Error for ParserError {}

pub type ParserCRLFResult<'a> = Result<(&'a [u8], &'a [u8]), ParserError>;

pub type ParserResult<'a> = Result<(RESPOutput, &'a [u8]), ParserError>;
pub struct Parser {}

impl Parser {
    pub fn parse(input: &[u8]) -> ParserResult {
        // If input is empty, return an error
        if input.len() == 0 || input[0] == 0 {
            return Err(ParserError::IncompleteInput);
        }

        let symbol_lossy = String::from_utf8_lossy(&input[0..1]);
        let symbol = symbol_lossy.as_ref();
        let payload = &input[1..];

        match symbol {
            "*" => Parser::parse_array(payload),
            "$" => Parser::parse_bulk_string(payload),
            _ => Err(ParserError::UnsupportedCommand),
        }
    }

    fn parse_array(payload: &[u8]) -> Result<(RESPOutput, &[u8]), ParserError> {
        // An array is a list of RESP commands, formatted as:
        // *<number of elements>\r\n<element 1>\r\n<element 2>\r\n...<element N>\r\n
        // We need to parse the number of elements, then parse each element
        let parsed = Parser::parse_until_crlf(payload);
        if parsed.is_err() {
            return Err(ParserError::CRLFNotFound);
        }

        let (num_elements, remaining) = parsed.unwrap();
        let num_elements: u32 = match String::from(String::from_utf8_lossy(num_elements)).parse() {
            Ok(num) => num,
            Err(_) => {
                return Err(ParserError::InvalidInput);
            }
        };

        // Now we need to parse each element
        let mut resp_result: Vec<RESPOutput> = Vec::new();
        let mut remaining = remaining;

        for _ in 0..num_elements {
            let parsed = Parser::parse(remaining);
            if parsed.is_err() {
                return Err(ParserError::InvalidInput);
            }
            let (result, rem) = parsed.unwrap();
            resp_result.push(result);
            remaining = rem;
        }

        Ok((RESPOutput::Array(resp_result), remaining))
    }

    fn parse_bulk_string(payload: &[u8]) -> Result<(RESPOutput, &[u8]), ParserError> {
        // Bulk strings are formatted as:
        // $<number of bytes>\r\n<string data>\r\n
        // We need to parse the length, then parse the string data
        let parsed = Parser::parse_until_crlf(payload);
        if parsed.is_err() {
            return Err(ParserError::CRLFNotFound);
        }
        let (length, rem) = parsed.unwrap();
        let length: u32 = match String::from_utf8_lossy(length).parse() {
            Ok(num) => num,
            Err(_) => {
                return Err(ParserError::InvalidInput);
            }
        };

        let parsed = Parser::parse_until_crlf(rem);
        if parsed.is_err() {
            return Err(ParserError::CRLFNotFound);
        }
        let (result, rem) = parsed.unwrap();
        let res = String::from(String::from_utf8_lossy(result));

        // Validate the length of the string
        if res.len() as u32 != length {
            return Err(ParserError::InvalidInput);
        }

        Ok((RESPOutput::BulkString(res), rem))
    }

    fn parse_until_crlf(input: &[u8]) -> ParserCRLFResult {
        for index in 0..input.len() - 1 {
            if input[index] == b'\r' && input[index + 1] == b'\n' {
                return Ok((&input[0..index], &input[index + 2..]));
            }
        }

        Err(ParserError::CRLFNotFound)
    }
}
