use crate::hex_to_ascii;

/// Various parameter types that can be passed to pjl commands
#[derive(Clone, Debug)]
pub enum Param {
    Compression(u8),
    Data(Vec<u8>),
    Param1(usize),
    Unknown(Vec<u8>),
    Msg(String),
}

/// Different types of valid PJL Commands
#[derive(Debug)]
pub enum Command {
    /// Start of the PJL
    UEL,

    /// Reset the printer
    E,

    /// Initialize bitmap
    AsteriskB(u8),

    /// Initialize the dictionary for the sliding window
    AsteriskR(u8),
}

/// A given printer job language command
#[derive(Debug)]
pub struct PJLCommand {
    command: Command,
    params: Vec<Param>,
    _offset: usize,
}

/// Finds all the sections in the binary, and puts them together, removing the section meta-data, so
/// we are left with a binary blob that we can then do further work on
pub fn parse_pjl(blob: &Vec<u8>) -> Vec<PJLCommand> {
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
                    params: vec![Param::Msg(msg)],
                    _offset: offset,
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
                        params: vec![Param::Msg(msg)],
                        _offset: offset,
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
                                    params.push(Param::Compression(
                                        compression_level.try_into().expect("u8 compression size"),
                                    ));
                                }
                            }
                            b'0'..=b'9' => {
                                let length = hex_to_ascii(rest, 10).0;
                                params.push(Param::Param1(length));
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
                                _offset: offset,
                            }
                        }
                        b'b' => {
                            let command = Command::AsteriskB(method);
                            if let Some(&Param::Param1(read_length)) =
                                params.iter().find(|p| matches!(p, Param::Param1(_)))
                            {
                                match method {
                                    b'V' | b'W' => {
                                        let stack = &extra[..read_length];
                                        index += read_length;
                                        params.push(Param::Data(stack.to_vec()));
                                        PJLCommand {
                                            command,
                                            params,
                                            _offset: offset,
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
                                            _offset: offset,
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

/// Decompress pjl bitmap
pub fn decompress_bitmap(compress_type: (u8, &Command), blob: &Vec<u8>, seed_row: &Vec<u8>) 
        -> Vec<u8> {
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

/// Extract the bitmap from the pjl commands and decompress it
pub fn extract_bitmap(pjls: &Vec<PJLCommand>) -> Vec<u8> {
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
        //println!("Decompressing at {:X}", part.offset);
        for param in part.params.iter() {
            match param {
                Param::Compression(level) => {
                    c_type = *level;
                    println!("Compression switched to {}", c_type);
                }
                Param::Data(x) => {
                    seed_row = decompress_bitmap((c_type, &part.command), x, &seed_row);
                    result.append(&mut seed_row.to_vec());
                }
                _ => {}
            }
        }
    }
    result
}


