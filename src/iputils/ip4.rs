use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::ControlFlow;
use std::vec::IntoIter;

use crate::utils::calc_checksum_g;
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
pub struct EchoICMPv4<'a> {
    pub echo_type: u8,
    pub code: u8,
    pub identifier: [u8; 2],
    pub seq_num: u16,
    pub init_check: u32,
    pub echo_data: &'a [u8; 7],
}

impl<'a> Default for EchoICMPv4<'a> {
    fn default() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            identifier: [0; 2],
            seq_num: 1,
            echo_data: DATA,
            init_check: 0,
        }
    }
}

impl<'b> EchoICMPv4<'b> {
    pub fn new() -> Self {
        Self::default()
    }
    // Change this function to accept a bool to indicate where it should return the checksum or not
    // fn calc_checksum(&mut self, bytes: &mut [u8; 14], some: bool ) -> Option<[u8; 2]>
    #[inline]
    pub fn calc_checksum(&mut self, bytes: &mut [u8]) {
        // let local_adr = self.
        // let psuedo_bytes: &mut [u8] = &mut [0];
        // psuedo_bytes.copy_from_slice(bytes);
        calc_checksum_g(self.init_check, bytes, None);
    }
    pub fn increase_seq(&mut self) {
        self.seq_num += 1;
    }
    pub fn final_bytes<'a>(&mut self, final_bytes: &mut [u8]) {
        if final_bytes[0] == self.echo_type {
            self.increase_seq();
            final_bytes[6] = (self.seq_num >> 8) as u8;
            final_bytes[7] = (self.seq_num & 0x00FF) as u8;
            self.calc_checksum(final_bytes);
            return;
        }
        final_bytes[0] = self.echo_type;
        final_bytes[1] = self.code;
        // It's already zero, but still make sure
        final_bytes[2] = 0;
        final_bytes[3] = 0;

        final_bytes[4] = self.identifier[0];
        final_bytes[5] = self.identifier[1];
        final_bytes[6] = (self.seq_num >> 8) as u8;
        final_bytes[7] = (self.seq_num & 0x00FF) as u8;
        final_bytes[8..].copy_from_slice(&self.echo_data[0..7]);
        self.calc_checksum(final_bytes);
    }
}
