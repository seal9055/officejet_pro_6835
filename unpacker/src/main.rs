/// Different types the S-Record can take. Extracted from the byte following the 'S' while parsing
/// the data
#[derive(Debug)]
enum SRecordType {
    /// Generally S-Record types appear to be [0-9]. Type-A may be a proprietary addition made by HP
    A,

    /// Initial SRecord, also
    /// Vendor specific ascii text comment, in this case used once in the seventh record with a
    /// data-field of `reflash`
    Zero,

    /// Data SRecord (32-bit)
    /// This type instructs the flash programmer to store the record data to a specified section in
    /// memory
    Three,

    /// Last SRecord (32-bit)
    Seven,
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

    /// Address field
    address: usize,

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
fn bytes_to_int_be(bytes: &[u8], size: usize) -> usize {
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
fn bytes_to_int_le(bytes: &[u8], size: usize) -> usize {
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

    loop {
        let record_cat = bytes[index];

        // Closure to find the next newline within sequence of bytes
        let find_nl = |id: &[u8]| id.iter().position(|&c| c == b'\n').unwrap_or(id.len());

        // Closure to verify the checksums of records
        let verify = |calc: &[u8], len: u8, checksum: u8| {
            let calc_add = calc.iter().fold(len as u16, |acc, &ele| acc + ele as u16);
            let calc_mask_comp = (calc_add & 0xFF) as u8 ^ 0xFF;
            assert!(checksum == calc_mask_comp);
        };

        match record_cat {
            // Check if record starts with `S` and is thus an S-Record
            0x53 => {
                // <ASCII Text>
                // S                                                    Header
                // 3                                                    Type
                // 19                                                   Length
                // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA     Data
                // 28                                                   Checksum
                // `\n`                                                 Newline


                // Parse out length field from the Record
                let (len, _) = hex_to_ascii(&bytes[index + 2..index + 4], 16);

                // Parse out type of this record
                let t_type = match bytes[index + 1] {
                    0x41 => SRecordType::A,
                    0x30 => SRecordType::Zero,
                    0x33 => SRecordType::Three,
                    0x37 => SRecordType::Seven,
                    _ => panic!("found type: {:X}", bytes[index + 1]),
                };
                let raw_type = bytes[index + 1] - 0x30;

                // Extract data fields as ascii instead of hex
                let ascii_byte_start = index + 4;
                let ascii_bytes = bytes[ascii_byte_start..ascii_byte_start + (len * 2)].chunks(2);
                let mut data = vec![];
                for ascii_byte in ascii_bytes {
                    data.push(hex_to_ascii(ascii_byte, 16).0 as u8);
                }

                // Verify checksum for the data
                assert!(data.len() == len);
                let checksum = data.pop().unwrap();
                verify(&data, len as u8, checksum);

                // Parse out addresses from S-Records
                let address_size = match raw_type {
                    0 | 1 | 5 | 9 => 2,
                    2 | 6 | 8 => 3,
                    3 | 7 => 4,
                    _ => 0,
                };
                let (address_raw, data) = data.split_at(address_size);
                let address = bytes_to_int_be(address_raw, address_size);

                records.push(SRecord {
                    header: bytes[index],
                    t_type,
                    len,
                    address,
                    data: data.to_vec(),
                    checksum,
                });
                // Increment index by length*2 + new-line byte (1) + Header bytes (4)
                index += (len * 2) + 5;
            }
            // Binary S-Record instead of Ascii S-Record
            0x30..=0x3F => {
                // <Hexdump>
                // 33                                                   Header+Type
                // 05                                                   Length
                // AA AA AA AA                                          Data
                // 28                                                   Checksum
                let raw_type = record_cat & 0xF;
                let t_type = match raw_type {
                    0x0 => SRecordType::Zero,
                    0x3 => SRecordType::Three,
                    0x7 => SRecordType::Seven,
                    0xA => SRecordType::A,
                    _ => panic!("found type: {:X}", record_cat),
                };
                let len = bytes[index + 1] as usize;
                let checksum = bytes[index + 1 + len];
                let data = &bytes[index + 2..index + 1 + len];
                verify(&data, len as u8, checksum);
                // Address size in bytes
                let address_size = match raw_type {
                    0 | 1 | 5 | 9 => 2,
                    2 | 6 | 8 => 3,
                    3 | 7 => 4,
                    _ => 0,
                };
                let (address_raw, data) = data.split_at(address_size);
                let address = bytes_to_int_be(address_raw, address_size);
                records.push(SRecord {
                    header: bytes[index],
                    t_type,
                    len,
                    address,
                    data: data.to_vec(),
                    checksum,
                });
                // Increment index by
                // length + new-line byte (1) + length byte (1)
                index += len + 2;
            }
            // Useless data, just skip past it until next record is found
            b'F' | b'P' => {
                // Skip until new-line
                let endl = find_nl(&bytes[index..]);
                println!(
                    "Skipping {} record: `{}`",
                    record_cat,
                    String::from_utf8(bytes[index..index + endl].to_vec()).unwrap(),
                );
                index += endl + 1;
            }
            // Not a valid record type
            _ => {
                println!(
                    "{:X}: type = {:X}, bytes[index] = 0x{:X}, bytes[index+1] = 0x{:X}",
                    index,
                    record_cat,
                    bytes[index],
                    bytes[index + 1]
                );
                break;
            }
        }
    }
    records
}

#[derive(Clone, Debug)]
enum Param {
    compression(u8),
    data(Vec<u8>),
    param1(usize),
    unknown(Vec<u8>),
    msg(String),
}

#[derive(Debug)]
enum Command {
    /// Start of the PJL
    UEL,

    /// Reset the printer
    E,

    /// Initialize bitmap
    AsteriskB(u8),

    /// Initialize the dictionary for the sliding window
    AsteriskR(u8),
}

#[derive(Debug)]
struct PJLCommand {
    command: Command,
    params: Vec<Param>,
    offset: usize,
}

/// Finds all the sections in the binary, and puts them together, removing the section meta-data, so
/// we are left with a binary blob that we can then do further work on
fn parse_pjl(blob: &Vec<u8>) -> Vec<PJLCommand> {
    let mut result = vec![];
    let mut index = 0;

    // Closure to find next newline in byte-array
    let find_next = |id: &[u8]| id.iter().position(|&c| c == 0x1B).unwrap_or(id.len());

    loop {
        let offset = index;
        //println!("Go to index {:X}", index);
        // First element in command is <ESC>
        if index >= blob.len() {
            println!("Finished");
            break;
        }
        if blob[index] != 0x1B {
            println!("PJL Command header mismatch");
            break;
        }
        index += 1;
        let parse: PJLCommand = if blob[index] == b'%' {
            if &blob[index..index + 8] == b"%-12345X" {
                let endl = find_next(&blob[index..]);
                let msg = String::from_utf8(blob[index..index + endl].to_vec())
                    .expect("Message following command");
                index += endl;
                PJLCommand {
                    command: Command::UEL,
                    params: vec![Param::msg(msg)],
                    offset,
                }
            } else {
                println!("UEL mismatch at index {}", index);
                break;
            }
        } else {
            let cmd_len = 1 + blob[index..]
                .iter()
                .position(|&c| matches!(c, b'A'..=b'Z'))
                .unwrap();
            let cmdline = &blob[index..cmd_len + index];
            let extra = &blob[cmd_len + index..];
            index += cmd_len;
            match cmdline[0] {
                b'E' => {
                    let endl = find_next(&blob[index..]);
                    let msg = String::from_utf8(blob[index..index + endl].to_vec())
                        .expect("Message following command");
                    index += endl;
                    PJLCommand {
                        command: Command::E,
                        params: vec![Param::msg(msg)],
                        offset,
                    }
                }
                b'*' => {
                    let cmd_name = cmdline[1];
                    let method = cmdline[cmd_len - 1];
                    let mut params = vec![];
                    // Parse possible param
                    for rest in cmdline[2..cmd_len - 1].split_inclusive(|x| x.is_ascii_lowercase())
                    {
                        let len = rest.len();
                        match rest[len - 1] {
                            b'm' => {
                                // println!("{:X?}", &rest[..len - 1]);
                                if len > 0 {
                                    let compression_level = hex_to_ascii(&rest[..len - 1], 10).0;
                                    params.push(Param::compression(
                                        compression_level.try_into().expect("u8 compression size"),
                                    ));
                                }
                            }
                            b'0'..=b'9' => {
                                let length = hex_to_ascii(rest, 10).0;
                                params.push(Param::param1(length));
                            }
                            _ => {
                                println!("Skipped parameter {:X?}", rest);
                            }
                        }
                    }

                    match cmd_name {
                        b'r' => {
                            let command = Command::AsteriskR(method);
                            PJLCommand {
                                command,
                                params,
                                offset,
                            }
                        }
                        b'b' => {
                            let command = Command::AsteriskB(method);
                            if let Some(&Param::param1(read_length)) =
                                params.iter().find(|p| matches!(p, Param::param1(_)))
                            {
                                match method {
                                    b'V' | b'W' => {
                                        let stack = &extra[..read_length];
                                        index += read_length;
                                        params.push(Param::data(stack.to_vec()));
                                        PJLCommand {
                                            command,
                                            params,
                                            offset,
                                        }
                                    }
                                    _ => {
                                        let endl = find_next(&blob[index..]);
                                        println!(
                                            "Unknown command at index {:X?}-{:X?}",
                                            index,
                                            index + endl
                                        );
                                        index += endl;
                                        PJLCommand {
                                            command,
                                            params,
                                            offset,
                                        }
                                    }
                                }
                            } else {
                                println!("Length info not found at index {:X}", index);
                                break;
                            }
                        }
                        _ => {
                            println!(
                                "Cannot parse command *{:X} method {:X} at index {:X}",
                                cmd_name, method, index
                            );
                            break;
                        }
                    }
                }
                _ => panic!("Unimplemented command"),
            }
        };
        result.push(parse);
    }

    result
}

fn decompress_bitmap(compress_type: (u8, &Command), blob: &Vec<u8>, seed_row: &Vec<u8>) -> Vec<u8> {
    println!("INFO: Decompressing type {:X}", compress_type.0);
    match compress_type {
        (0, _) => {
            let mut expand = blob.to_vec();
            if matches!(compress_type.1, Command::AsteriskB(b'V')) && blob.len() != 16384 {
                expand.resize(16384, 0);
            }
            expand
        }
        (2, _) => {
            let mut index = 0;
            let mut expand = vec![];
            loop {
                if index >= blob.len() {
                    assert!(index == blob.len());
                    break;
                }
                let control = blob[index] as i8;
                // println!("Found control {:X} at offset {:X}", control, index);
                index += 1;
                match control {
                    0 => {
                        let next = blob[index];
                        index += 1;
                        expand.push(next);
                    }
                    1..=127 => {
                        let mut literal = blob[index..index + control as usize + 1].to_vec();
                        index += literal.len();
                        expand.append(&mut literal);
                    }
                    -128 => {
                        println!("Found Do nothing pattern at {}", index);
                        // Do nothing
                    }
                    -127..=-1 => {
                        let mut repeat = blob[index..index + 1].repeat(control.abs() as usize + 1);
                        index += 1;
                        expand.append(&mut repeat);
                    }
                }
            }
            // the command Transfer Raster Data by Plane (‘V’) is zero-filled
            // if the amount of bytes after decompression is less than the raster width,
            // while the Transfer Raster Data by Row (‘W’) is not zero-filled
            if matches!(compress_type.1, Command::AsteriskB(b'V')) && expand.len() != 16384 {
                expand.resize(16384, 0);
            }
            expand
        }
        (3, _) => {
            let mut index = 0;
            let mut position = 0;
            let mut seed_row = seed_row.to_vec();
            loop {
                if index >= blob.len() {
                    assert!(index == blob.len());
                    break;
                }
                let control = blob[index];
                // println!("Found control {:X} at offset {:X}", control, index);
                index += 1;
                let replace_count = 1 + ((control >> 5) & 0b111) as usize;
                let mut replace_offset = (control & 0b11111) as usize;
                if replace_offset == 0b11111 {
                    loop {
                        let next_byte = blob[index] as usize;
                        index += 1;
                        replace_offset += next_byte;
                        if next_byte != 0xFF {
                            break;
                        }
                    }
                }
                position += replace_offset;
                let copy_data = &blob[index..index + replace_count];
                index += replace_count;
                for i in 0..replace_count {
                    seed_row[position + i] = copy_data[i];
                }
                position += replace_count;
            }
            seed_row
        }
        _ => {
            println!("Warning: Could not decompress. Leaving as-is.");
            blob.to_vec()
        }
    }
}

fn print_binary_record(record: &Vec<SRecord>) -> Vec<u8> {
    // For now, printing binary part only
    record
        .iter()
        .skip_while(|rec| rec.header != 0x30)
        .filter(|rec| matches!(rec.t_type, SRecordType::Three))
        .map(|rec| rec.data.clone())
        .collect::<Vec<_>>()
        .concat()
        // Removing unused OOB data
        .chunks(0x840)
        .map(|chunk| &chunk[..0x800])
        .collect::<Vec<_>>()
        .concat()
}

fn extract_bitmap(pjls: &Vec<PJLCommand>) -> Vec<u8> {
    let mut result = vec![];
    let mut seed_row = vec![0u8; 16384];
    let start = pjls
        .iter()
        .position(|x| matches!(x.command, Command::AsteriskR(b'A')))
        .expect("Bitmap start");
    let end = pjls
        .iter()
        .position(|x| matches!(x.command, Command::AsteriskR(b'C')))
        .expect("Bitmap end");

    let mut c_type = 0;
    for part in &pjls[start + 1..end] {
        println!("Decompressing at {:X}", part.offset);
        for param in part.params.iter() {
            match param {
                Param::compression(level) => {
                    c_type = *level;
                    println!("Compression switched to {}", c_type);
                }
                Param::data(x) => {
                    seed_row = decompress_bitmap((c_type, &part.command), x, &seed_row);
                    result.append(&mut seed_row.to_vec());
                }
                _ => {}
            }
        }
    }
    result
}

#[derive(Debug)]
struct Header {
    magic: u32,
    header_size: u32,
    page_size: u32,
    bmp_size: u32,
    load_addr: u32,
    load_size: u32,
    exec_addr: u32,
}

fn parse_headers(srecords: &[u8]) -> Header {
    Header {
        magic: bytes_to_int_be(&srecords[0x0..0x4], 4) as u32,
        header_size: bytes_to_int_be(&srecords[0x8..0xc], 4) as u32,
        page_size: bytes_to_int_be(&srecords[0x10..0x14], 4) as u32,
        bmp_size: bytes_to_int_be(&srecords[0x1c..0x20], 4) as u32,
        load_addr: bytes_to_int_be(&srecords[0x30..0x34], 4) as u32,
        load_size: bytes_to_int_be(&srecords[0x34..0x38], 4) as u32,
        exec_addr: bytes_to_int_be(&srecords[0x3c..0x40], 4) as u32,
    }
}

/// Parse out the firmware from the raw flash image
fn parse_firmware(header: &Header, data: &[u8]) -> Vec<u8> {
    // Calculate the number of pages occupied by the bootsplash bmp, round up to nearest page
    let num_bmp_pages: usize = (header.bmp_size / header.page_size) as usize + 
        if (header.bmp_size % header.page_size) > 0 { 1 } else { 0 };

    // Calculate the number of pages occupied by the firmware, round up to nearest page
    let num_firmware_pages: usize = (header.load_size / header.page_size) as usize + 
        if (header.page_size % header.page_size) > 0 { 1 } else { 0 };

    // Get start address and end address of firmware to then extract it from data
    let start_addr = (num_bmp_pages + 1) * header.page_size as usize;
    let end_addr = start_addr + (num_firmware_pages * header.page_size as usize);
    let firmware_data = &data[start_addr..end_addr];

    assert_eq!(firmware_data.len(), num_firmware_pages * header.page_size as usize);

    firmware_data.to_vec()
}

fn main() {
    let blob = std::fs::read("./init_blob.bin").unwrap();
    let raw = parse_pjl(&blob);
    let bm = extract_bitmap(&raw);
    let srecord = parse_s_records(&bm);
    let data = print_binary_record(&srecord);

    let header = parse_headers(&data);
    assert_eq!(header.magic, 0xBAD2BFED);

    let firmware = parse_firmware(&header, &data);

    std::fs::write("./firmware", firmware).unwrap();

    println!("{:#0X?}", header);
    println!("Map binary base to 0x26710000, with start address at 0x27FFC118.");
}
