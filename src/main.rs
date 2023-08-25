use socket2::{Domain, Protocol, Socket, Type};
use std::{env, net::ToSocketAddrs};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

#[derive(Debug, PartialEq, Eq)]
struct EchoRequest {
    echo_type: u8,
    code: u8,
    checksum: [u8; 2],
    identifier: [u8; 2],
    seq_num: [u8; 2],
    echo_data: [u8; 3],
}

impl EchoRequest {
    fn new() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            checksum: [0; 2],
            identifier: [0; 2],
            seq_num: [0; 2],
            echo_data: [0; 3],
        }
    }
    fn calc_checksum(&mut self) {
        unimplemented!()
    }
    fn populate_data(&mut self) {
        unimplemented!()
    }
}

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let mut url = get_args(arg)?;
    if !url.contains(":") {
        url = format!("{}:7", url);
    }

    let sock_addr = if let Some(dest) = url.to_socket_addrs()?.last() {
        dest
    } else {
        println!("\x1b[1;31mDestination Host Unparsable\x1b[0m");
        return Err(RingError::NetworkError);
    };
    let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    socket.connect(&sock_addr.into())?;
    let _echo = EchoRequest::new();
    Ok(())
}
// \x1b[1m
