/// Different types the S-Record can take. Extracted from the byte following the 'S' while parsing
/// the data
#[derive(Debug)]
enum SRecordType {
    /// Generally S-Record types appear to be [0-9]. Type-A may be a proprietary addition made by HP
    A,

    /// Vendor specific ascii text comment, in this case used once in the seventh record with a
    /// data-field of `reflash`
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
            break;
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
        // Check the first few bytes to be `<ESC>*b`
        if bytes[index] == 0x1b && bytes[index + 1] == 0x2a && bytes[index + 2] == 0x62 {
            // Check if `m` appears
            if bytes[index + 4] == 0x6d {
                index += 2;
            } else if bytes[index + 6] == 0x6d {
                index += 4;
            }

            let (section_size, index_advance) = hex_to_ascii(&bytes[index + 3..index + 3 + 10], 10);
            index += index_advance + 3;
            combined_data.extend_from_slice(&bytes[index..index + section_size]);
            index += section_size;
        } else {
            println!(
                "{:X}: bytes[index] = {:X}, bytes[index+1] = {:X}",
                index,
                bytes[index],
                bytes[index + 1]
            );
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
            let (len, _) = hex_to_ascii(&bytes[index + 2..index + 4], 16);

            // Parse out type of this record
            let t_type = match bytes[index + 1] {
                0x41 => SRecordType::A,
                0x30 => SRecordType::Zero,
                0x33 => SRecordType::Three,
                _ => panic!("found type: {:X}", bytes[index + 1]),
            };

            records.push(SRecord {
                header: bytes[index],
                t_type,
                len,
                data: bytes[index + 4..index + 3 + (len * 2)].to_vec(),
                checksum: bytes[index + 4 + (len * 2)],
            });
            // Increment index by length*2 + new-line byte (1) + Header bytes (4)
            index += (len * 2) + 5;
        } else {
            println!(
                "{:X}: bytes[index] = 0x{:X}, bytes[index+1] = 0x{:X}",
                index,
                bytes[index],
                bytes[index + 1]
            );
            break;
        }
    }
    records
}

/// Decompress the given bytes that correspond to HP printer-firmware
fn decompress(blob: &Vec<u8>) {
    let combined_data = combine_data(&blob[0x7e..]);
    println!(
        "Successfully combined data. Size: 0x{:X}",
        combined_data.len()
    );

    std::fs::write("./bin1", &combined_data).unwrap();

    let records = parse_s_records(&combined_data[0x0..0x44afa]);
    println!("Successfully parsed {:X} records", records.len());
    println!("{:#X?}", records);
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
    // Start of the PJL
    UEL,
    // Reset the printer
    E,
    AsteriskB(u8),
    AsteriskR(u8),
}

#[derive(Debug)]
struct PJLCommand {
    command: Command,
    params: Vec<Param>,
    offset: usize,
}

fn parse_pjl(blob: &Vec<u8>) -> Vec<PJLCommand> {
    let mut compression_state = Param::compression(0);
    let mut result = vec![];
    let mut index = 0;
    let find_next = |id: &[u8]| id.iter().position(|&c| c == 0x1B).unwrap_or(id.len());
    loop {
        let offset = index;
        println!("Go to index {:X}", index);
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
        let parse = if blob[index] == b'%' {
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
                                println!("{:X?}", &rest[..len - 1]);
                                let compression_level = hex_to_ascii(&rest[..len - 1], 10).0;
                                compression_state = Param::compression(
                                    compression_level.try_into().expect("u8 compression size"),
                                );
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
                                        if let Param::compression(x) = &compression_state {
                                            params.push(compression_state.clone());
                                            match x {
                                                0 | 2 | 3 => {
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
                                                    println!(
                                                    "Cannot parse compression type at index {:X}",
                                                    index,
                                                    );
                                                    break;
                                                }
                                            }
                                        } else {
                                            println!("Compression undefined at index {:X}", index);
                                            break;
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
                _ => panic!(),
            }
        };
        result.push(parse);
    }

    result
}

fn decompress_bitmap(
    compress_type: (u8, &Command),
    blob: &Vec<u8>,
    seed_row: &mut Vec<u8>,
) -> Vec<u8> {
    println!("Decompressing type {:X}", compress_type.0);
    match compress_type {
        (0, _) => blob.to_vec(),
        (2, _) => {
            let mut index = 0;
            let mut expand = vec![];
            loop {
                if index >= blob.len() {
                    break;
                }
                let control = blob[index] as i8;
                println!("Found control {:X} at offset {:X}", control, index);
                index += 1;
                match control as i8 {
                    0..=127 => {
                        let mut literal = blob[index..index + control as usize + 1].to_vec();
                        index += literal.len();
                        expand.append(&mut literal)
                    }
                    -128 => {
                        println!("Found 1 Do nothing pattern at {}", index);
                        // Do nothing
                    }
                    -127..=-1 => {
                        let mut repeat = blob[index..index + 1].repeat(control.abs() as usize + 1);
                        index += 1;
                        expand.append(&mut repeat);
                    }
                }
            }
            expand
        }
        (3, _) => {
            let mut index = 0;
            loop {
                if index >= blob.len() {
                    break;
                }
                let control = blob[index];
                println!("Found control {:X} at offset {:X}", control, index);
                index += 1;
                let replace_count = 1 + ((control >> 5) & 0b111) as usize;
                let mut replace_offset = (control & 0b11111) as usize;
                if replace_offset == 0b11111 {
                    loop {
                        let next_byte = blob[index] as usize;
                        index += 1;
                        replace_offset = (replace_offset << 8) & next_byte;
                        if next_byte != 0xFF {
                            break;
                        }
                    }
                }
                let copy_data = &blob[index..index + replace_count];
                index += replace_count;
                for i in 0..replace_count {
                    seed_row[replace_offset + i] = copy_data[i];
                }
            }
            seed_row.clone()
        }
        _ => {
            println!("Warning: Could not decompress. Leaving as-is.");
            blob.to_vec()
        }
    }
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
                Param::compression(level) => c_type = *level,
                Param::data(x) => result.append(&mut decompress_bitmap(
                    (c_type, &part.command),
                    x,
                    &mut seed_row,
                )),
                _ => (),
            }
        }
    }
    result
}

fn main() {
    let blob = std::fs::read("./firmware_blob.bin").unwrap();
    // decompress(&blob);
    let raw = parse_pjl(&blob);
    std::fs::write("./bin1_struct", format!("{:#X?}", &raw)).unwrap();
    let bm = extract_bitmap(&raw);
    std::fs::write("./bin1", &bm).unwrap();
    // parse_s_records(&bm);
}
