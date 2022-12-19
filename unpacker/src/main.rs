use unpacker::{
    bytes_to_int_be,
    lzss::lzss_uncompress,
    pjl::{parse_pjl, extract_bitmap},
    srecord::{parse_srecords, print_binary_record}
};

/// Start of table that is used to retrieve section details for decompression
/// Finding this could be automated by instead searching memory of the firmware for the magic value
/// `0x3ca55a3c`, marking the start of the app_hdr struct, which can be used to retrieve this 
/// address
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

    pub fn dump_hardcoded(&self) {
        // Hardcoded memcpy
        { 
            let dst: usize = 0x60;
            let src: usize = 0x26710000;
            let len: usize = 0x7b0;

            let data = &self.data[src.checked_sub(self.header.load_addr).unwrap()..
                (src - self.header.load_addr).checked_add(len).unwrap()];

            let path: String = format!("segments/{:X}.dump", dst).to_string();
            std::fs::write(path, data).unwrap()
        }

        // Hardcoded uncompress
        {
            let tripples: [(usize, usize, usize);3] = [
                (0xa007cd60, 0x26724b18, 0x3cfa),
                (0x2009fd8c, 0x26722398, 0x277f),
                (0x200890c0, 0x267107b0, 0x11be7),
            ];
            for tripple in tripples {
                let dst  = tripple.0;
                let src  = tripple.1;
                let size = tripple.2;

                let data = lzss_uncompress(&self.data[src
                                           .checked_sub(self.header.load_addr).unwrap()..
                                           (src - self.header.load_addr)
                                           .checked_add(size).unwrap()]);

                let path: String = format!("segments/{:X}.dump", dst).to_string();
                std::fs::write(path, data).unwrap()
            }
        }

        // Hardcoded memsets
        {
            let tripples: [(usize, u8, usize);4] = [
                (0xa007cd20, 0, 0x40),
                (0xa0081160, 0, 0x7f58),
                (0x600788a0, 0, 0x4464),
                (0x60000000, 0, 0xc7c),
            ];

            // Dump data for memset tripples
            for tripple in &tripples {
                let dst  = tripple.0;
                let val  = tripple.1;
                let size = tripple.2;

                let data = vec![val; size];
                let path: String = format!("segments/{:X}.dump", dst).to_string();
                std::fs::write(path, data).unwrap()
            }
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct AppHeader {
    magic: usize,
    size: usize,
    _magic1: usize,
    _magic2: usize,
    _bootsplash_bmp: usize,
    entry_point: usize,
    protected_count: usize,
    protected_addr: usize,
    section_linked_list: usize,
    memset_list_start: usize,
    memset_list_end: usize,
    copy_list_start: usize,
    copy_list_end: usize,
    _copy_list_barrier: usize,
    uncompress_list_start: usize,
    uncompress_list_end: usize,
    _uncompress_list_barrier: usize,
}

#[derive(Default, Debug)]
struct BootLoader {
    /// App header used to parse out other important structures
    header: AppHeader,

    /// Each entry lists start and end addresses of a protected section. Protected sections should 
    /// not be overwritten
    protected_ranges: Vec<(usize, usize)>,

    /// dst, src, compressed_size that are passed to the uncompress section to later decompress
    uncompress_tripples: Vec<(usize, usize, usize)>,

    /// dst, src, length that are passed to the memcpy function to setup memory mappings
    memcpy_tripples: Vec<(usize, usize, usize)>,

    /// dst, val, length that are passed to the memset function to setup memory mappings
    memset_tripples: Vec<(usize, usize, usize)>,
}

const APP_HEADER_MAGIC: usize = 0x3ca55a3c;
impl BootLoader {
    /// Parse out app header structure from firmware
    pub fn parse_header(&mut self, firmware: &Firmware) -> Option<()> {
        // Find header offset
        let mut index = 0;
        let mut app_hdr_index: Option<usize> = None;

        // Find app_hdr struct based on magic value
        firmware.data.chunks_exact(4).for_each(|e| {
            if bytes_to_int_be(e, 4) == APP_HEADER_MAGIC {
                assert!(app_hdr_index.is_none(), "Found magic bytes more than once. Failed to \
                        automatically locate app header");
                    app_hdr_index = Some(index);
            }
            index += 4;
        });
        assert!(app_hdr_index.is_some());

        self.header.magic = APP_HEADER_MAGIC;
        self.header.size = bytes_to_int_be(&firmware.data[app_hdr_index?+4..app_hdr_index?+8], 4);
        self.header.entry_point = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+52..app_hdr_index?+56], 4);
        self.header.protected_count = 
            bytes_to_int_be(&firmware.data[app_hdr_index?-4..app_hdr_index?], 4);
        self.header.protected_addr = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+60..app_hdr_index?+64], 4);
        self.header.section_linked_list = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+64..app_hdr_index?+68], 4);
        self.header.memset_list_start = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+72..app_hdr_index?+76], 4);
        self.header.memset_list_end = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+76..app_hdr_index?+80], 4);
        self.header.copy_list_start = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+80..app_hdr_index?+84], 4);
        self.header.copy_list_end = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+84..app_hdr_index?+88], 4);
        self.header.uncompress_list_start = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+92..app_hdr_index?+96], 4);
        self.header.uncompress_list_end = 
            bytes_to_int_be(&firmware.data[app_hdr_index?+96..app_hdr_index?+100], 4);

        Some(())
    }

    /// Parse out protected segments from firmware. These are address ranges that should not be 
    /// overwritten since they are virtal for the boot process
    pub fn initialize_protected(&mut self, firmware: &Firmware) -> Option<()> {
        for i in 0..self.header.protected_count {
            let start = bytes_to_int_be(&firmware.data[
                self.header.protected_addr - firmware.header.load_addr + i*8..
                self.header.protected_addr - firmware.header.load_addr + i*8 + 4], 4);

            let end = bytes_to_int_be(&firmware.data[
                self.header.protected_addr - firmware.header.load_addr + i*8+4..
                self.header.protected_addr - firmware.header.load_addr + i*8 + 8], 4);

            self.protected_ranges.push((start, end));
        }
        Some(())
    }

    /// Parse out protected segments from firmware. These are address ranges that should not be 
    /// overwritten since they are virtal for the boot process
    pub fn parse_tripples(&mut self, data: &[u8]) 
        -> Option<Vec<(usize, usize, usize)>> {

            Some(data.chunks_exact(12)
            .map(|e| {
                (
                    bytes_to_int_be(&e[0..4], 4),
                    bytes_to_int_be(&e[4..8], 4),
                    bytes_to_int_be(&e[8..12], 4),
                )
            }).collect())
    }

    /// Parse out protected segments from firmware. These are address ranges that should not be 
    /// overwritten since they are virtal for the boot process
    pub fn initialize_tripples(&mut self, firmware: &Firmware) {
        // Parse out uncompress tripples
        self.uncompress_tripples = self.parse_tripples(
            &firmware.data[self.header.uncompress_list_start - firmware.header.load_addr..
            self.header.uncompress_list_end - firmware.header.load_addr]).unwrap();

        // Parse out memset tripples
        self.memset_tripples = self.parse_tripples(
            &firmware.data[self.header.memset_list_start - firmware.header.load_addr..
            self.header.memset_list_end - firmware.header.load_addr]).unwrap();

        // Parse out memcpy tripples
        self.memcpy_tripples = self.parse_tripples(
            &firmware.data[self.header.copy_list_start - firmware.header.load_addr..
            self.header.copy_list_end - firmware.header.load_addr]).unwrap();
    }

    /// Return true if given range overlaps with a protected section
    pub fn is_protected(&self, start: usize, end: usize) -> bool {
        for protected_section in self.protected_ranges.iter() {
            if std::cmp::max(protected_section.0, start) 
                <= std::cmp::min(protected_section.1, end) {
                return true;
            }
        }
        false
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

    let _ = std::fs::remove_dir_all("segments");
    std::fs::create_dir_all("segments").unwrap();
    firmware.dump_hardcoded();

    //println!("HEADER: {:#X?}", firmware.header);
    std::fs::write("./firmware", &firmware.data).unwrap();

    let mut bootloader = BootLoader::default();
    bootloader.parse_header(&firmware);
    bootloader.initialize_protected(&firmware);
    bootloader.initialize_tripples(&firmware);

    //println!("PROTECTED: {:#X?}", bootloader.protected_ranges);


    // Uncompress all tripples related to sections meant to be uncompressed
    for tripple in &bootloader.uncompress_tripples {
        let dst  = tripple.0;
        let src  = tripple.1;
        let size = tripple.2;

        if size == 0 {
            continue;
        }

        let data = lzss_uncompress(&firmware.data[src
                                   .checked_sub(firmware.header.load_addr).unwrap()..
                                   (src - firmware.header.load_addr)
                                   .checked_add(size).unwrap()]);

        // Verify that this section is not going to be overwriting a protected segment
        if bootloader.is_protected(dst, dst+data.len()) {
            println!("[!] Uncompress: Found protected at: {:#X?} : {:#X?}", dst, dst+data.len());
            continue
        }

        let path: String = format!("segments/{:X}.dump", dst).to_string();
        std::fs::write(path, data).unwrap()
    }

    // Dump data for memset tripples
    for tripple in &bootloader.memset_tripples {
        let dst  = tripple.0;
        let val  = tripple.1 as u8;
        let size = tripple.2;

        if size == 0 {
            continue;
        }

        // Verify that this section is not going to be overwriting a protected segment
        if bootloader.is_protected(dst, dst+size) {
            println!("[!] Memset: Found protected at: {:#X?} : {:#X?}", dst, dst+data.len());
            continue
        }

        let data = vec![val; size];
        let path: String = format!("segments/{:X}.dump", dst).to_string();
        std::fs::write(path, data).unwrap()
    }

    // Dump data for memcpy tripples
    for tripple in &bootloader.memcpy_tripples {
        let dst  = tripple.0;
        let src  = tripple.1;
        let size = tripple.2;

        if size == 0 {
            continue;
        }

        let data = &firmware.data[src.checked_sub(firmware.header.load_addr).unwrap()..
            (src - firmware.header.load_addr).checked_add(size).unwrap()];

        // Verify that this section is not going to be overwriting a protected segment
        if bootloader.is_protected(dst, dst+size) {
            println!("[!] Memcpy: Found protected at: {:#X?} : {:#X?}", dst, dst+data.len());
            continue
        }
        // Verify that this section is not going to be overwriting a protected segment
        if bootloader.is_protected(dst, dst+size) {
            println!("[!] Memcpy: Found protected at: {:#X?} : {:#X?}", dst, dst+data.len());
            continue
        }

        let path: String = format!("segments/{:X}.dump", dst).to_string();
        std::fs::write(path, data).unwrap()
    }

    println!("{:#X?}", bootloader);


    /*
    let _ = std::fs::remove_dir_all("segments");
    std::fs::create_dir_all("segments").unwrap();
    for segment in &firmware.segments {
        if segment.size == 0 {
            println!("Skipping {} because size==0", segment.name);
            continue
        }

        if segment.start < firmware.header.load_addr { println!("Skipping {} because start < load-address", segment.name);
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
    */
    println!("Firmware Load address: {:#X?}", firmware.header.load_addr);
    println!("Entrypoint: {:#X?}", bootloader.header.entry_point);

}
