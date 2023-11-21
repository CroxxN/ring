use crate::iputils::ip4;
use crate::iputils::ip4::EchoICMPv4;
use crate::iputils::ip6;
use crate::iputils::ip6::EchoICMPv6;
use crate::{error::RingError, DATA_LENGTH};
use ctrlc;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    env,
    io::Read,
    net::{SocketAddr, ToSocketAddrs},
    sync::{
        atomic::AtomicBool,
        mpsc::{self, channel},
        Arc, Condvar, Mutex,
    },
    thread, time,
};

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
    identifier: [u8; 2],
    seq_num: u16,
    echo_data: [u8; 6],
    init_check: u32,
}

impl Default for EchoRequest {
    fn default() -> Self {
        Self {
            echo_type: 8,
            code: 0,
            identifier: [0; 2],
            seq_num: 1,
            // Fixed Constant Data used to Ping the server
            // It's completely arbitrary
            echo_data: b"MITTEN".to_owned(),
            init_check: 0,
        }
    }
}

#[derive(PartialEq, Eq)]
enum RingMessage {
    Continue((u16, time::Instant)),
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
    let parsed_socket_vec = url.to_socket_addrs()?;
    let addr = parsed_socket_vec.into_iter().try_for_each(|a| {
        if a.is_ipv4() {
            return std::ops::ControlFlow::Break(a);
        }
        std::ops::ControlFlow::Continue(())
    });
    // Use std::ops::ControlFlow::Break().break_value() when it stabalizes
    if let std::ops::ControlFlow::Break(s) = addr {
        return Ok(s);
    } else {
        return Err(RingError::NetworkError);
    }
}

// Global Checksum Calculator
// Making this function global such that it's not tied to a `EchoRequest` struct
// Calculating checksums is required when data is returned, so instead of typing it
// down as a method, it's global

fn calculate_chcksm_g() {
    unimplemented!()
}

// fn calc_psuedo_checksum(bytes: &mut [u8]) -> u16 {
//     let new_bytes: &mut [u8] = &mut [0];
//     // new_bytes[0..4].copy_from_slice();
//     let mut sum = 0u32;
//     // Source Address
//     new_bytes[0..12].copy_from_slice(unimplemented!());
//     // Destination Address
//     new_bytes[12..28].copy_from_slice(unimplemented!());
//     // ICMPv6 length
//     new_bytes[28..32].copy_from_slice(unreachable!());
//     // zeros + next header
//     new_bytes[32..36].copy_from_slice(unreachable!());
//     // Actual ICMPv6 data
//     new_bytes[36..].copy_from_slice(&bytes);
//     for word in new_bytes.chunks(2) {
//         let mut part = u16::from(word[0]) << 8;
//         if word.len() > 1 {
//             part += u16::from(word[1]);
//         }
//         sum = sum.wrapping_add(u32::from(part));
//     }

//     while (sum >> 16) > 0 {
//         sum = (sum & 0xffff) + (sum >> 16);
//     }

//     let sum = !sum as u16;
//     return sum;
// }

// Accepts a data buffer with already calculated and checksum-ed fields
// Checks if the checksum provided in the value matches with the one we
// calculate. If they don't match, some data has been corrupted
// Making global for the same resean as the above function + it can't be
// tied down to any struct
fn check_checksum_g(bytes: &mut [u8]) -> bool {
    // let init_checksum = ((bytes[2] as u16) << 8) | (bytes[3] as u16);
    // let mut final_checksum = 0;
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

    let chck = !chck as u16;
    if chck == 0 {
        return true;
    }
    false
    // calc_checksum_g(init_check, bytes, Some(&mut final_checksum));
    // init_checksum == final_checksum
}

// impl EchoRequest {

