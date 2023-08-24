#![allow(unused_imports)]

use std::{
    env,
    net::{IpAddr, TcpListener, ToSocketAddrs},
};
mod cli_parse;
mod error;
use cli_parse::get_url_from_args;
use error::RingError;

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let url = get_url_from_args(arg)?;
    let _sock_addr = url.to_socket_addrs()?;
    Ok(())
}
// \x1b[1m
