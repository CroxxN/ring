// TODO: Remove these ignored warnings
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod error;
mod ring_impl;
use ctrlc::Error;
use error::RingError;
use getopts::{Matches, Options};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{
    env,
    net::{SocketAddr, SocketAddrV4, ToSocketAddrs},
    str::FromStr,
};
mod iputils;

const VERSION: &'static str = "0.1";

struct RingOptions {
    socket: Socket,
    count: u32,
    ip: IP,
    ttl: u32,
    quite: bool,
    addr: String,
}

#[derive(PartialEq)]
enum IP {
    V4,
    V6,
}

trait Ip {}

impl Ip for IP {}

fn ip4_socket(url: &str, ip_type: IP) -> Result<SocketAddr, RingError> {
    let mut parsed_socket_vec = (url, 0).to_socket_addrs()?.into_iter();
    // let addr = parsed_socket_vec.into_iter().try_for_each(|a| {
    //     if a.is_ipv4() {
    //         return std::ops::ControlFlow::Break(a);
    //     }
    //     std::ops::ControlFlow::Continue(())
    // });
    // // Use std::ops::ControlFlow::Break().break_value() when it stabalizes
    // if let std::ops::ControlFlow::Break(s) = addr {
    //     return Ok(s);
    // } else {
    //     return Err(RingError::NetworkError);
    // }
    if let Some(a) = parsed_socket_vec.next() {
        match ip_type {
            IP::V4 => {
                if a.is_ipv4() {
                    return Ok(a);
                } else {
                    return parsed_socket_vec.next_back().ok_or(RingError::NetworkError);
                }
            }
            IP::V6 => {
                if a.is_ipv6() {
                    return Ok(a);
                } else {
                    return parsed_socket_vec.next_back().ok_or(RingError::NetworkError);
                }
            }
        }
    } else {
        return Err(RingError::NetworkError);
    }
}

impl From<&str> for IP {
    fn from(value: &str) -> Self {
        if value == "4" {
            Self::V4
        } else {
            Self::V6
        }
    }
}

impl RingOptions {
    fn new() -> Result<Self, RingError> {
        Ok(Self {
            // socket: Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?,
            socket: Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?,
            count: 0,
            ip: IP::V4,
            ttl: 64,
            quite: false,
            addr: String::new(),
        })
    }
    fn set_count(&mut self, count: u32) {
        self.count = count;
    }
    fn set_ip(&mut self, ip: &str) {
        self.ip = IP::from(ip);
    }
    fn set_ttl(&mut self, ttl: u32) -> Result<(), RingError> {
        self.socket.set_ttl(ttl)?;
        self.ttl = ttl;
        Ok(())
    }
    fn set_quite(&mut self, quite: bool) {
        self.quite = quite;
    }
    fn set_ipv(&mut self, ip: Option<IP>) -> Result<(), RingError> {
        if ip == Some(IP::V4) {
            self.socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
        }
        Ok(())
    }
    fn get_socket(&self) -> &Socket {
        &self.socket
    }
}
// TODO: Add more cli options like choosing between IP modes
// and number of pings

// TODO: Build the help message, with colors too
const HELP_LONG: &'static str = "TODO";

// Utility to print the help screen
fn print_help(pname: &str) {
    println!("{}: Usage\n", pname);
    println!("{}", HELP_LONG);
}

fn print_help_brief(pname: &str) {
    println!("Usage: {} <destination_address>", pname);
}

fn print_version(pname: &str) {
    println!("\n\x1b[1;33m{}: Version {}\x1b[0m", pname, VERSION);
}
// Build the options of the ring utility
// TODO: Support more options?
fn build_options() -> Options {
    let mut opts = Options::new();

    // A optional, no-argument option
    opts.optflag("4", "ipv4", "Ring a IPV4 address");
    opts.optflag("6", "ipv6", "Ring a IPV6 address");
    opts.optflag(
        "q",
        "quite",
        "Ring quitely without printing intermediate ping results",
    );
    opts.optflag("h", "help", "Print this help message");
    opts.optflag("V", "version", "Print current Ring version");

    // A optional, argument option
    opts.optflagopt("c", "count", "Stop ringing after <count> times", "<COUNT>");
    opts.optflagopt("t", "ttl", "Set time-to-live value", "<ttl>");

    // The destination is the only positional argument, so we don't designate a option flag
    opts
}

// Doesn't return anything because it handles all errors by displaying the help message.
fn cli_actions(pname: &str, matches: Matches) -> Option<RingOptions> {
    // TODO: remove the unwrap
    let mut cli_options = RingOptions::new().unwrap();
    if matches.opt_present("h") {
        print_help(pname);
        return None;
    };
    if matches.opt_present("V") {
        print_version(pname);
        return None;
    }
    // TODO: Maybe check and use `unwrap_or_default()`
    if let Some(c) = matches.opt_str("c") {
        cli_options.set_count(c.parse().unwrap_or(0));
    };
    if let Some(t) = matches.opt_str("ttl") {
        _ = cli_options.set_ttl(t.parse().unwrap_or(64));
    }
    if matches.opt_present("q") {
        cli_options.set_quite(true);
    }

    // Get the (only) positional argument
    cli_options.addr = if !matches.free.is_empty() {
        matches.free[0].to_owned()
    } else {
        // "RED: Missing\RED: Destination Address"
        eprintln!("\n\x1b[1;31mError: Missing destination address\x1b[0m\n");
        print_help_brief(pname);

        return None;
    };
    Some(cli_options)
}

fn main() -> Result<(), RingError> {
    let args: Vec<String> = env::args().collect();
    let opts = build_options();
    let matches = if let Ok(m) = opts.parse(&args[1..]) {
        m
    } else {
        eprintln!("Failed to parse command-line arguments");
        return Ok(());
    };

    let opt = if let Some(opt) = cli_actions(&args[0], matches) {
        opt
    } else {
        eprintln!("\x1b[1;31mError: Missing Url\x1b[0");
        return Ok(());
    };
    let socket = opt.get_socket();
    let parsed_addr = (opt.addr.as_str(), 0).to_socket_addrs().unwrap();
    let addr = iputils::ip4::get_ip4_addr(parsed_addr)?;
    match socket.connect(&SockAddr::from(addr)) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
    let address = socket.local_addr().unwrap().as_socket_ipv6().unwrap();
    dbg!(address);
    if let Err(e) = ring_impl::run(socket) {
        eprintln!("Error: {e}");
        return Ok(());
    };
    Ok(())
}

// TODO: Change the function signature to not accept anything
// TODO: Convert to main function & don't return anything
// pub fn get_args(args: Vec<String>) -> Result<String, RingError> {
//     // let args: Vec<String> = env::args().collect();
//     let opts = build_options();
//     let matches = if let Ok(m) = opts.parse(&args[1..]) {
//         m
//     } else {
//         eprintln!("Failed to parse command-line arguments");
//         return Err(RingError::ArgError);
//     };

//     if let Some(addr) = cli_actions(&args[0], matches) {
//         Ok(addr)
//     } else {
//         Err(RingError::ArgError)
//     }
//}
