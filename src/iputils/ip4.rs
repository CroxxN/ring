use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::ControlFlow;
use std::vec::IntoIter;

use crate::RingError;
use crate::DATA;

pub fn get_ip4_addr(mut socket: IntoIter<SocketAddr>) -> Result<SocketAddr, RingError> {
    // Very hacky
    let ipv4addr = socket.try_for_each(|addr| {
        if addr.is_ipv4() {
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
struct EchoICMPv4<'a> {
    echo_type: u8,
    code: u8,
    identifier: [u8; 2],
    seq_num: u16,
    echo_data: &'a [u8; 7],
}

impl<'a> Default for EchoICMPv4<'a> {
    fn default() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            identifier: [0; 2],
            seq_num: 1,
            echo_data: DATA,
        }
    }
}
