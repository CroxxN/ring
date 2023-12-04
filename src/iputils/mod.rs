use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::vec::IntoIter;

use crate::RingError;
use crate::DATA;

pub fn get_ip4_addr(mut socket: IntoIter<SocketAddr>) -> Result<SocketAddr, RingError> {
    let ipv4addr = socket.try_for_each(|addr| {
        if addr.is_ipv4() {
            if let std::net::IpAddr::V4(ip) = addr.ip() {
                if ip.is_loopback() {
                    println!("\n\x1b[1;33m[WARNING]: Ringing a loopback address\x1b[0m\n");
                }
            }
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

pub fn get_ip6_addr(socket: IntoIter<SocketAddr>) -> Result<SocketAddr, RingError> {
    // Very hacky
    let ipv6addr = socket.clone().try_for_each(|addr| {
        if addr.is_ipv6() {
            if let std::net::IpAddr::V6(ip) = addr.ip() {
                if ip.is_loopback() {
                    println!("\n\x1b[1;33m[WARNING]: Ringing a loopback address\x1b[0m\n");
                }
            }
            return ControlFlow::Break(addr);
        }
        ControlFlow::Continue(())
    });

    if let ControlFlow::Break(a) = ipv6addr {
        Ok(a)
    } else {
        Err(RingError::NetworkError)
    }
}
// fn psuedo_check(pheader: &[u8]) -> u32 {}

#[derive(Debug, PartialEq, Eq)]
pub struct EchoICMP<'a> {
    pub echo_type: u8,
    pub code: u8,
    pub identifier: [u8; 2],
    pub seq_num: u16,
    pub base_chcksm: u32,
    pub echo_data: &'a [u8; 21],
}

impl<'a> Default for EchoICMP<'a> {
    fn default() -> Self {
        Self {
            echo_type: 128,
            code: 0,
            identifier: [0; 2],
            seq_num: 0,
            echo_data: DATA,
            base_chcksm: 0,
        }
    }
}

impl<'b> EchoICMP<'b> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_v4() -> Self {
        let mut temp = Self::default();
        temp.echo_type = 8;
        temp
    }
    // Change this function to accept a bool to indicate where it should return the checksum or not
    // fn calc_checksum(&mut self, bytes: &mut [u8; 14], some: bool ) -> Option<[u8; 2]>
    pub fn update_chksm(&mut self, bytes: &mut [u8]) {
        bytes[2] = 0;
        bytes[3] = 0;
        let mut sum = self.base_chcksm;
        // for word in bytes.chunks(2) {
        let mut part = u16::from(bytes[6]) << 8;
        part += u16::from(bytes[7]);
        sum = sum.wrapping_add(u32::from(part));
        // }

        while (sum >> 16) > 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        let sum = !sum as u16;
        bytes[2] = (sum >> 8) as u8;
        bytes[3] = (sum & 0xff) as u8;
    }

    fn calc_checksum(&mut self, bytes: &[u8]) -> u32 {
        let mut chck = 0u32;
        for word in bytes.chunks(2) {
            let mut part = u16::from(word[0]) << 8;
            if word.len() > 1 {
                part += u16::from(word[1]);
            }
            chck = chck.wrapping_add(u32::from(part));
        }
        while (chck >> 16) > 0 {
            chck = (chck & 0xffff) + (chck >> 16);
        }
        chck
    }

    pub fn increase_seq(&mut self, container: &mut [u8]) {
        self.seq_num += 1;
        container[6] = (self.seq_num >> 8) as u8;
        container[7] = (self.seq_num & 0x00FF) as u8;
    }

    // initialize ipv4 bytes
    pub fn init_bytes(&mut self, container: &mut [u8]) {
        container[0] = self.echo_type;
        container[1] = self.code;
        container[4] = self.identifier[0];
        container[5] = self.identifier[1];
        container[8..].copy_from_slice(&self.echo_data[0..21]);
        self.base_chcksm = self.calc_checksum(container);
    }
    pub fn update_bytes(&mut self, final_bytes: &mut [u8]) {
        self.update_chksm(final_bytes);
    }
}
