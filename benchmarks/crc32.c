#include "bench_timing.h"

#define DATA_SIZE 5000

static unsigned int crc32_table[256];

static void crc32_init_table(void) {
    unsigned int i, j, crc;
    for (i = 0; i < 256; i++) {
        crc = i;
        for (j = 0; j < 8; j++) {
            if (crc & 1)
                crc = (crc >> 1) ^ 0xEDB88320u;
            else
                crc = crc >> 1;
        }
        crc32_table[i] = crc;
    }
}

static unsigned int crc32_table_driven(const unsigned char *data, int len) {
    unsigned int crc = 0xFFFFFFFFu;
    int i;
    for (i = 0; i < len; i++) {
        crc = (crc >> 8) ^ crc32_table[(crc ^ data[i]) & 0xFF];
    }
    return crc ^ 0xFFFFFFFFu;
}

static unsigned int crc32_bitwise(const unsigned char *data, int len) {
    unsigned int crc = 0xFFFFFFFFu;
    int i, j;
    for (i = 0; i < len; i++) {
        crc ^= data[i];
        for (j = 0; j < 8; j++) {
            if (crc & 1)
                crc = (crc >> 1) ^ 0xEDB88320u;
            else
                crc = crc >> 1;
        }
    }
    return crc ^ 0xFFFFFFFFu;
}

static unsigned int xor_rotate_checksum(const unsigned char *data, int len) {
    unsigned int hash = 0x12345678u;
    int i;
    for (i = 0; i < len; i++) {
        hash ^= (unsigned int)data[i];
        hash = (hash << 5) | (hash >> 27);
        hash += data[i] * 0x01000193u;
    }
    return hash;
}

static unsigned int popcount_sum(const unsigned char *data, int len) {
    unsigned int total = 0;
    int i;
    for (i = 0; i < len; i++) {
        unsigned int v = data[i];
        while (v) {
            v &= v - 1;
            total++;
        }
    }
    return total;
}

static unsigned int bit_reverse_accum(const unsigned char *data, int len) {
    unsigned int accum = 0;
    int i;
    for (i = 0; i < len; i++) {
        unsigned int b = data[i];
        b = ((b & 0xF0) >> 4) | ((b & 0x0F) << 4);
        b = ((b & 0xCC) >> 2) | ((b & 0x33) << 2);
        b = ((b & 0xAA) >> 1) | ((b & 0x55) << 1);
        accum += b;
        accum ^= (accum << 3);
    }
    return accum;
}

/* --- Variant 1: Adler-32 checksum --- */

static unsigned int adler32_checksum(const unsigned char *data, int len) {
    unsigned int a = 1, b = 0;
    int i;
    int block;
    for (i = 0; i < len; ) {
        block = len - i;
        if (block > 5552) block = 5552;
        int end = i + block;
        for (; i < end; i++) {
            a += data[i];
            b += a;
        }
        a %= 65521u;
        b %= 65521u;
    }
    return (b << 16) | a;
}

/* --- Variant 2: Fletcher-16 checksum --- */

static unsigned int fletcher16_checksum(const unsigned char *data, int len) {
    unsigned int sum1 = 0xff, sum2 = 0xff;
    int i;
    while (len) {
        int tlen = len > 20 ? 20 : len;
        len -= tlen;
        for (i = 0; i < tlen; i++) {
            sum1 += *data++;
            sum2 += sum1;
        }
        sum1 = (sum1 & 0xff) + (sum1 >> 8);
        sum2 = (sum2 & 0xff) + (sum2 >> 8);
    }
    sum1 = (sum1 & 0xff) + (sum1 >> 8);
    sum2 = (sum2 & 0xff) + (sum2 >> 8);
    return (sum2 << 8) | sum1;
}

/* --- Variant 3: CRC-32C (Castagnoli) bitwise --- */

static unsigned int crc32c_bitwise(const unsigned char *data, int len) {
    unsigned int crc = 0xFFFFFFFFu;
    int i, j;
    for (i = 0; i < len; i++) {
        crc ^= data[i];
        for (j = 0; j < 8; j++) {
            if (crc & 1)
                crc = (crc >> 1) ^ 0x82F63B78u;
            else
                crc = crc >> 1;
        }
    }
    return crc ^ 0xFFFFFFFFu;
}

static unsigned int workload(const unsigned char *data, int len) {
    unsigned int result = 0;
    result ^= crc32_table_driven(data, len);
    result ^= crc32_bitwise(data, len);
    result ^= xor_rotate_checksum(data, len);
    result ^= popcount_sum(data, len);
    result ^= bit_reverse_accum(data, len);
    result ^= adler32_checksum(data, len);
    result ^= fletcher16_checksum(data, len);
    result ^= crc32c_bitwise(data, len);
    return result;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    unsigned char *data = (unsigned char *)malloc(DATA_SIZE);
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < DATA_SIZE; i++) {
        data[i] = (unsigned char)(bench_lcg_rand() & 0xFF);
    }

    crc32_init_table();

    volatile unsigned int sink;
    BENCH_TIME(niters, { sink = workload(data, DATA_SIZE); });

    free(data);
    return 0;
}
