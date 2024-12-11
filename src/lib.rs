use std::net::TcpStream;
use std::io::{Read, Write};

pub mod parser;

use parser::{RESPCommand, RESPOutput};

use crate::parser::Parser;

pub fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    println!("Connection closed");
                    break;
                }

                let parsed_resp = Parser::parse(&buffer[..size])
                    .expect("Failed to parse command");

                // println!("Parsed: {:?}", parsed_resp);

                match parsed_resp.0 {
                    RESPOutput::Array(elements) => {
                        let command = parse_resp_to_command(elements);

                        // println!("Command: {:?}", command);

                        match command {
                            RESPCommand::Ping => {
                                let response = "+PONG\r\n";
                                stream.write_all(response.as_bytes()).unwrap();
                            }
                            RESPCommand::Echo(s) => {
                                let response = format!("${}\r\n{}\r\n", s.len(), s);
                                stream.write_all(response.as_bytes()).unwrap();
                            }
                        }
                    }
                    _ => todo!(),
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}

pub fn parse_resp_to_command(resp: Vec<RESPOutput>) -> RESPCommand {
    let command = resp.get(0).unwrap();
    let args = resp.get(1).unwrap();

    match command {
        RESPOutput::BulkString(s) => {
            if s.to_uppercase() == "PING" {
                RESPCommand::Ping
            } else if s.to_uppercase() == "ECHO" {
                if let RESPOutput::BulkString(s) = args {
                    RESPCommand::Echo(s.to_string())
                } else {
                    todo!()
                }
            } else {
                todo!()
            }
        }
        _ => todo!(),
    }
}
