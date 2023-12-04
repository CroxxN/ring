mod error;
mod ring_impl;
use error::RingError;
use getopts::Options;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{env, net::ToSocketAddrs};
mod iputils;

// TODO: Build the help message, with colors too
const HELP_LONG: &str = "

\x1b[1;32mSend ICMP Echo Request to hosts\x1b[0m

Options:
-h, --help        Print this help message
-v, --version     Print current version
-4, --ipv4        Ring IPV4 host
-6, --ipv6        Ring IPV6 host
-b, --broadcast   Enable ringing broadcast address
-c, --count       Ring <n> times
-i, --interval    Ring every <n> seconds
-d, --timeout     Wait atmost <n> seconds for echo replies
-q, --quiet       Don't print intermediate ring results
-t, --ttl         Set time-to-live value

Arguments:
    \x1b[1;31m<destination_host>\x1b[0m

See ring(1).";

// -a, --adaptive    Adaptive ring [comming soon]
// -f, --flood       Flood ring [comming soon]

const VERSION: &str = "0.2";

pub(crate) const DATA: &[u8; 21] = b"SWIKISSSWIKISSSWIKISS"; // sweetkiss
pub(crate) const DATA_LENGTH: usize = 8 + DATA.len(); // fixed 8 bytes data field

struct RingOptions {
    socket: Socket,
    count: u32,
    ttl: u32,
    interval: u64,
    timeout: u128,
    quite: bool,
    broadcast: bool,
    addr: String,
}

#[derive(PartialEq, Debug, Eq, Clone, Copy)]
pub(crate) enum IP {
    V4,
    V6,
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
            socket: Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6))?,
            count: 0,
            ttl: 128,
            interval: 1,
            timeout: 1000,
            quite: false,
            broadcast: false,
            addr: String::new(),
        })
    }
    fn new_ip4() -> Result<Self, RingError> {
        Ok(Self {
            socket: Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))?,
            count: 0,
            ttl: 128,
            interval: 1,
            timeout: 1000,
            quite: false,
            broadcast: false,
            addr: String::new(),
        })
    }
    fn set_count(&mut self, count: u32) {
        self.count = count;
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
            self.socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))?;
            self.set_ttl(self.ttl)?;
            self.socket.set_broadcast(self.broadcast)?;
        }
        Ok(())
    }
}
// TODO: Add more cli options like choosing between IP modes
// and number of pings

// Utility to print the help screen
fn print_help(pname: &str) {
    println!(
        "\x1b[1;32mUsage:\x1b[0m {} \x1b[1;33m[options]\x1b[0m <destination>",
        pname
    );
    println!("{}", HELP_LONG);
}

fn print_help_brief(pname: &str) {
    println!("Usage:\n{} [options] <destination_address>", pname);
}

fn print_version(pname: &str) {
    println!("\x1b[1;33m{}: Version {}\x1b[0m", pname, VERSION);
}

