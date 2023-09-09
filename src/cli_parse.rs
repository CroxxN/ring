use crate::error::RingError;

// TODO: Add more cli options like choosing between IP modes
// and number of pings

pub fn get_args(arg: Vec<String>) -> Result<String, RingError> {
    // if arg.len() == 2 {
    //     Ok(&arg[1])
    // } else {
    //     Err(RingError { source: ArgError })
    // }
    match arg.len() {
        2 => Ok(arg[1].to_owned()),
        _ => Err(RingError::ArgError),
    }
}
