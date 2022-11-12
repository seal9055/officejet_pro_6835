// turn off MSVC library CRT warning
#define _CRT_SECURE_NO_WARNINGS

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define uint8_t unsigned char
#define int8_t char
#define uint32_t unsigned int
#define int32_t int

uint32_t uncompress(uint8_t *dst, uint8_t *src, uint32_t len) {
    int32_t window_start = -4078;
    int32_t window_counter = 4078;
    uint32_t src_idx = 0;
    uint32_t control = 0;
    uint32_t write_idx = 0;
    while (1) {
        if (src_idx >= len) {
            if (src_idx > len) {
                puts("src_idx overflow");
            }
            break;
        }
        uint8_t data = src[src_idx];
        // Check if we exhausted the control bit
        if ((control & 0x100) == 0) {
            // Add new control bit
            printf("Found window %hhx at 0x%x\n", data, src_idx);
            // assert(data == 0xFF || data == 0xFD || data == 0xFB);
            control = 0xFF00U | data;
            ++src_idx;
            printf("Control set to %x\n", control);
        } else if ((control & 1) == 1) {
            // Bit 1 means data byte
            control >>= 1;
            // printf("Control left %x\n", control);
            dst[write_idx++] = data;
            if (window_counter + 1 >= 0x1000) {
                window_start += 0x1000;
            }
            window_counter = (window_counter + 1) & 0xfff;
            ++src_idx;
        } else if ((control & 1) == 0) {
            // Bit 0 means byte from window
            control >>= 1;

            uint32_t offset_upper = (src[src_idx + 1] >> 4) & 0xfU;
            uint32_t offset_lower = src[src_idx] & 0xffU;
            // displacement
            int32_t offset = (int32_t)((offset_upper << 8) | offset_lower);
            uint32_t length = (src[src_idx + 1] & 0xfU) + 3;

            if (window_counter + (int32_t)length >= 0x1000) {
                window_start += 0x1000;
            }

            int32_t lookup = offset + window_start;
            if (lookup >= (int32_t)write_idx) {
                lookup -= 0x1000;
            }

            printf("Decompress with token %0hhx %0hhx "
                   "at 0x%x, to 0x%x, length %d, "
                   "window_start %d, window_counter %d, "
                   "offset 0x%x, lookup 0x%x\n",
                   src[src_idx], src[src_idx + 1], src_idx, write_idx, length,
                   window_start, window_counter, offset, lookup);

            src_idx += 2;
            // Extract from window into dst
            for (uint32_t i = 0; i < length; ++i) {
                int32_t target = lookup + (int32_t)i;
                dst[write_idx + i] = (target >= 0) ? dst[target] : 0;
            }
            write_idx += length;
            window_counter = (window_counter + (int32_t)length) & 0xfff;
        }
    }
    return write_idx;
}

