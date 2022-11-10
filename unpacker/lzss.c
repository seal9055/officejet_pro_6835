#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define uint8_t unsigned char
#define int8_t char
#define uint32_t unsigned int
#define int32_t int

int32_t uncompress(uint8_t* dst, char* src, int32_t len) {
    uint8_t* r5 = dst;
    int32_t r3 = 0xfee;
    void* r6 = &src[len];
    uint32_t r4 = 0;
    while (src != r6)
    {
        r4 = (r4 >> 1);
        if ((r4 & 0x100) == 0)
        {
            uint32_t r2 = ((uint32_t)*(int8_t*)src);
            src = &src[1];
            r4 = (r2 | 0xff00);
        }
        uint32_t r2_1 = ((uint32_t)*(int8_t*)src);
        src = &src[1];
        if ((r4 & 1) != 0)
        {
            *(int8_t*)dst = ((int8_t)r2_1);
            dst = &dst[1];
            r3 = (((r3 + 1) >> 0) & 0xfff);
        }
        else
        {
            uint32_t r12_1 = ((uint32_t)*(int8_t*)src);
            src = &src[1];
            int32_t r12_3 = ((r12_1 & 0xf) + 3);
            int32_t r2_3 = (r2_1 | ((r12_1 & 0xf0) << 4));
            void* r7_3 = (&dst[r2_3] - r3);
            if (r3 <= r2_3)
            {
                r7_3 = ((char*)r7_3 - 0x1000);
            }
            for (int32_t i = 0; i < r12_3; i = (i + 1))
            {
                if (r7_3 >= r5)
                {
                    do
                    {
                        uint8_t r10_1 = *(int8_t*)r7_3;
                        r7_3 = ((char*)r7_3 + 1);
                        i = (i + 1);
                        *(int8_t*)dst = r10_1;
                        dst = &dst[1];
                    } while (i < r12_3);
                    break;
                }
                *(int8_t*)dst = 0;
                dst = &dst[1];
                r7_3 = ((char*)r7_3 + 1);
            }
            r3 = (((r12_3 + r3) >> 0) & 0xfff);
        }
    }
    return (dst - r5);
}

int main() {
    unsigned int src_data_1 = 0x14b18;
    unsigned int len_data_1 = 0x3cfa;
    char *dst_1 = malloc(len_data_1);
    unsigned char *uncompressed_1 = malloc(len_data_1 * 5);

    unsigned int src_data_2 = 0x12398;
    unsigned int len_data_2 = 0x277f;
    char *dst_2 = malloc(len_data_2);
    unsigned char *uncompressed_2 = malloc(len_data_2 * 5);

    unsigned int src_data_3 = 0x7b0;
    unsigned int len_data_3 = 0x11be7;
    char *dst_3 = malloc(len_data_3);
    unsigned char *uncompressed_3 = malloc(len_data_3 * 5);

    // Read firmware file into memory
    FILE *fd = fopen("firmware", "rb");
    fseek(fd, 0, SEEK_END);
    long fsize = ftell(fd);
    fseek(fd, 0, SEEK_SET);

    char *buf = malloc(fsize);
    fread(buf, fsize, 1, fd);
    fclose(fd);

    memcpy(dst_1, buf+src_data_1, len_data_1);
    memcpy(dst_2, buf+src_data_2, len_data_2);
    memcpy(dst_3, buf+src_data_3, len_data_3);

    unsigned int uncompressed_len_1 = uncompress(uncompressed_1, dst_1, len_data_1);
    unsigned int uncompressed_len_2 = uncompress(uncompressed_2, dst_2, len_data_2);
    unsigned int uncompressed_len_3 = uncompress(uncompressed_3, dst_3, len_data_3);

    FILE *file_1 = fopen("data1","wb");
    fwrite(uncompressed_1, uncompressed_len_1, 1, file_1);

    FILE *file_2 = fopen("data2","wb");
    fwrite(uncompressed_2, uncompressed_len_2, 1, file_2);

    FILE *file_3 = fopen("data3","wb");
    fwrite(uncompressed_3, uncompressed_len_3, 1, file_3);
}
