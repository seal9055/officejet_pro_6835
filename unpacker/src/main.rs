use unpacker::{
    bytes_to_int_be,
    lzss::lzss_uncompress,
    pjl::{parse_pjl, extract_bitmap},
    srecord::{parse_srecords, print_binary_record}
};

/// Start of table that is used to retrieve section details for decompression
const TABLE_START: usize = 0x68690;

/// Firmware Header
#[derive(Debug, Default)]
struct Header {
    magic: usize,
    _header_size: usize,
    page_size: usize,
    bmp_size: usize,
    load_addr: usize,
    load_size: usize,
    exec_addr: usize,
}

/// Structure that describes the segments
#[derive(Debug)]
struct Segment {
    /// Pointer to next element of linked list
    _next: usize,

    /// Segment Name
    name: String,

    /// Starting address of section
    start: usize,

    /// Size of section
    size: usize,

    /// Some options, possibly rwx bits, but doesn't quite line up
    _flags: usize,

    /// Used for intermediate loads using memcpys
    _dst: usize,
}

/// Firmware image after initial uncompression routines are completed
struct Firmware {
    header: Header,
    segments: Vec<Segment>,
    data: Vec<u8>,
}

impl Firmware {
    /// Create new empty firmware
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            segments: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Parse out the firmware header from the srecords
    pub fn parse_header(&mut self, srecords: &[u8]) {
        self.header.magic = bytes_to_int_be(&srecords[0x0..0x4], 4);
        self.header._header_size = bytes_to_int_be(&srecords[0x8..0xc], 4);
        self.header.page_size = bytes_to_int_be(&srecords[0x10..0x14], 4);
        self.header.bmp_size = bytes_to_int_be(&srecords[0x1c..0x20], 4);
        self.header.load_addr = bytes_to_int_be(&srecords[0x30..0x34], 4);
        self.header.load_size = bytes_to_int_be(&srecords[0x34..0x38], 4);
        self.header.exec_addr = bytes_to_int_be(&srecords[0x3c..0x40], 4);
        assert_eq!(self.header.magic, 0xBAD2BFED);
    }

    /// Parse out the firmware from the raw flash image
    pub fn parse_data(&mut self, data: &[u8]) {
        // Calculate the number of pages occupied by the bootsplash bmp, round up to nearest page
        let num_bmp_pages: usize = (self.header.bmp_size / self.header.page_size) + 
            if (self.header.bmp_size % self.header.page_size) > 0 { 1 } else { 0 };

        // Calculate the number of pages occupied by the firmware, round up to nearest page
        let num_firmware_pages: usize = (self.header.load_size / self.header.page_size) + 
            if (self.header.page_size % self.header.page_size) > 0 { 1 } else { 0 };

        // Get start address and end address of firmware to then extract it from data
        let start_addr = (num_bmp_pages + 1) * self.header.page_size;
        let end_addr = start_addr + (num_firmware_pages * self.header.page_size);
        self.data.extend(&data[start_addr..end_addr]);

        assert_eq!(self.data.len(), num_firmware_pages * self.header.page_size);
    }

    /// Parse out segment table from firmware
    pub fn parse_segments(&mut self) {
        // Start of the segment-table in memory
        let mut next = TABLE_START;

        while next != 0x0 {
            let mut name = Vec::new();
            let str_name;

            let name_addr = bytes_to_int_be(&self.data[next+4..], 4) - self.header.load_addr;
            let start = bytes_to_int_be(&self.data[next+8..], 4);
            let size = bytes_to_int_be(&self.data[next+12..], 4);
            let flags = bytes_to_int_be(&self.data[next+16..], 4);
            let dst = bytes_to_int_be(&self.data[next+20..], 4);

            // Parse out name
            let mut i = 0;
            while self.data[name_addr+i] != 0x0 {
                name.push(self.data[name_addr+i]);
                i+=1;
            }
            str_name = std::str::from_utf8(&name).unwrap();

            next = bytes_to_int_be(&self.data[next..], 4).checked_sub(self.header.load_addr)
                .unwrap_or(0);
            self.segments.push(Segment {
                    _next: next + self.header.load_addr,
                    name: str_name.to_string(),
                    start,
                    size,
                    _flags: flags,
                    _dst: dst,
                });
        }
    }
}

fn main() {
    let blob = std::fs::read("./init_blob.bin").unwrap();
    let raw = parse_pjl(&blob);
    let bm = extract_bitmap(&raw);
    let srecord = parse_srecords(&bm);
    let data = print_binary_record(&srecord);
    let mut firmware = Firmware::new();

    firmware.parse_header(&data);
    firmware.parse_data(&data);
    firmware.parse_segments();

    std::fs::write("./firmware", &firmware.data).unwrap();

    // TODO idk if this should be done on the original firmware, or the firmware with the 3 
    // initial data sections appended to it
    // let fw_3_data = std::fs::read("./firmware_with_3_data").unwrap();

    let _ = std::fs::remove_dir_all("segments");
    std::fs::create_dir_all("segments").unwrap();
    for segment in &firmware.segments {
        if segment.size == 0 {
            println!("Skipping {} because size==0", segment.name);
            continue
        }

        if segment.start < firmware.header.load_addr {
            println!("Skipping {} because start < load-address", segment.name);
            continue
        }

        if segment.start.checked_sub(firmware.header.load_addr).unwrap()
                .checked_add(segment.size).unwrap() > firmware.data.len() {
            println!("Skipping {} because start > data length", segment.name);
            continue
        }

        println!("Applying lzss decompression to {} segment", segment.name);

        let path: String = format!("segments/{}", segment.name).to_string().replace(".", "_");
        let data = lzss_uncompress(&firmware.data[segment.start
                                   .checked_sub(firmware.header.load_addr)
                                   .unwrap()..(segment.start - firmware.header.load_addr)
                                   .checked_add(segment.size).unwrap()]);
        std::fs::write(path, data).unwrap()
    }

    println!("Map binary base to {:0X}, with start address at {:0X}.", 
             firmware.header.load_addr, firmware.header.exec_addr);
}
