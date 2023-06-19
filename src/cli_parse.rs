use std::env;
pub mod error;
use error::{ErrorSource::*, RingError};

fn get_url_from_args() -> Result<&'a str, RingError> {
    if arg.len() == 2 {
        Ok(&arg[1])
    } else {
        Err(RingError { source: ArgError })
    }
}
