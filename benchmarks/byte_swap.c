#include "bench_timing.h"
#include <string.h>

/*
 * Byte manipulation and endian conversion patterns —
 * targets for aggressive-instcombine (trunc/zext/bswap folding).
 */

#define N 4000

/* Manual byte swap 16-bit — aggressive-instcombine should fold to bswap */
static unsigned short bswap16_manual(unsigned short x) {
    return (unsigned short)((x >> 8) | (x << 8));
}

/* Manual byte swap 32-bit via shifts */
static unsigned int bswap32_manual(unsigned int x) {
    return ((x >> 24) & 0xFF) |
           ((x >> 8)  & 0xFF00) |
           ((x << 8)  & 0xFF0000) |
           ((x << 24) & 0xFF000000u);
}

/* Manual byte swap 64-bit */
static unsigned long long bswap64_manual(unsigned long long x) {
    x = ((x & 0x00000000FFFFFFFFull) << 32) | ((x & 0xFFFFFFFF00000000ull) >> 32);
    x = ((x & 0x0000FFFF0000FFFFull) << 16) | ((x & 0xFFFF0000FFFF0000ull) >> 16);
    x = ((x & 0x00FF00FF00FF00FFull) << 8)  | ((x & 0xFF00FF00FF00FF00ull) >> 8);
    return x;
}

/* Bit reversal in a byte — aggressive-instcombine pattern */
static unsigned char reverse_bits(unsigned char b) {
    b = (unsigned char)(((b * 0x0802u & 0x22110u) | (b * 0x8020u & 0x88440u)) * 0x10101u >> 16);
    return b;
}

/* Extract packed fields from a 32-bit word (network protocol style) */
static void unpack_fields(unsigned int packed,
                          unsigned char *a, unsigned char *b,
                          unsigned short *c) {
    *a = (unsigned char)(packed >> 24);
    *b = (unsigned char)((packed >> 16) & 0xFF);
    *c = (unsigned short)(packed & 0xFFFF);
}

/* Pack fields back — truncation pattern */
static unsigned int pack_fields(unsigned char a, unsigned char b,
                                unsigned short c) {
    return ((unsigned int)a << 24) |
           ((unsigned int)b << 16) |
           (unsigned int)c;
}

/* Rotate left 32-bit — instcombine pattern */
static unsigned int rotl32(unsigned int x, int n) {
    return (x << n) | (x >> (32 - n));
}

/* Population count via bit manipulation (not __builtin_popcount) */
static int popcount_manual(unsigned int x) {
    x = x - ((x >> 1) & 0x55555555u);
    x = (x & 0x33333333u) + ((x >> 2) & 0x33333333u);
    x = (x + (x >> 4)) & 0x0F0F0F0Fu;
    return (int)((x * 0x01010101u) >> 24);
}

/* Count leading zeros via bit manipulation */
static int clz_manual(unsigned int x) {
    int n = 32;
    if (x & 0xFFFF0000u) { n -= 16; x >>= 16; }
    if (x & 0x0000FF00u) { n -= 8;  x >>= 8;  }
    if (x & 0x000000F0u) { n -= 4;  x >>= 4;  }
    if (x & 0x0000000Cu) { n -= 2;  x >>= 2;  }
    if (x & 0x00000002u) { n -= 1;  x >>= 1;  }
    if (x) n--;
    return n;
}

/* SipHash-like mixing — wide operations that get narrowed */
static unsigned long long sip_round(unsigned long long v0, unsigned long long v1) {
    v0 += v1;
    v1 = (v1 << 13) | (v1 >> 51);
    v1 ^= v0;
    v0 = (v0 << 32) | (v0 >> 32);
    v0 += v1;
    v1 = (v1 << 17) | (v1 >> 47);
    v1 ^= v0;
    v0 = (v0 << 32) | (v0 >> 32);
    return v0 ^ v1;
}

static long long workload(unsigned int *data) {
    long long sum = 0;
    int i;

    /* Byte swaps */
    for (i = 0; i < N; i++) {
        unsigned short lo = (unsigned short)(data[i] & 0xFFFF);
        sum += bswap16_manual(lo);
        sum += bswap32_manual(data[i]);
    }

    /* 64-bit byte swaps */
    for (i = 0; i < N - 1; i += 2) {
        unsigned long long w = ((unsigned long long)data[i] << 32) | data[i + 1];
        sum += (long long)bswap64_manual(w);
    }

    /* Bit reversal */
    for (i = 0; i < N; i++) {
        sum += reverse_bits((unsigned char)(data[i] & 0xFF));
    }

    /* Pack/unpack round-trip */
    for (i = 0; i < N; i++) {
        unsigned char a, b;
        unsigned short c;
        unpack_fields(data[i], &a, &b, &c);
        sum += pack_fields((unsigned char)(a + 1), b, (unsigned short)(c ^ 0x1234));
    }

    /* Rotate + popcount + clz */
    for (i = 0; i < N; i++) {
        unsigned int r = rotl32(data[i], i % 31 + 1);
        sum += popcount_manual(r);
        sum += clz_manual(r);
    }

    /* SipHash mixing */
    unsigned long long v = 0x736F6D6570736575ull;
    for (i = 0; i < N; i++) {
        v = sip_round(v, (unsigned long long)data[i]);
    }
    sum += (long long)v;

    return sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    unsigned int data[N];
    int i;

    bench_lcg_seed(99);
    for (i = 0; i < N; i++) {
        data[i] = (unsigned int)(bench_lcg_rand() << 17) ^ bench_lcg_rand();
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data); });

    return 0;
}
