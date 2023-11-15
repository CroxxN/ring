use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::ControlFlow;
use std::vec::IntoIter;

use crate::RingError;

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
