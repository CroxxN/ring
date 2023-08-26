use socket2::{Domain, Protocol, Socket, Type};
use std::{env, io::Read, net::ToSocketAddrs};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

// Fixed Constant Data used to Ping the server
// It's completely arbitrary

#[derive(Debug, PartialEq, Eq)]
struct EchoRequest {
    echo_type: u8,
    code: u8,
    checksum: [u8; 2],
    identifier: [u8; 2],
    seq_num: u16,
    echo_data: [u8; 6],
}

impl Default for EchoRequest {
    fn default() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            checksum: [0; 2],
            identifier: [0; 2],
            seq_num: 0,
            echo_data: b"MITTEN".to_owned(),
        }
    }
}

impl EchoRequest {
    fn new() -> Self {
        Self::default()
    }
    fn calc_checksum(&mut self) -> [u8; 14] {
        let mut sum = 0u32;
        let mut bytes = self.final_bytes();
        for word in bytes.chunks(2) {
            let mut part = u16::from(word[0]) << 8;
            if word.len() > 1 {
                part += u16::from(word[1]);
            }
            sum = sum.wrapping_add(u32::from(part));
        }

        while (sum >> 16) > 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        let sum = !sum as u16;
        bytes[2] = (sum >> 8) as u8;
        bytes[3] = (sum & 0xff) as u8;
        println!("The final bytes is {:?}", bytes);
        bytes
    }
    fn _increase_seq(&mut self) {
        self.seq_num = self.seq_num + 1;
    }
    fn final_bytes(&mut self) -> [u8; 14] {
        let mut final_bytes: [u8; 14] = [0; 14];
        final_bytes[0] = self.echo_type;
        final_bytes[1] = self.code;
        final_bytes[4] = self.identifier[0];
        final_bytes[5] = self.identifier[1];
        final_bytes[6] = (self.seq_num >> 8) as u8;
        final_bytes[7] = (self.seq_num & 0x00FF) as u8;
        final_bytes[8] = self.echo_data[0];
        final_bytes[9] = self.echo_data[1];
        final_bytes[10] = self.echo_data[2];
        final_bytes[11] = self.echo_data[3];
        final_bytes[12] = self.echo_data[4];
        final_bytes[13] = self.echo_data[5];
        final_bytes
    }
}

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let mut url = get_args(arg)?;
    if !url.contains(":") {
        url = format!("{}:0", url);
    }

    let sock_addr = if let Some(dest) = url.to_socket_addrs()?.last() {
        println!("Socket Address: {dest}");
        dest
    } else {
        println!("\x1b[1;31mDestination Host Unparsable\x1b[0m");
        return Err(RingError::NetworkError);
    };
    let mut socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    match socket.connect(&sock_addr.into()) {
        Ok(()) => {
            println!("Successfully Connected to Host")
        }
        Err(e) => {
            println!("{e}");
        }
    };
    let mut echo = EchoRequest::new();
    echo.calc_checksum();
    match socket.send_to(&echo.final_bytes(), &sock_addr.into()) {
        Ok(i) => println!("Successfully sent {} bytes", i),
        Err(_) => return Err(RingError::NetworkError),
    }
    let mut buf = [0u8; 64];
    match socket.read(&mut buf) {
        Ok(i) => {
            println!("{buf:?}");
            println!("{} bytes successfully returned from the server", i)
        }
        Err(e) => {
            println!("Encountered an Error: {e}")
        }
    }
    println!("\nSuccessfully Ringed! Now exiting!");
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
// \x1b[1m
