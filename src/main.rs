use ctrlc;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    env,
    io::Read,
    net::ToSocketAddrs,
    sync::{atomic::AtomicBool, Arc},
    thread, time,
};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

#[derive(Debug, PartialEq, Eq)]
struct RingStats {
    packet_sent: i32,
    successful: Arc<i32>,
    loss: Arc<i32>,
    // Better than using `Duration` as `Instant` exits specially for this purpose
    time: time::Instant,
}

impl Default for RingStats {
    fn default() -> Self {
        Self {
            packet_sent: 0,
            successful: Arc::new(0),
            loss: Arc::new(0),
            time: time::Instant::now(),
        }
    }
}

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
    #[inline]
    fn calc_checksum(&mut self) {
        let mut sum = 0u32;
        let mut bytes = [0u8; 14];
        self.final_bytes(&mut bytes);
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
    fn final_bytes<'a>(&mut self, final_bytes: &mut [u8; 14]) {
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
    }
    // Will change `_ping_status()` to `ping_status()` currently uses
    // underscore to get the compiler stop shouting
    fn _ping_status() {
        //
        unimplemented!()
    }
}

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let mut url = get_args(arg)?;
    if !url.contains(":") {
        url = format!("{}:0", url);
    }

    let sock_addr = if let Some(dest) = url.to_socket_addrs()?.next() {
        dest
    } else {
        println!("\x1b[1;31mFailed to parse url\x1b[0m");
        return Err(RingError::NetworkError);
    };

    // let parsed_url = if let Some(u) = url.split_once(":") {
    //     u.0
    // } else {
    //     &url
    // };

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
    let mut stats = RingStats::default();
    // let arc_recieved = Arc::clone(&mut stats.successful);
    thread::spawn(move || {
        while cont_recv.load(std::sync::atomic::Ordering::Relaxed) {
            match recv_socket.as_ref().read(&mut buf) {
                Ok(i) => {
                    // syncing of atomic bool lags a bit, so when Ctrl + C is pressed,
                    // a zero-sized buffer is returned. We check the length, and print
                    // only when it's greater than 20
                    if i > 20 {
                        println!("{} bytes successfully returned from the server", (i - 20));
                    }
                }
                Err(e) => {
                    println!("Encountered an Error: {e}")
                }
            }
        }
    });
    // Use a mut array of u8, so increasing the `seq_num` doesn't require creating a whole new copy of
    // bytes.
    let mut packet: [u8; 14] = [0; 14];
    echo.final_bytes(&mut packet);
    let cont_send = Arc::clone(&cont);

    // The ctrlc crate takes a FnMut as an argument. We use Arc to store and load a boolen value to
    // determine when to stop running the loop

    ctrlc::set_handler(move || cont_send.store(false, std::sync::atomic::Ordering::Relaxed))
        .expect("Failed to register callback");

    println!(
        // Terminal Color(VT100) Specification form (https://chrisyeh96.github.io/2020/03/28/terminal-colors.html)

        "\n\x1b[1;32mRinging \x1b[0m\x1b[4;34m{}({})\x1b[0m \x1b[1;32mwith \x1b[1;37m{} bytes\x1b[0m\x1b[1;32m of data\x1b[0m",
        url, sock_addr, 14
    );

    // Starts measuring and taking stats
    // We initialize the stat struct here to be as correct as possible while measuring the time taken.
    // If we start early, the internal calculations may dilute the the time

    while cont.load(std::sync::atomic::Ordering::Relaxed) {
        match socket.send(&packet) {
            Ok(_) => {
                stats.packet_sent = stats.packet_sent + 1;
                thread::sleep(std::time::Duration::new(1, 0));
            }
            Err(_) => return Err(RingError::NetworkError),
        }
    }
    // Add code here to give the diagnostics result of all the times Pinged.

    println!(
        "\nSuccessfully Ringed {} packets! Now exiting!",
        stats.packet_sent
    );
    // Free Up the socket just in case
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
// \x1b[1m
