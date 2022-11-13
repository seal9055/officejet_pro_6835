use crate::{
    hex_to_ascii, bytes_to_int_be,
};

/// Different types the S-Record can take. Extracted from the byte following the 'S' while parsing
/// the data
#[derive(Debug)]
pub enum SRecordType {
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
pub struct SRecord {
    /// Header, this should always be ascii for 'S'
    header: u8,

    /// Type of this record
    t_type: SRecordType,

    /// Length of data field
    _len: usize,

    /// Address field
    _address: usize,

    /// Data field
    data: Vec<u8>,

    /// Sum all bytes (% 256) starting at len field and take 1's complement
    _checksum: u8,
}

/// Parse out all S-Records from the passed in bytes and return them to user
pub fn parse_srecords(bytes: &[u8]) -> Vec<SRecord> {
    let mut index: usize = 0;
    let mut records: Vec<SRecord> = Vec::new();

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
                    _len: len,
                    _address: address,
                    data: data.to_vec(),
                    _checksum: checksum,
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
                    _len: len,
                    _address: address,
                    data: data.to_vec(),
                    _checksum: checksum,
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

/// Return only the binary sections of the srecords
pub fn print_binary_record(record: &Vec<SRecord>) -> Vec<u8> {
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
