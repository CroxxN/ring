use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::ControlFlow;
use std::vec::IntoIter;

use crate::RingError;
use crate::DATA;

pub fn get_ip6_addr(mut socket: IntoIter<SocketAddr>) -> Result<SocketAddr, RingError> {
    // Very hacky
    let ipv4addr = socket.try_for_each(|addr| {
        if addr.is_ipv6() {
            return ControlFlow::Break(addr);
        }
        ControlFlow::Continue(())
    });

    if let ControlFlow::Break(a) = ipv4addr {
        Ok(a)
    } else {
        Err(RingError::NetworkError)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EchoICMPv6<'a> {
    // pseudo header: https://en.wikipedia.org/wiki/ICMPv6#Checksum
    pub source: &'a [u8; 12], // 96 bytes source adderes
    pub destination: &'a [u8; 12],
    pub length: u32,
    pub zeros: u32,
    pub next_header: u8,
    // normal header
    pub echo_type: u8,
    pub code: u8,
    pub identifier: [u8; 2],
    pub seq_num: u16,
    pub echo_data: &'a [u8; 7],
}

impl<'a> Default for EchoICMPv6<'a> {
    fn default() -> Self {
        Self {
            source: &[0; 12],
            destination: &[0; 12],
            length: 15,
            next_header: 58, // const
            zeros: 0,
            echo_type: 128,
            code: 0,
            identifier: [0; 2],
            seq_num: 1,
            echo_data: DATA,
        }
    }
}

trait ICMPv6 {
    fn new() -> Self;
    fn calc_checksum();
}