fn main() -> Result<(), RingError> {
    let args: Vec<String> = env::args().collect();
    let pname = args[0].clone();
    let pname = pname.as_str();

    // Build the options of the ring utility
    let mut opts = Options::new();
    // TODO: Support more options?

    // A optional, no-argument option
    opts.optflag("4", "ipv4", "Ring a IPV4 address");
    opts.optflag("6", "ipv6", "Ring a IPV6 address");
    opts.optflag(
        "q",
        "quite",
        "Ring quitely without printing intermediate ping results",
    );
    opts.optflag("h", "help", "Print this help message");
    opts.optflag("v", "version", "Print current Ring version");
    opts.optflag("b", "broadcast", "Enable ringing broadcast address");

    // A optional, argument option
    opts.optflagopt("c", "count", "Stop ringing after <count> times", "<COUNT>");
    opts.optflagopt("t", "ttl", "Set time-to-live value", "ring -t<n> <dest>");
    opts.optflagopt(
        "i",
        "interval",
        "Wait <n> seconds before each echo request",
        "ring -i2 <destination>",
    );
    opts.optflagopt(
        "d",
        "timeout",
        "Wait <n> seconds for echo reply message",
        "ring -d2 <destination>",
    );

    let matches = if let Ok(m) = opts.parse(&args[1..]) {
        m
    } else {
        eprintln!("Failed to parse command-line arguments");
        return Err(RingError::ArgError);
    };

    let mut opt;
    if matches.opt_present("h") {
        print_help(pname);
        return Ok(());
    };
    if matches.opt_present("v") {
        print_version(pname);
        return Ok(());
    }

    let ip = if matches.opt_present("4") {
        opt = RingOptions::new_ip4().unwrap();
        Some(IP::V4)
    } else if matches.opt_present("6") {
        opt = RingOptions::new().unwrap();
        Some(IP::V6)
    } else {
        opt = RingOptions::new().unwrap();
        None
    };
    if matches.opt_present("q") {
        opt.set_quite(true);
    }
    // TODO: Maybe check and use `unwrap_or_default()`
    if let Some(c) = matches.opt_str("c") {
        opt.set_count(c.parse().unwrap_or(0));
    };

    if let Some(i) = matches.opt_str("i") {
        opt.interval = i.parse().unwrap_or(1);
    };

    if let Some(d) = matches.opt_str("d") {
        opt.timeout = d.parse().unwrap_or(1);
        opt.timeout *= 1000; // sec to millisecs
    };

    // Get the (only) positional argument
    opt.addr = if !matches.free.is_empty() {
        matches.free[0].to_owned()
    } else {
        // "RED: Missing\RED: Destination Address"
        eprintln!("\n\x1b[1;31mError: Missing destination address\x1b[0m\n");
        print_help_brief(pname);

        return Err(RingError::ArgError);
    };
    // let mut opt = if let Some(opt) = cli_actions(&args[0], matches.clone()) {
    //     opt
    // } else {
    //     eprintln!("\x1b[1;31mError: Missing Url\x1b[0");
    //     return Ok(());
    // };
    let url = if matches.free.is_empty() {
        eprintln!("Error: No url supplied");
        return Err(RingError::ArgError);
    } else {
        matches.free[0].clone()
    };
    if matches.opt_present("b") {
        if let Err(e) = opt.socket.set_broadcast(true) {
            eprintln!("\x1b[1;32mError:\x1b[0m {e}");
            return Err(RingError::NetworkError);
        }
        opt.broadcast = true;
    }
    let parsed_addr = (opt.addr.as_str(), 0).to_socket_addrs().unwrap();
    let mut addr;
    if let Some(i) = ip {
        if i == IP::V4 {
            addr = iputils::get_ip4_addr(parsed_addr.to_owned())?;
        } else {
            addr = iputils::get_ip6_addr(parsed_addr.to_owned())?;
        }
        opt.socket.connect(&SockAddr::from(addr))?;
    } else {
        addr = match iputils::get_ip6_addr(parsed_addr.to_owned()) {
            Ok(ip) => ip,
            Err(_) => {
                let ip4 = iputils::get_ip4_addr(parsed_addr.to_owned())?;
                opt.set_ipv(Some(IP::V4))?;
                // socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))?;
                ip4
            }
        };
        match opt.socket.connect(&SockAddr::from(addr)) {
            Ok(_) => {}
            Err(_) => {
                // if one fails, try everything.
                if addr.is_ipv6() {
                    addr = iputils::get_ip4_addr(parsed_addr.clone())?;
                    opt.socket.connect(&SockAddr::from(addr))?;
                } else {
                    addr = iputils::get_ip6_addr(parsed_addr.clone())?;
                    opt.socket.connect(&SockAddr::from(addr))?;
                }
            }
        }
    }
    if let Some(t) = matches.opt_str("ttl") {
        _ = opt.set_ttl(t.parse().unwrap_or(64));
    }
    println!(
     // Terminal Color(VT100) Specification form (https://chrisyeh96.github.io/2020/03/28/terminal-colors.html)
     "\n\x1b[1;32mRinging \x1b[0m\x1b[4;34m{}({})\x1b[0m \x1b[1;32mwith \x1b[1;37m{} bytes\x1b[0m\x1b[1;32m of data\x1b[0m\n",
         url, addr, DATA.len()
     );

    if let Err(e) = ring_impl::run(opt, addr) {
        eprintln!("Error: {e}");
        return Ok(());
    };
    Ok(())
}
