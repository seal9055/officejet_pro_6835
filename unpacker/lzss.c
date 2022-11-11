#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define uint8_t unsigned char
#define int8_t signed char
#define uint32_t unsigned int
#define int32_t signed int

// Perform lzss decompression on len bytes of src and store them in dst
int32_t uncompress(uint8_t* dst, uint8_t* src, int32_t len) {
    uint8_t* base = dst;
    void* end = &src[len];
    uint32_t index = 0;

    // Lookahead buffer for lzss 4096 - 18 (PAGE_SIZE - TOKEN_LENGTH)
    int32_t window_start = 4078;

    // Loop through entire source buffer to decompress
    while (src != end) {
        index = (index >> 1);

        // Read flags data to check if this is a compressed block or raw data
        if ((index & 0x100) == 0) {
            uint32_t r2 = ((uint32_t)*(int8_t*)src);
            src = &src[1];
            index = (r2 | 0xff00);
        }

        uint32_t r2_1 = ((uint32_t)*(int8_t*)src);
        src = &src[1];

        if ((index & 1) != 0) {
            // Uncompressed block
            *(int8_t*)dst = ((int8_t)r2_1);
            dst = &dst[1];
            window_start = (((window_start + 1) >> 0) & 0xfff);
        } else {
            // Compressed block
            uint32_t r12_1 = ((uint32_t)*(int8_t*)src);
            src = &src[1];

            // Extract length and displacement from the window-indexing bytes
            int32_t length = ((r12_1 & 0xf) + 3);
            int32_t displacement = (r2_1 | ((r12_1 & 0xf0) << 4));
            uint8_t* r7_3 = (&dst[displacement] - window_start);

            // Wrap around window if necessary
            if (window_start <= displacement) {
                r7_3 = ((uint8_t*)r7_3 - 0x1000);
            }

            // Memcpy bytes from window to dst
            for (int32_t i = 0; i < length; i = (i + 1)) {
                if (r7_3 >= base) {
                    do {
                        uint8_t r10_1 = *(int8_t*)r7_3;
                        r7_3 = ((uint8_t*)r7_3 + 1);
                        i = (i + 1);
                        *(int8_t*)dst = r10_1;
                        dst = &dst[1];
                    } while (i < length);
                    break;
                }
                *(int8_t*)dst = 0;
                dst = &dst[1];
                r7_3 = ((uint8_t*)r7_3 + 1);
            }
            window_start = (((length + window_start) >> 0) & 0xfff);
        }
    }
    return (dst - base);
}

int main() {
    unsigned int src_data_1 = 0x14b18;
    unsigned int len_data_1 = 0x3cfa;
    unsigned char *dst_1 = malloc(len_data_1);
    unsigned char *uncompressed_1 = malloc(len_data_1 * 5);

    unsigned int src_data_2 = 0x12398;
    unsigned int len_data_2 = 0x277f;
    unsigned char *dst_2 = malloc(len_data_2);
    unsigned char *uncompressed_2 = malloc(len_data_2 * 5);

    unsigned int src_data_3 = 0x7b0;
    unsigned int len_data_3 = 0x11be7;
    unsigned char *dst_3 = malloc(len_data_3);
    unsigned char *uncompressed_3 = malloc(len_data_3 * 5);

    // Read firmware file into memory
    FILE *fd = fopen("firmware", "rb");
    fseek(fd, 0, SEEK_END);
    long fsize = ftell(fd);
    fseek(fd, 0, SEEK_SET);

    unsigned char *buf = malloc(fsize);
    unsigned char *buf_dst = malloc(fsize * 10);
    fread(buf, fsize, 1, fd);
    fclose(fd);

    memcpy(dst_1, buf+src_data_1, len_data_1);
    memcpy(dst_2, buf+src_data_2, len_data_2);
    memcpy(dst_3, buf+src_data_3, len_data_3);

    unsigned int uncompressed_len_1 = uncompress(uncompressed_1, dst_1, len_data_1);
    unsigned int uncompressed_len_2 = uncompress(uncompressed_2, dst_2, len_data_2);
    unsigned int uncompressed_len_3 = uncompress(uncompressed_3, dst_3, len_data_3);
    unsigned int uncompressed_len_4 = uncompress(buf_dst, buf, fsize);

    printf("LENGTHS: %d ; %d ; %d", uncompressed_len_1, uncompressed_len_2, uncompressed_len_3);

    FILE *file_1 = fopen("data1b","wb");
    fwrite(uncompressed_1, uncompressed_len_1, 1, file_1);

    FILE *file_2 = fopen("data2b","wb");
    fwrite(uncompressed_2, uncompressed_len_2, 1, file_2);

    FILE *file_3 = fopen("data3b","wb");
    fwrite(uncompressed_3, uncompressed_len_3, 1, file_3);

    FILE *file_4 = fopen("data4","wb");
    fwrite(buf_dst, uncompressed_len_4, 1, file_4);
}
