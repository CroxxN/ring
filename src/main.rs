use socket2::{Domain, Protocol, Socket, Type};
use std::{env, net::ToSocketAddrs};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

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

    Ok(())
}
// \x1b[1m
