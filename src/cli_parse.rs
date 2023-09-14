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

    // A required, argument option
    opts.reqopt("", "", "Destination Ring Address", "<dest_addr>");
    opts
}

// Doesn't return anything because it handles all errors by displaying the help message.
fn cli_actions(m: Matches) {}

pub fn parse_args() -> Result<String, RingError> {
    let args: Vec<String> = env::args().collect();
    let program_name = &args[0];
    let opts = build_options();
    let matches = if let Ok(m) = opts.parse(&args[1..]) {
        m
    } else {
        eprintln!("Failed to parse command-line arguments");
        return Err(RingError::ArgError);
    };

    cli_actions(matches);
    Ok("DONE".to_owned())
}

// TODO: Remove this function once the real functionality is implemented
pub fn get_args(arg: Vec<String>) -> Result<String, RingError> {
    let arglen = arg.len();
    match arglen {
        2 => Ok(arg[1].to_owned()),
        _ => Err(RingError::ArgError),
    }
}
