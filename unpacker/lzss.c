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

    // Read firmware file into memory
    FILE *fd = fopen("firmware", "rb");
    fseek(fd, 0, SEEK_END);
    size_t fsize = (size_t)ftell(fd);
    fseek(fd, 0, SEEK_SET);

    char *buf = malloc(fsize);
    fread(buf, fsize, 1, fd);
    fclose(fd);

    memcpy(dst_1, buf + src_data_1, len_data_1);
    memcpy(dst_2, buf + src_data_2, len_data_2);
    memcpy(dst_3, buf + src_data_3, len_data_3);

    unsigned int uncompressed_len_1 =
        uncompress(uncompressed_1, dst_1, len_data_1);
    unsigned int uncompressed_len_2 =
        uncompress(uncompressed_2, dst_2, len_data_2);
    unsigned int uncompressed_len_3 =
        uncompress(uncompressed_3, dst_3, len_data_3);

    FILE *file_1 = fopen("data1", "wb");
    fwrite(uncompressed_1, uncompressed_len_1, 1, file_1);

    FILE *file_2 = fopen("data2", "wb");
    fwrite(uncompressed_2, uncompressed_len_2, 1, file_2);

    FILE *file_3 = fopen("data3", "wb");
    fwrite(uncompressed_3, uncompressed_len_3, 1, file_3);
    puts("Write finished");
    return 0;
}
