use crate::iputils::EchoICMP;
use crate::RingOptions;
use crate::{error::RingError, DATA_LENGTH};

use socket2::Socket;
use std::net::SocketAddr;
use std::{
    io::Read,
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
    // Better than using `Duration` as `Instant` exists specially for this purpose
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

#[derive(PartialEq, Eq)]
enum RingMessage {
    Continue((u16, time::Instant)),
    Stop,
}

// Accepts a data buffer checks if the checksum is correct.
// If not, some data has been corrupted
// Making global as it can't be tied down to any struct
fn check_checksum(bytes: &mut [u8]) -> bool {
    // let mut final_checksum = 0;
    let mut chck = 0_u32;
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
    chck == 0
}

// parse ICMP error messages. See rfc 792
fn parse_error(mtype: u8, code: u8, seq: u16) {
    match mtype {
        3 => match code {
            0 => println!(
                "\x1b[1;31mDestination Network Unreachable. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            1 => println!(
                "\x1b[1;31mDestination Host Unreachable. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            2 => println!(
                "\x1b[1;31mDestination Protocol Unreachable. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            3 => println!(
                "\x1b[1;31mDestination Port Unreachable. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            4 => println!(
                "\x1b[1;31mFragmentation Needed. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            5 => println!(
                "\x1b[1;31mSource Route Failed. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            _ => (),
        },
        4 => {
            if code == 0 {
                println!(
                    "\x1b[1;31mSource Quench. ICMP Sequence Packet: {}\x1b[0m",
                    seq
                )
            }
        }
        11 => match code {
            0 => println!(
                "\x1b[1;31mTime to Live Exceeded. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            1 => println!(
                "\x1b[1;31mFragementation limit Exceeded. ICMP Sequence Packet: {}\x1b[0m",
                seq
            ),
            _ => (),
        },
        12 => {
            if code == 0 {
                println!(
                    "\x1b[1;31mParameter Problem. ICMP Sequence Packet: {}\x1b[0m",
                    seq
                )
            }
        }
        _ => (),
    }
}

fn handle_returned(
    rx: mpsc::Receiver<RingMessage>,
    mut recv_socket: Socket,
    opts: &RingOptions,
) -> (u32, u32, u32) {
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
                    while i.elapsed().as_millis() < opts.timeout
                        && recv_socket.peek_sender().is_err()
                    {
                        // If the user presses CTRL + C while we're waiting for a reply, exit every thing
                        if rx.try_recv().is_ok_and(|v| v == RingMessage::Stop) {
                            break 'outer;
                        }
                        continue;
                    }
                } else {
                    // If there is a packet and that packet is ICMP echo reply, discard it
                    if recv_socket.peek_sender().is_ok() {
                        rtx.2 += 1;
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

                        // Extracting ip header length.
                        // When using raw sockets, we get ip header + icmp packet.
                        // the first octet of the entire packet(ip header + icmp packet) is divided into two sections.
                        // The first 4 bits of the first octet is the ip version(in ipv4's case 0100), and the low 4 bits
                        // is the length of the ip header. here we grab the low 4 bits from the first octet(by masking with 0x0f)
                        // and then multiply by 4 (<< 2) to convert bytes to bits
                        // only required when using icmpv4 raw packets. isn't needed in dgram and icmpv6 packets.
                        // let len = ((buf[0] & 0x0F) << 2) as usize; // wtf?
                        // If the packet isn't ICMP echo reply, discard it.
                        if !(buf[0] == 129 || buf[0] == 0) {
                            // parse_error(buf[0], buf[1]); // (code, type)
                            if let RingMessage::Continue((seq, _)) = m {
                                parse_error(buf[0], buf[1], seq);
                            }
                            rtx.2 += 1;
                            continue;
                        }

                        let ttl = buf[8];
                        let seq = (buf[6] as u16) << 8 | (buf[7] as u16);
                        if buf[0] == 0 && !check_checksum(&mut buf[..i]) {
                            rtx.1 += 1;
                        } else {
                            if !opts.quite {
                                // TODO: fix ttl
                                println!(
                        "\x1b[1;32m{} bytes \x1b[37mreturned. \x1b[1;32mICMP Sequence Packet:\x1b[1;37m {}, \x1b[1;32mTTL: \x1b[1;37m{}, \x1b[32mTime: \x1b[1;37m{} ms\x1b[0m", i-8, seq, ttl, time
                            );
                            }
                            rtx.0 += 1;
                        }
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
    rtx
}

pub fn run(opts: RingOptions, dest: SocketAddr) -> Result<(), RingError> {
    let socket = opts.socket.try_clone()?;
    let (tx, rx) = channel::<RingMessage>();

    let mut echo = EchoICMP::new();

    // TEST: change
    // This bit seems extremely hacky. I don't want to introduce new dependency for MPMC channel, as
    // the std MPSC channel is not suitable for the task below.
    //Also, use Condvar?
    let cont = Arc::new(AtomicBool::new(true));
    // Weirdly, you have to clone the `cont` variabel here. If cloned inside the `FnMut`, the compiler shouts
    let cont_send = cont.clone();

    // Condvar! YAY!
    let pcond = Arc::new((Mutex::new(false), Condvar::new()));
    let scond = pcond.clone();

    let recv_socket = socket.try_clone()?;
    socket.set_nonblocking(true)?; // IMPORTANT

    // Use a mut array of u8, so increasing the `seq_num` doesn't require creating a whole new copy of
    // bytes.
    let mut packet: [u8; DATA_LENGTH] = [0; DATA_LENGTH];
    let ip = if !dest.is_ipv6() {
        echo = EchoICMP::new_v4();
        4u8
    } else {
        6u8
    };
    let interval = opts.interval;
    let mut loop_time = opts.count as i64;

    echo.init_bytes(&mut packet);
    echo.increase_seq(&mut packet);
    // seq 1
    echo.update_bytes(&mut packet);
    let handle = thread::spawn(move || handle_returned(rx, recv_socket, &opts));

    let tx_clone = tx.clone();
    // The ctrlc crate takes a FnMut as an argument. We use Arc to store and load a boolen value to
    // determine when to stop running the loop
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
        loop_time -= 1;
        let time = time::Instant::now();
        socket.send(&packet)?;
        if tx
            .send(RingMessage::Continue((echo.seq_num, time)))
            .is_err()
        {
            return Err(RingError::ChannelSendError);
        };
        stats.packet_sent += 1;
        echo.increase_seq(&mut packet);
        if ip == 4 {
            echo.update_bytes(&mut packet);
        }
        let mut lock = lock.lock().unwrap();
        let res = cond
            .wait_timeout(lock, time::Duration::from_secs(interval))
            .unwrap();
        lock = res.0;
        if *lock || (loop_time == 0) {
            if let Err(e) = tx.send(RingMessage::Stop) {
                eprintln!("{}", e);
                return Err(RingError::ChannelSendError);
            }
            drop(tx);
            break;
        }
    }
    // Add code here to give the diagnostics result of all the times Pinged.
    let (success, loss, discard) = match handle.join() {
        Ok((s, l, d)) => (s, l, d),
        Err(_) => (0, 0, 0),
    };
    if stats.packet_sent == 0 {
        stats.packet_sent = 1;
    }
    stats.packet_sent -= discard;
    if success > stats.packet_sent {
        stats.packet_sent += 1;
    }
    stats.loss = loss + (stats.packet_sent - success);
    if stats.loss > stats.packet_sent {
        stats.packet_sent = stats.loss;
    }
    stats.successful = success;
    println!("\n\x1b[1;32m------------Ring Stats------------\x1b[0m");
    println!(
        "\n\x1b[1;32mRinged!\x1b[0m Received \x1b[1;32m{} packets\x1b[0m of  \x1b[1;32m{} total packets,\x1b[0m with \x1b[1;31m{}% loss!\x1b[0m Pinged for \x1b[1;32m{} seconds\x1b[0m.",
        stats.successful,
        stats.packet_sent,
        ((stats.loss * 100) / stats.packet_sent),
        stats.time.elapsed().as_secs()
    );

    // Free Up the socket just in case
    socket.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}