int main(void) {
#ifndef BASE
#define BASE 0x26710000
#endif
    unsigned int base = BASE;
    printf("Base: 0x%x\n", base);
    unsigned int src_data_1 = 0x26724b18 - base;
    unsigned int len_data_1 = 0x3cfa;
    uint8_t *dst_1 = malloc(len_data_1);
    uint8_t *uncompressed_1 = malloc(len_data_1 * 100);

    unsigned int src_data_2 = 0x26722398 - base;
    unsigned int len_data_2 = 0x277f;
    uint8_t *dst_2 = malloc(len_data_2);
    uint8_t *uncompressed_2 = malloc(len_data_2 * 100);

    unsigned int src_data_3 = 0x267107b0 - base;
    unsigned int len_data_3 = 0x11be7;
    uint8_t *dst_3 = malloc(len_data_3);
    uint8_t *uncompressed_3 = malloc(len_data_3 * 10);

    // Cromtext
    unsigned int src_cromtext = 0x26778EF8 - base;
    unsigned int len_cromtext = 0x6D5F25;
    uint8_t *dst_cromtext = malloc(len_cromtext);
    uint8_t *uncompressed_cromtext = malloc(len_cromtext * 100);


    // Cromdata
    unsigned int src_cromdata = 0x278cf318 - base;
    unsigned int len_cromdata = 0x4c11a;
    uint8_t *dst_cromdata = malloc(len_cromdata);
    uint8_t *uncompressed_cromdata = malloc(len_cromdata * 100);

    // Cromrodata
    unsigned int src_crom_ro_data = 0x2757a360 - base;
    unsigned int len_crom_ro_data = 0x354fb8;
    uint8_t *dst_crom_ro_data = malloc(len_crom_ro_data);
    uint8_t *uncompressed_crom_ro_data = malloc(len_crom_ro_data * 100);


    // Cromncdata
    unsigned int src_crom_nc_data = 0x2791b434 - base;
    unsigned int len_crom_nc_data = 0x30d;
    uint8_t *dst_crom_nc_data = malloc(len_crom_nc_data);
    uint8_t *uncompressed_crom_nc_data = malloc(len_crom_nc_data * 100);

    // Crom Module
    unsigned int src_crom_module = 0x27ff166c - base;
    unsigned int len_crom_module = 0x73a;
    uint8_t *dst_crom_module = malloc(len_crom_module);
    uint8_t *uncompressed_crom_module = malloc(len_crom_module * 100);

    // Crom FS
    unsigned int src_crom_fs = 0x27ff1da8 - base;
    unsigned int len_crom_fs = 0x502;
    uint8_t *dst_crom_fs = malloc(len_crom_fs);
    uint8_t *uncompressed_crom_fs = malloc(len_crom_fs * 100);

    // Crom FS Objects
    unsigned int src_crom_fs_objs = 0x27ff22ac - base;
    unsigned int len_crom_fs_objs = 0x32b9;
    uint8_t *dst_crom_fs_objs = malloc(len_crom_fs_objs);
    uint8_t *uncompressed_crom_fs_objs = malloc(len_crom_fs_objs * 100);

    // Read firmware file into memory
    FILE *fd = fopen("firmware_with_3_data", "rb");
    fseek(fd, 0, SEEK_END);
    size_t fsize = (size_t)ftell(fd);
    fseek(fd, 0, SEEK_SET);

    // Load firmware
    char *buf = malloc(fsize);
    fread(buf, fsize, 1, fd);
    fclose(fd);

    // Load bytes from disk-firmware into memory
    memcpy(dst_1, buf + src_data_1, len_data_1);
    memcpy(dst_2, buf + src_data_2, len_data_2);
    memcpy(dst_3, buf + src_data_3, len_data_3);
    memcpy(dst_cromtext, buf + src_cromtext, len_cromtext);
    memcpy(dst_cromdata, buf + src_cromdata, len_cromdata);
    memcpy(dst_crom_ro_data, buf + src_crom_ro_data, len_crom_ro_data);
    memcpy(dst_crom_nc_data, buf + src_crom_nc_data, len_crom_nc_data);
    memcpy(dst_crom_module, buf + src_crom_module, len_crom_module);
    memcpy(dst_crom_fs, buf + src_crom_fs, len_crom_fs);
    memcpy(dst_crom_fs_objs, buf + src_crom_fs_objs, len_crom_fs_objs);
    
    // Decompress the segments
    unsigned int uncompressed_len_1 =
        uncompress(uncompressed_1, dst_1, len_data_1);
    unsigned int uncompressed_len_2 =
        uncompress(uncompressed_2, dst_2, len_data_2);
    unsigned int uncompressed_len_3 =
        uncompress(uncompressed_3, dst_3, len_data_3);
    unsigned int uncompressed_cromtext_len =
        uncompress(uncompressed_cromtext, dst_cromtext, len_cromtext);
    unsigned int uncompressed_cromdata_len =
        uncompress(uncompressed_cromdata, dst_cromdata, len_cromdata);

    unsigned int uncompressed_crom_ro_data_len =
        uncompress(uncompressed_crom_ro_data, dst_crom_ro_data, len_crom_ro_data);
    unsigned int uncompressed_crom_nc_data_len =
        uncompress(uncompressed_crom_nc_data, dst_crom_nc_data, len_crom_nc_data);
    unsigned int uncompressed_crom_module_len =
        uncompress(uncompressed_crom_module, dst_crom_module, len_crom_module);
    unsigned int uncompressed_crom_fs_len =
        uncompress(uncompressed_crom_fs, dst_crom_fs, len_crom_fs);
    unsigned int uncompressed_crom_fs_objs_len =
        uncompress(uncompressed_crom_fs_objs, dst_crom_fs_objs, len_crom_fs_objs);

    FILE *file_1 = fopen("data1", "wb");
    fwrite(uncompressed_1, uncompressed_len_1, 1, file_1);
    FILE *file_2 = fopen("data2", "wb");
    fwrite(uncompressed_2, uncompressed_len_2, 1, file_2);
    FILE *file_3 = fopen("data3", "wb");
    fwrite(uncompressed_3, uncompressed_len_3, 1, file_3);
    FILE *file_4 = fopen("cromtext", "wb");
    fwrite(uncompressed_cromtext, len_cromtext, 1, file_4);
    FILE *file_5 = fopen("cromdata", "wb");
    fwrite(uncompressed_cromdata, len_cromdata, 1, file_5);
    FILE *file_6 = fopen("crom_ro_data", "wb");
    fwrite(uncompressed_crom_ro_data, len_crom_ro_data, 1, file_6);
    FILE *file_7 = fopen("crom_nc_data", "wb");
    fwrite(uncompressed_crom_nc_data, len_crom_nc_data, 1, file_7);
    FILE *file_8 = fopen("crom_module", "wb");
    fwrite(uncompressed_crom_module, len_crom_module, 1, file_8);
    FILE *file_9 = fopen("crom_fs", "wb");
    fwrite(uncompressed_crom_fs, len_crom_fs, 1, file_9);
    FILE *file_10 = fopen("crom_fs_objs", "wb");
    fwrite(uncompressed_crom_fs_objs, len_crom_fs_objs, 1, file_10);

    puts("Write finished");
    return 0;
}
