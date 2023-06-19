use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub struct RingError {
    pub source: ErrorSource,
}

impl Display for RingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An \x1b[1;31mError\x1b[0m Occured")
    }
}

impl Error for RingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
pub enum ErrorSource {
    ArgError,
    NetworkError,
    ByteParseError,
}

impl Display for ErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_owned() {
            ErrorSource::ArgError => {
                write!(f, "\x1b[1;31mInvalid Number of Arguments\x1b[0m")
            }
            ErrorSource::NetworkError => {
                write!(f, "\x1b[1;31mNetwork Error Occured\x1b[0md")
            }
            ErrorSource::ByteParseError => {
                write!(f, "\x1b[1;31mError Occured While Parsing bytes\x1b[0md")
            }
        }
    }
}

impl Error for ErrorSource {}
