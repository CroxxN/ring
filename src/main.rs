use ctrlc;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    env,
    io::Read,
    net::{SocketAddr, ToSocketAddrs},
    sync::{atomic::AtomicBool, mpsc::channel, Arc},
    thread, time,
};
mod cli_parse;
mod error;
use cli_parse::get_args;
use error::RingError;

#[derive(Debug, PartialEq, Eq)]
struct RingStats {
    packet_sent: u32,
    successful: u32,
    loss: u32,
    // Better than using `Duration` as `Instant` exits specially for this purpose
    time: time::Instant,
}

impl Default for RingStats {
    fn default() -> Self {
        Self {
            packet_sent: 0,
            successful: 0,
            loss: 0,
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
            seq_num: 1,
            // Fixed Constant Data used to Ping the server
            // It's completely arbitrary
            echo_data: b"MITTEN".to_owned(),
        }
    }
}

#[derive(PartialEq, Eq)]
enum RingMessage {
    Continue,
    Stop,
}

// Maybe in subsequent versions, the function below will be a generic one to get either IPV4 or IPV6
// Could make it by adding a type parameter `<IPVersion>` in the function description, and make it generic over
// IP versions

// The function could look like:

// fn get_ip_socket<IPVersion>(url: &str) -> Result<SocketAddr, RingError>{}
// where `IPVersion` is
// enum IPVersion {
//     IPV4,
//     IPV6
// }
// Better yet, the whole function could be separated over the another module entirely
// So, this function is curretly unstable, and may break over time
// TODO: Make this function generic over both IP versions

fn ip4_socket(url: &str) -> Result<SocketAddr, RingError> {
    let mut parsed_socket_vec = url.to_socket_addrs()?;
    let sock_addr = match parsed_socket_vec.next() {
        Some(addr) if addr.is_ipv4() => addr,
        Some(_) => {
            if let Some(addr) = parsed_socket_vec.last() {
                addr
            } else {
                return Err(RingError::NetworkError);
            }
        }
        None => {
            return Err(RingError::NetworkError);
        }
    };
    return Ok(sock_addr);
}

impl EchoRequest {
    fn new() -> Self {
        Self::default()
    }
    // Change this function to accept a bool to indicate where it should return the checksum or not
    // fn calc_checksum(&mut self, bytes: &mut [u8; 14], some: bool ) -> Option<[u8; 2]>
    #[inline]
    fn calc_checksum(&mut self, bytes: &mut [u8; 14]) {
        let mut sum = 0u32;
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
        bytes[2] = self.checksum[0];
        bytes[3] = self.checksum[1];
    }
    fn increase_seq(&mut self) {
        self.seq_num = self.seq_num + 1;
    }
    fn final_bytes<'a>(&mut self, final_bytes: &mut [u8; 14]) {
        if final_bytes[0] == self.echo_type {
            self.increase_seq();
            final_bytes[2] = 0;
            final_bytes[3] = 0;
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
        final_bytes[8] = self.echo_data[0];
        final_bytes[9] = self.echo_data[1];
        final_bytes[10] = self.echo_data[2];
        final_bytes[11] = self.echo_data[3];
        final_bytes[12] = self.echo_data[4];
        final_bytes[13] = self.echo_data[5];
        self.calc_checksum(final_bytes);
    }
    // Will change `_ping_status()` to `ping_status()` currently uses
    // underscore to get the compiler stop shouting
    fn _ping_status() {
        //
        unimplemented!()
    }

    //To get the compiler to stop shouting about `ByteParseError` not being used
    fn _parse_error() -> RingError {
        //TODO: Actaully implement this
        RingError::ByteParseError
    }
}

