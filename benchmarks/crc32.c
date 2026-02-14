#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static long long timespec_diff_ns(struct timespec *a, struct timespec *b) {
    return (long long)(b->tv_sec - a->tv_sec) * 1000000000LL + (b->tv_nsec - a->tv_nsec);
}

static int cmp_ll(const void *a, const void *b) {
    long long x = *(const long long *)a, y = *(const long long *)b;
    return (x > y) - (x < y);
}

static unsigned int lcg_state = 12345;
static unsigned int lcg_rand(void) {
    lcg_state = lcg_state * 1103515245 + 12345;
    return (lcg_state >> 16) & 0x7fff;
}

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

/* Standard table-driven CRC32 */
static unsigned int crc32_table_driven(const unsigned char *data, int len) {
    unsigned int crc = 0xFFFFFFFFu;
    int i;
    for (i = 0; i < len; i++) {
        crc = (crc >> 8) ^ crc32_table[(crc ^ data[i]) & 0xFF];
    }
    return crc ^ 0xFFFFFFFFu;
}

/* Bit-by-bit CRC32 (no table) — more branch-heavy */
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

/* XOR checksum with bit rotation — tests instcombine on bitwise ops */
static unsigned int xor_rotate_checksum(const unsigned char *data, int len) {
    unsigned int hash = 0x12345678u;
    int i;
    for (i = 0; i < len; i++) {
        hash ^= (unsigned int)data[i];
        /* Rotate left by 5 */
        hash = (hash << 5) | (hash >> 27);
        hash += data[i] * 0x01000193u;  /* FNV-like multiply */
    }
    return hash;
}

/* Popcount-style reduction — bitwise heavy */
static unsigned int popcount_sum(const unsigned char *data, int len) {
    unsigned int total = 0;
    int i;
    for (i = 0; i < len; i++) {
        unsigned int v = data[i];
        /* Brian Kernighan's popcount */
        while (v) {
            v &= v - 1;
            total++;
        }
    }
    return total;
}

/* Byte-level bit reversal + accumulate */
static unsigned int bit_reverse_accum(const unsigned char *data, int len) {
    unsigned int accum = 0;
    int i;
    for (i = 0; i < len; i++) {
        unsigned int b = data[i];
        /* Reverse bits of byte */
        b = ((b & 0xF0) >> 4) | ((b & 0x0F) << 4);
        b = ((b & 0xCC) >> 2) | ((b & 0x33) << 2);
        b = ((b & 0xAA) >> 1) | ((b & 0x55) << 1);
        accum += b;
        accum ^= (accum << 3);
    }
    return accum;
}

static unsigned int workload(const unsigned char *data, int len) {
    unsigned int result = 0;
    result ^= crc32_table_driven(data, len);
    result ^= crc32_bitwise(data, len);
    result ^= xor_rotate_checksum(data, len);
    result ^= popcount_sum(data, len);
    result ^= bit_reverse_accum(data, len);
    return result;
}

int main(void) {
    unsigned char *data = (unsigned char *)malloc(DATA_SIZE);
    int i;

    lcg_state = 12345;
    for (i = 0; i < DATA_SIZE; i++) {
        data[i] = (unsigned char)(lcg_rand() & 0xFF);
    }

    crc32_init_table();

    /* Warmup */
    volatile unsigned int sink;
    for (i = 0; i < 5; i++) {
        sink = workload(data, DATA_SIZE);
    }

    /* Timing: 201 runs, 10% trimmed mean */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(data, DATA_SIZE);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(data);
    return 0;
}
