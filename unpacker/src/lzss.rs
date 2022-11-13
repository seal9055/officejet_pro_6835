/// Uncompress given data with lzss decompression routine
pub fn lzss_uncompress(src: &[u8]) -> Vec<u8> {
    let mut dst: Vec<u8> = Vec::new();
    let mut window_start: i32 = -4078;
    let mut window_counter: i32 = 4078;
    let mut src_idx: usize = 0;
    let mut control: u32 = 0;

    loop {
        if src_idx == src.len() {
            break
        }
        if src_idx == src.len()-1 {
            dst.push(src[src_idx]);
            break
        }
        let data: u8 = src[src_idx];
        if (control & 0x100) == 0x0 {
            control = 0xff00u32 | data as u32;
            src_idx += 1;
        } else if (control & 1) == 0x1 {
            control >>= 1;
            dst.push(data);
            if (window_counter + 1) >= 0x1000 {
                window_start += 0x1000;
            }
            window_counter = (window_counter + 1) & 0xfff;
            src_idx += 1;
        } else if (control & 1) == 0x0 {
            control >>= 1;

            let offset_upper: u32 = ((src[src_idx+1] >> 4) & 0xfu8).into();
            let offset_lower: u32 = (src[src_idx] & 0xffu8).into();
            let offset: i32 = ((offset_upper << 8) | offset_lower) as i32;
            let length: u32 = ((src[src_idx + 1] & 0xfu8) + 3).into();

            if window_counter + (length  as i32) >= 0x1000 {
                window_start += 0x1000;
            }

            let mut lookup: i32 = offset + window_start;
            while lookup >= dst.len() as i32 {
                lookup -= 0x1000;
            }
            
            src_idx += 2;

            for i in 0..length {
                let target: i32 = lookup + i as i32;
                dst.push(if target >= 0 { dst[target as usize] } else { 0 });
            }
            window_counter = (window_counter + length as i32) & 0xfff;
        }
    }
    dst
}