fn handle_returned(rx: mpsc::Receiver<RingMessage>, mut recv_socket: Socket) -> (u32, u32, u32) {
    let mut rtx = (0u32, 0u32, 0u32);
    let mut buf = [0; 64];
    'outer: loop {
        match rx.recv() {
            // Don't need to check because there are only two variants and one is already coverd
            Ok(m) => {
                let instant;
                if let RingMessage::Continue((_, i)) = m {
                    instant = i;
                    // Can't help spining
                    // while there is no data on the buffer, and 1s has not elapsed,
                    // keep spinning. Also keep checking if we receive SIGINT.
                    while i.elapsed().as_millis() < 1000 && recv_socket.peek_sender().is_err() {
                        // If the user presses CTRL + C while we're waiting for a reply, exit every thing
                        if rx.try_recv().is_ok_and(|v| v == RingMessage::Stop) {
                            break 'outer;
                        }
                        continue;
                    }
                } else {
                    // If there is a packet and that packet is ICMP echo reply, discard it
                    if recv_socket.peek_sender().is_ok() {
                        rtx.2 = rtx.2 + 1;
                    }
                    break;
                }
                // Weird hack to return as soon as CTRL + C is hit.
                // We could do it with timeout, but if we do, pressing CTRL + C
                // doesn't immediety return
                match recv_socket.read(&mut buf) {
                    Ok(i) => {
                        let time = instant.elapsed().as_millis();
                        // If Ctrl + C is already pressed, but there is still data on the buffer,
                        // we currently discard it.
                        let len = ((buf[0] & 0x0F) << 2) as usize;
                        // If the packet isn't ICMP echo reply, discard it.
                        if buf[len] != 0 {
                            rtx.2 = rtx.2 + 1;
                            continue;
                        }

                        let ttl = buf[8];
                        let seq = (buf[len + 6] as u16) << 8 | (buf[len + 7] as u16);
                        if !check_checksum_g(&mut buf[len..i]) {
                            rtx.1 = rtx.1 + 1;
                        } else {
                            rtx.0 = rtx.0 + 1;
                        }
                        println!(
                        "\x1b[1;32m{} bytes \x1b[37mreturned. \x1b[1;32mICMP Sequence Packet:\x1b[1;37m {}, \x1b[1;32mTTL: \x1b[1;37m{}, \x1b[32mTime: \x1b[1;37m{} ms\x1b[0m", (i - len), seq, ttl, time
                            );
                    }
                    Err(_e) => {
                        let _seq_num;
                        if let RingMessage::Continue((i, _)) = m {
                            _seq_num = i;
                            // We actually report timed-out packets instead of just ignoring it.
                            // Also destination host unrechable is just timed-out packets.
                            println!(
                                "\x1b[1;31mPacket Timed Out. ICMP Sequence Packet: {}\x1b[0m",
                                _seq_num
                            );
                        }
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    return rtx;
}

pub fn run(socket: &Socket) -> Result<(), RingError> {
    // let arg: Vec<String> = env::args().collect();
    // let mut url = get_args(arg)?;
    // if !url.contains(":") {
    //     url = format!("{}:0", url);
    // }
    // // let parsed_url = if let Some(u) = url.split_once(":") {
    // //     u.0
    // // } else {
    // //     &url
    // // };
    // let sock_addr = ip4_socket(&url)?;
    // let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    // // socket.set_read_timeout(Some(time::Duration::from_millis(500)))?; // Also the timeout for ICMP packets

    // socket.set_nonblocking(true)?;

    // match socket.connect(&sock_addr.into()) {
    //     Ok(()) => {}
    //     Err(e) => {
    //         println!("{e}");
    //     }
    // };

    // println!(
    //     // Terminal Color(VT100) Specification form (https://chrisyeh96.github.io/2020/03/28/terminal-colors.html)

    //     "\n\x1b[1;32mRinging \x1b[0m\x1b[4;34m{}({})\x1b[0m \x1b[1;32mwith \x1b[1;37m{} bytes\x1b[0m\x1b[1;32m of data\x1b[0m\n",
    //     url, sock_addr, 14

    // );

    let (tx, rx) = channel::<RingMessage>();

    let mut echo = EchoICMPv4::new();
    // let address = socket.local_addr()?.as_socket_ipv6().unwrap().ip().octets();

    // let ip_as_string;
    // if let Some(addr) = address {
    //     ip_as_string = addr.to_string();
    // } else {
    //     return Ok(());
    // }
    // echo.source = ip_as_string.as_bytes().try_into().unwrap();

    // TEST: change
    // This bit seems extremely hacky. I don't want to introduce new dependency for MPMC channel, as
    // the std MPSC channel is not suitable for the task below.
    //Also, use Condvar?
    let cont = Arc::new(AtomicBool::new(true));
    // let cont_recv = Arc::clone(&cont);
    let cont_send = Arc::clone(&cont);

    // Condvar! YAY!
    let pcond = Arc::new((Mutex::new(false), Condvar::new()));
    let scond = pcond.clone();

    let recv_socket = socket.try_clone()?;
    socket.set_nonblocking(true)?; // IMPORTANT
    let handle = thread::spawn(move || handle_returned(rx, recv_socket));

    // Use a mut array of u8, so increasing the `seq_num` doesn't require creating a whole new copy of
    // bytes.
    let mut packet: [u8; DATA_LENGTH] = [0; DATA_LENGTH];
    echo.final_bytes(&mut packet);

    // Weirdly, you have to clone the `cont` variabel here. If cloned inside the `FnMut`, the compiler shouts

    // The ctrlc crate takes a FnMut as an argument. We use Arc to store and load a boolen value to
    // determine when to stop running the loop

    let tx_clone = tx.clone();
    ctrlc::set_handler(move || {
        cont_send.store(false, std::sync::atomic::Ordering::Relaxed);
        let (lock, cond) = &*scond;
        // Unwrap seems good here. There is not much we can do if sending
        // stop messege fails. The best thing to do would be to exit the program.
        let mut lock = lock.lock().expect("Failed to aquire the lock");
        *lock = true;
        cond.notify_all();
        tx_clone.send(RingMessage::Stop).unwrap();
    })
    .expect("Failed to register callback");

    // Starts measuring and taking stats
    // We initialize the stat struct here to be as correct as possible while measuring the time taken.
    // If we start early, the internal calculations may dilute the time
    let mut stats = RingStats::default();

    let (lock, cond) = &*pcond;

    loop {
        let time = time::Instant::now();
        socket.send(&packet)?;
        if let Err(_) = tx.send(RingMessage::Continue((echo.seq_num, time))) {
            return Err(RingError::ChannelSendError);
        };
        stats.packet_sent = stats.packet_sent + 1;
        echo.final_bytes(&mut packet);
        let mut lock = lock.lock().unwrap();
        let res = cond
            .wait_timeout(lock, time::Duration::from_secs(1))
            .unwrap();
        lock = res.0;
        if *lock {
            drop(tx);
            break;
        }
    }
    // Add code here to give the diagnostics result of all the times Pinged.
    let (sucsess, loss, discard) = match handle.join() {
        Ok((s, l, d)) => (s, l, d),
        Err(_) => (0, 0, 0),
    };
    stats.packet_sent = stats.packet_sent - discard;
    stats.loss = loss + (stats.packet_sent - sucsess);
    stats.successful = sucsess;
    println!("\n\x1b[1;32m------------Ring Stats------------\x1b[0m");
    println!(
        "\n\x1b[1;32mRinged!\x1b[0m Received \x1b[1;32m{} packets\x1b[0m of  \x1b[1;32m{} total packets,\x1b[0m with \x1b[1;31m{}% loss!\x1b[0m Pinged for \x1b[1;32m{} seconds\x1b[0m.",
        stats.successful,
        stats.packet_sent,
        ((stats.loss * 100) / stats.packet_sent as u32),
        stats.time.elapsed().as_secs()
    );

    // Free Up the socket just in case
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
