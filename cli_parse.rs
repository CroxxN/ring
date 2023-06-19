use error::{ErrorSource::*, RingError};
use std::env;

pub fn get_url_from_args(arg: Vec<String>) -> Result<&'a str, RingError> {
    if arg.len() == 2 {
        Ok(&arg[1])
    } else {
        Err(RingError { source: ArgError })
    }
}
