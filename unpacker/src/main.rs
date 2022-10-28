
/// Different types the S-Record can take. Extracted from the byte following the 'S' while parsing 
/// the data
#[derive(Debug)]
enum SRecordType {
    /// Generally S-Record types appear to be [0-9]. Type-A may be a proprietary addition made by HP
    A,

    /// Vendor specific ascii text comment
    Zero,

    /// This type instructs the flash programmer to store the record data to a specified section in 
    /// memory
    Three,
}

/// SRecord struct
#[derive(Debug)]
struct SRecord {
    /// Header, this should always be ascii for 'S'
    header: u8,

    /// Type of this record
    t_type: SRecordType,

    /// Length of data field
    len: usize,

    /// Data field
    data: Vec<u8>,

    /// Sum all bytes (% 256) starting at len field and take 1's complement
    checksum: u8,
}

/// Converts a sequence of bytes to a number by putting together the ascii value of each individual 
/// byte to form a number with a given base
fn hex_to_ascii(bytes: &[u8], base: usize) -> (usize, usize) {
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
            break
        }
    }

    let mut ret = 0;
    for (i, element) in results.iter().rev().enumerate() {
        ret += element * base.pow((i) as u32);
    }
    (ret, index_advance)
}

/// Finds all the sections in the binary, and puts them together, removing the section meta-data, so
/// we are left with a binary blob that we can then do further work on
fn combine_data(bytes: &[u8]) -> Vec<u8> {
    let mut index: usize = 0;
    let mut combined_data: Vec<u8> = Vec::new();

    loop {
        if bytes[index] == 0x1b && bytes[index+1] == 0x2a && bytes[index+2] == 0x62 {
            if bytes[index+4] == 0x6d {
                index += 2;
            } else if bytes[index+6] == 0x6d {
                index += 4;
            }

            let (section_size, index_advance) = hex_to_ascii(&bytes[index+3..index+3+10], 10);
            index += index_advance + 3;
            combined_data.extend_from_slice(&bytes[index..index+section_size]);
            index += section_size;
        } else {
            println!("{:X}: bytes[index] = {:X}, bytes[index+1] = {:X}", 
                   index, bytes[index], bytes[index+1]);
            break;
        }
    }
    combined_data
}

/// Parse out all S-Records from the passed in bytes and return them to user
fn parse_s_records(bytes: &[u8]) -> Vec<SRecord> {
    let mut index: usize = 0;
    let mut records: Vec<SRecord> = Vec::new();

    // TODO-1 use first ~4096 bytes to parse out the window used by potential LZSS compression
    // Token-offset=2-bytes, S-Record-length is used to determine how long the token loaded 
    // from the window should be (maybe? Not sure yet)
    //
    // Split into sections of 8 bytes, each of which starts with a flag-byte
    // Flag Bytes: 0x7f, 0xfd, 0xfe, 0xff, ...?
    // bits of flag-byte correspond to next 8 bytes
    //   eg. 0xFB = 0b11111011 -> First 2 bytes are literals (11), and then comes a 2 byte token (0)

    // TODO-2 Check checksum for validity of parsed S-record

    loop {
        // Check if record starts with `S`
        if bytes[index] == 0x53 {
            let (len, _) = hex_to_ascii(&bytes[index+2..index+4], 16);

            // Parse out type of this record
            let t_type = match bytes[index+1] {
                0x41 => SRecordType::A,
                0x30 => SRecordType::Zero,
                0x33 => SRecordType::Three,
                _ => panic!("found type: {:X}", bytes[index+1]),
            };

            records.push(SRecord {
                header: bytes[index],
                t_type,
                len,
                data: bytes[index+4..index+3+(len*2)].to_vec(),
                checksum: bytes[index+4+(len*2)],
                
            });
            // Increment index by length*2 + new-line byte (1) + Header bytes (4)
            index += (len*2) + 5;
        } else {
            println!("{:X}: bytes[index] = 0x{:X}, bytes[index+1] = 0x{:X}", 
                   index, bytes[index], bytes[index+1]);
            break;
        }
    }
    records
}

/// Decompress the given bytes that correspond to HP printer-firmware
fn decompress(blob: &Vec<u8>) {
    let combined_data = combine_data(&blob[0x7e..]);
    println!("Successfully combined data. Size: 0x{:X}", combined_data.len());

    std::fs::write("./bin1", &combined_data).unwrap();

    let records = parse_s_records(&combined_data[0x0..0x44afa]); 
    println!("Successfully parsed {:X} records", records.len());
    println!("{:#X?}", records);
}

fn main() {
    let blob = std::fs::read("./firmware_blob.bin").unwrap();
    decompress(&blob);
}