fn main() -> Result<(), RingError> {
    let arg: Vec<String> = env::args().collect();
    let mut url = get_args(arg)?;
    if !url.contains(":") {
        url = format!("{}:0", url);
    }
    // let parsed_url = if let Some(u) = url.split_once(":") {
    //     u.0
    // } else {
    //     &url
    // };
    let sock_addr = ip4_socket(&url)?;
    // YES! ARC! Electric!
    let socket = Arc::new(Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4),
    )?);
    // socket.set_read_timeout(Some(time::Duration::new(1, 0)))?;
    // Just set it to non-blocking
    socket.set_nonblocking(true)?;

    match socket.connect(&sock_addr.into()) {
        Ok(()) => {}
        Err(e) => {
            println!("{e}");
        }
    };
    println!(
        // Terminal Color(VT100) Specification form (https://chrisyeh96.github.io/2020/03/28/terminal-colors.html)

        "\n\x1b[1;32mRinging \x1b[0m\x1b[4;34m{}({})\x1b[0m \x1b[1;32mwith \x1b[1;37m{} bytes\x1b[0m\x1b[1;32m of data\x1b[0m",
        url, sock_addr, 14

    );

    let (tx, rx) = channel::<RingMessage>();

    let mut echo = EchoRequest::new();
    let mut buf = [0u8; 64];

    let mut recv_socket = socket.try_clone()?;

    // This bit seems extremely hacky. I don't want to introduce new dependency for MPMC channel, as
    // the std MPSC channel is not suitable for the task below.
    //Also, use Condvar?
    let cont = Arc::new(AtomicBool::new(true));
    // let cont_recv = Arc::clone(&cont);
    let cont_send = Arc::clone(&cont);

    let handle = thread::spawn(move || {
        let mut rtx = (0u32, 0u32, 0u32);
        loop {
            match rx.recv() {
                // Don't need to check because there are only two variants and one is already coverd
                Ok(m) => {
                    match recv_socket.read(&mut buf) {
                        Ok(i) => {
                            // If Ctrl + C is already pressed, but there is still data on the buffer,
                            // we currently discard it. TODO: Check the buffer regardless for data
                            // integrity, and if corrupted, add it to the loss packet var
                            if m == RingMessage::Stop {
                                rtx.2 = rtx.2 + 1;
                                break;
                            }
                            // syncing of atomic bool lags a bit, so when Ctrl + C is pressed,
                            // a zero-sized buffer is returned. We check the length, and print
                            // only when it's greater than 20
                            if i > 20 {
                                println!(
                                    "{} bytes successfully returned from the server",
                                    (i - 20)
                                );
                                rtx.0 = rtx.0 + 1;
                            }
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {
                    println!("Failed to receive message");
                    break;
                }
            }
        }
        return rtx;
    });

    // Use a mut array of u8, so increasing the `seq_num` doesn't require creating a whole new copy of
    // bytes.
    let mut packet: [u8; 14] = [0; 14];
    echo.final_bytes(&mut packet);

    // Weirdly, you have to clone the `cont` variabel here. If cloned inside the `FnMut`, the compiler shouts

    // The ctrlc crate takes a FnMut as an argument. We use Arc to store and load a boolen value to
    // determine when to stop running the loop

    let tx_clone = tx.clone();
    ctrlc::set_handler(move || {
        cont_send.store(false, std::sync::atomic::Ordering::Relaxed);
        // TODO: Remove unwrap
        tx_clone.clone().send(RingMessage::Stop).unwrap();
    })
    .expect("Failed to register callback");

    // Starts measuring and taking stats
    // We initialize the stat struct here to be as correct as possible while measuring the time taken.
    // If we start early, the internal calculations may dilute the the time
    let mut stats = RingStats::default();

    while cont.load(std::sync::atomic::Ordering::Relaxed) {
        socket.send(&packet)?;
        if let Err(_) = tx.send(RingMessage::Continue) {
            return Err(RingError::ByteParseError);
        };
        stats.packet_sent = stats.packet_sent + 1;
        echo.final_bytes(&mut packet);
        // println!("{:?}", packet);
        thread::sleep(std::time::Duration::new(1, 0));
    }
    // Add code here to give the diagnostics result of all the times Pinged.
    let (sucsess, loss, discard) = match handle.join() {
        Ok((s, l, d)) => (s, l, d),
        Err(_) => (0, 0, 0),
    };
    stats.packet_sent = stats.packet_sent - discard;
    stats.successful = sucsess;
    stats.loss = loss;
    println!(
        "\nSuccessfully Ringed! Received {} packets of {} total packets, with {}% loss! Now exiting! Pinged for a total of {} seconds",
        stats.successful,
        stats.packet_sent,
        ((stats.loss * 100) / stats.packet_sent as u32),
        stats.time.elapsed().as_secs()
    );

    // Free Up the socket just in case
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
// \x1b[1m
