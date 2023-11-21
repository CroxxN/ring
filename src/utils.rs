pub(crate) fn calculate_chksm_fast(init: u32, bytes: &mut [u8]) -> u16 {
    let mut sum = init;
    // for word in bytes.chunks(2) {
    //     let mut part = u16::from(word[0]) << 8;
    //     if word.len() > 1 {
    //         part += u16::from(word[1]);
    //     }
    //     sum = sum.wrapping_add(u32::from(part));
    // }
    // let temp: u32 = u32::from(bytes[6] << 8) + u32::from(bytes[7]);
    // sum += !temp as u32;
    // while (sum >> 16) > 0 {
    //     sum = (sum & 0xffff) + (sum >> 16);
    // }

    !sum as u16
}

// initial checksum calculation
pub(crate) fn calc_checksum_g(init_check: u32, bytes: &mut [u8], container: Option<&mut u16>) {
    bytes[2] = 0;
    bytes[3] = 0;
    let mut sum = init_check;
    for word in bytes.chunks(2) {
        let mut part = u16::from(word[0]) << 8;
        if word.len() > 1 {
            part += u16::from(word[1]);
        }
        sum = sum.wrapping_add(u32::from(part));
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    let sum = !sum as u16;
    // If a container is provided, return the checksum through the container
    if let Some(c) = container {
        *c = sum;
    }
    // If not, just append it to the original data buffer
    else {
        bytes[2] = (sum >> 8) as u8;
        bytes[3] = (sum & 0xff) as u8;
    }
}
