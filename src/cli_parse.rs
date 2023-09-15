// TODO: Remove these ignored warnings
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use crate::error::RingError;
use getopts::{Matches, Options};
use std::env;

// TODO: Add more cli options like choosing between IP modes
// and number of pings

// TODO: Build the help message, with colors too
const HELP: &'static str = "TODO";

// Utility to print the help screen
fn print_help(pname: &str) {
    println!("{}: Usage\n", pname);
    println!("{}", HELP);
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
    opts.optflagopt("D", "timestamp", "Print timestamp", "<TIMESTAMP>");

    // The destination is the only positional argument, so we don't designate a option flag
    opts
}

// Doesn't return anything because it handles all errors by displaying the help message.
fn cli_actions(pname: &str, matches: Matches) -> Option<String> {
    if matches.opt_present("h") {
        print_help(pname);
        return None;
    };
    // Get the (only) positional argument
    let addr = if !matches.free.is_empty() {
        matches.free[0].to_owned()
    } else {
        // TODO: Print specific message with colors like "RED: Missing\RED: Destination Address"
        print_help(pname);
        return None;
    };
    Some(addr)
    // if let Some(addr) = matches.free {
    //     return Some(addr);
    //     // TODO: call the ping util. IMP: should come last
    // } else {
    //     print_help(pname);
    //     return None;
    // }
}

// TODO: Change the function signature to not accept anything
// TODO: Convert to main function & don't return anything
pub fn get_args(args: Vec<String>) -> Result<String, RingError> {
    // let args: Vec<String> = env::args().collect();
    let opts = build_options();
    let matches = if let Ok(m) = opts.parse(&args[1..]) {
        m
    } else {
        eprintln!("Failed to parse command-line arguments");
        return Err(RingError::ArgError);
    };

    if let Some(addr) = cli_actions(&args[0], matches) {
        Ok(addr)
    } else {
        Err(RingError::ArgError)
    }
}
