#![allow(unused_imports)]

use std::{
    env,
    net::{IpAddr, TcpListener, ToSocketAddrs},
};
mod cli_parse;
mod error;
use error::{ErrorSource::*, RingError};

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let url = get_url_from_args(arg);
    Ok(())
}
// \x1b[1m
