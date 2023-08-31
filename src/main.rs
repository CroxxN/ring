use ctrlc;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    env,
    io::Read,
    net::ToSocketAddrs,
    sync::{atomic::AtomicBool, Arc},
    thread,
};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

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
            // Fixed Constant Data used to Ping the server
            // It's completely arbitrary
            echo_data: b"MITTEN".to_owned(),
        }
    }
}

impl EchoRequest {
    fn new() -> Self {
        Self::default()
    }
    fn calc_checksum(&mut self) {
        let mut sum = 0u32;
        let bytes = self.final_bytes();
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
        self.checksum[0] = (sum >> 8) as u8;
        self.checksum[1] = (sum & 0xff) as u8;
    }
    fn _increase_seq(&mut self) {
        self.seq_num = self.seq_num + 1;
    }
    fn final_bytes(&mut self) -> [u8; 14] {
        let mut final_bytes: [u8; 14] = [0; 14];
        final_bytes[0] = self.echo_type;
        final_bytes[1] = self.code;
        final_bytes[2] = self.checksum[0];
        final_bytes[3] = self.checksum[1];
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
        dest
    } else {
        println!("\x1b[1;31mFailed to parse url\x1b[0m");
        return Err(RingError::NetworkError);
    };

    // YES! ARC! Electric!
    let socket = Arc::new(Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4),
    )?);

    match socket.connect(&sock_addr.into()) {
        Ok(()) => {}
        Err(e) => {
            println!("{e}");
        }
    };
    let mut echo = EchoRequest::new();
    echo.calc_checksum();
    let mut buf = [0u8; 64];
    let recv_socket = Arc::clone(&socket);
    let cont = Arc::new(AtomicBool::new(true));
    let cont_recv = Arc::clone(&cont);
    thread::spawn(move || {
        while cont_recv.load(std::sync::atomic::Ordering::SeqCst) {
            match recv_socket.as_ref().read(&mut buf) {
                Ok(i) => {
                    println!(
                        "\n{} bytes successfully returned from the server",
                        i.wrapping_sub(20)
                    )
                }
                Err(e) => {
                    println!("Encountered an Error: {e}")
                }
            }
            thread::sleep(std::time::Duration::new(1, 0));
        }
    });
    let final_bytes = &echo.final_bytes();
    // Currently Unrechable - Create a breaking condition
    let cont_send = Arc::clone(&cont);

    // The ctrlc crate takes a FnMut as an argument. We use Arc to store and load a boolen value to
    // determine when to stop running the loop

    ctrlc::set_handler(move || cont_send.store(false, std::sync::atomic::Ordering::SeqCst))
        .expect("Failed to register callback");

    // Trial. Will see if the overhead of Arc exceeds that of channels, and if so, will replace with channels
    while cont.load(std::sync::atomic::Ordering::SeqCst) {
        match socket.send(final_bytes) {
            Ok(i) => {
                println!(
                // Terminal Color(VT100) Specification form (https://chrisyeh96.github.io/2020/03/28/terminal-colors.html)

                "\n\x1b[1;32mRinging \x1b[0m\x1b[4;34m{}({})\x1b[0m \x1b[1;32mwith \x1b[1;37m{} bytes\x1b[0m\x1b[1;32m of data\x1b[0m",
                url, sock_addr, i
                );
            }
            Err(_) => return Err(RingError::NetworkError),
        }
        thread::sleep(std::time::Duration::new(1, 0));
    }
    // Add code here to give the diagnostics result of all the times Pinged.

    println!("\nSuccessfully Ringed! Now exiting!");
    // Free Up the socket just in case
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
// \x1b[1m
