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

#[derive(Debug, PartialEq, Eq)]
pub struct EchoICMPv4<'a> {
    pub echo_type: u8,
    pub code: u8,
    pub identifier: [u8; 2],
    pub seq_num: u16,
    pub base_chcksm: u32,
    pub echo_data: &'a [u8; 7],
}

impl<'a> Default for EchoICMPv4<'a> {
    fn default() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            identifier: [0; 2],
            seq_num: 0,
            echo_data: DATA,
            base_chcksm: 0,
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
    pub fn update_chksm(&mut self, bytes: &mut [u8]) {
        // let local_adr = self.
        // let psuedo_bytes: &mut [u8] = &mut [0];
        // psuedo_bytes.copy_from_slice(bytes);
        // calc_checksum_g(self.base_chcksm, bytes, None);
        bytes[2] = 0;
        bytes[3] = 0;
        let mut sum = self.base_chcksm as u32;
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
    fn calc_checksum(&mut self, bytes: &mut [u8]) {
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

        bytes[2] = (chck >> 8) as u8;
        bytes[3] = (chck & 0xff) as u8;
    }
    pub fn increase_seq(&mut self) {
        self.seq_num += 1;
    }

    // initialize ipv4 bytes
    pub fn init_bytes(&mut self, container: &mut [u8]) {
        container[0] = self.echo_type;
        container[1] = self.code;
        // It's already zero, but still make sure
        container[2] = 0;
        container[3] = 0;

        container[4] = self.identifier[0];
        container[5] = self.identifier[1];
        container[8..].copy_from_slice(&self.echo_data[0..7]);
        self.calc_checksum(container);
        let temp = (((container[2] as u16) << 8) | (container[3] as u16 & 0x00FF)) as u32;
        // self.base_chcksm = temp.wrapping_add(0);
        self.base_chcksm = temp;
    }

    pub fn final_bytes<'a>(&mut self, final_bytes: &mut [u8]) {
        self.increase_seq();
        final_bytes[6] = (self.seq_num >> 8) as u8;
        final_bytes[7] = (self.seq_num & 0x00FF) as u8;
        self.update_chksm(final_bytes);
    }
}
