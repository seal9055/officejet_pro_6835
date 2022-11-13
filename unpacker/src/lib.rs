pub mod lzss;
pub mod pjl;
pub mod srecord;

/// Converts a sequence of bytes to a number by putting together the ascii value of each individual
/// byte to form a number with a given base
pub fn hex_to_ascii(bytes: &[u8], base: usize) -> (usize, usize) {
    let mut results: Vec<usize> = Vec::new();
    let mut index_advance = 0;

    for byte in bytes {
        index_advance += 1;
        if *byte >= 0x30 && *byte <= 0x39 {
            // 0x0-0x9 range
            results.push(*byte as usize - 0x30);
        } else if *byte >= 0x41 && *byte <= 0x46 {
            // Hex A-F range
            results.push(*byte as usize - 0x37);
        } else {
            break;
        }
    }

    let mut ret = 0;
    for (i, element) in results.iter().rev().enumerate() {
        ret += element * base.pow((i) as u32);
    }
    (ret, index_advance)
}

/// Convert `size` `bytes` from a sequence of bytes to a big endian number
pub fn bytes_to_int_be(bytes: &[u8], size: usize) -> usize {
    let mut result = 0usize;
    let mut count = 0;
    for &byte in bytes {
        if count == size {
            break;
        }
        result = (result << 8) | (byte as usize);
        count += 1;
    }
    result
}

/// Convert `size` `bytes` from a sequence of bytes to a little endian number
pub fn bytes_to_int_le(bytes: &[u8], size: usize) -> usize {
    let mut result = 0usize;
    let mut count = 0;
    for &byte in bytes {
        if count == size {
            break;
        }
        result |= (byte as usize) << (count * 8);
        count += 1;
    }
    result
}


