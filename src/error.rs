use std::error::Error;
use std::fmt::Display;
use std::io;

#[derive(Debug)]
pub enum RingError {
    ArgError,
    IoError(io::Error),
    NetworkError,
    ByteParseError,
}

// impl From

impl From<io::Error> for RingError {
    fn from(value: io::Error) -> Self {
        RingError::IoError(value)
    }
}

impl Display for RingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_owned() {
            RingError::ArgError => {
                write!(f, "\x1b[1;31mInvalid Number of Arguments\x1b[0m")
            }
            RingError::IoError(e) => {
                write!(f, "\x1b[1;31m{e}\x1b[0md")
            }
            RingError::NetworkError => {
                write!(f, "\x1b[1;31mNetwork Error Occured\x1b[0md")
            }
            RingError::ByteParseError => {
                write!(f, "\x1b[1;31mError Occured While Parsing bytes\x1b[0md")
            }
        }
    }
}

impl Error for RingError {}
