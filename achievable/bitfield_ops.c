/*
 * Targets: instcombine (bit manipulation), adce (dead bit ops),
 * early-cse (redundant mask computations), simplifycfg (branch chains).
 *
 * Packed bitfield manipulation, bitmap operations, bit counting.
 */
#include "bench_timing.h"

#define BITMAP_WORDS 256    /* 256 * 32 = 8192 bits */
#define BITMAP_BITS (BITMAP_WORDS * 32)
#define NUM_QUERIES 5000

typedef unsigned int uint32;

/* ---- Bitfield operations on packed 32-bit words ---- */

/* Get a field of `width` bits starting at bit `offset` */
static uint32 bitfield_get(uint32 word, int offset, int width) {
    uint32 mask = ((uint32)1 << width) - 1;
    return (word >> offset) & mask;
}

/* Set a field of `width` bits starting at bit `offset` to `value` */
static uint32 bitfield_set(uint32 word, int offset, int width, uint32 value) {
    uint32 mask = (((uint32)1 << width) - 1) << offset;
    word &= ~mask;
    word |= (value << offset) & mask;
    return word;
}

/* Toggle bits in a field */
static uint32 bitfield_toggle(uint32 word, int offset, int width) {
    uint32 mask = (((uint32)1 << width) - 1) << offset;
    return word ^ mask;
}

/* Count set bits in a field (Kernighan's algorithm) */
static int bitfield_popcount(uint32 word, int offset, int width) {
    uint32 val = bitfield_get(word, offset, width);
    int count = 0;
    while (val) {
        val &= val - 1;
        count++;
    }
    return count;
}

/* Find first set bit position (1-based), 0 if none */
static int bitfield_ffs(uint32 word) {
    if (word == 0) return 0;
    int pos = 1;
    if (!(word & 0xFFFF)) { word >>= 16; pos += 16; }
    if (!(word & 0xFF))   { word >>= 8;  pos += 8;  }
    if (!(word & 0xF))    { word >>= 4;  pos += 4;  }
    if (!(word & 0x3))    { word >>= 2;  pos += 2;  }
    if (!(word & 0x1))    { pos += 1; }
    return pos;
}

/* Find last set bit (bit scan reverse) */
static int bitfield_bsr(uint32 word) {
    if (word == 0) return 0;
    int pos = 0;
    if (word & 0xFFFF0000) { word >>= 16; pos += 16; }
    if (word & 0xFF00)     { word >>= 8;  pos += 8;  }
    if (word & 0xF0)       { word >>= 4;  pos += 4;  }
    if (word & 0xC)        { word >>= 2;  pos += 2;  }
    if (word & 0x2)        { pos += 1; }
    return pos;
}

/* Reverse bits in a 32-bit word */
static uint32 bit_reverse(uint32 word) {
    word = ((word & 0x55555555) << 1) | ((word >> 1) & 0x55555555);
    word = ((word & 0x33333333) << 2) | ((word >> 2) & 0x33333333);
    word = ((word & 0x0F0F0F0F) << 4) | ((word >> 4) & 0x0F0F0F0F);
    word = ((word & 0x00FF00FF) << 8) | ((word >> 8) & 0x00FF00FF);
    word = (word << 16) | (word >> 16);
    return word;
}

/* Rotate left */
static uint32 bit_rotl(uint32 word, int shift) {
    shift &= 31;
    return (word << shift) | (word >> (32 - shift));
}

/* Rotate right */
static uint32 bit_rotr(uint32 word, int shift) {
    shift &= 31;
    return (word >> shift) | (word << (32 - shift));
}

/* ---- Bitmap operations ---- */

static void bitmap_set_bit(uint32 *bitmap, int bit) {
    bitmap[bit / 32] |= (uint32)1 << (bit % 32);
}

static int bitmap_get_bit(const uint32 *bitmap, int bit) {
    return (bitmap[bit / 32] >> (bit % 32)) & 1;
}

static void bitmap_or(const uint32 *a, const uint32 *b, uint32 *out, int nwords) {
    for (int i = 0; i < nwords; i++) {
        out[i] = a[i] | b[i];
    }
}

static void bitmap_and(const uint32 *a, const uint32 *b, uint32 *out, int nwords) {
    for (int i = 0; i < nwords; i++) {
        out[i] = a[i] & b[i];
    }
}

static void bitmap_xor(const uint32 *a, const uint32 *b, uint32 *out, int nwords) {
    for (int i = 0; i < nwords; i++) {
        out[i] = a[i] ^ b[i];
    }
}

static void bitmap_andnot(const uint32 *a, const uint32 *b, uint32 *out, int nwords) {
    for (int i = 0; i < nwords; i++) {
        out[i] = a[i] & ~b[i];
    }
}

/* Count total set bits in bitmap */
static int bitmap_popcount(const uint32 *bitmap, int nwords) {
    int total = 0;
    for (int i = 0; i < nwords; i++) {
        uint32 v = bitmap[i];
        /* Kernighan's */
        while (v) {
            v &= v - 1;
            total++;
        }
    }
    return total;
}

/* Find first set bit in bitmap, returns -1 if none */
static int bitmap_ffs(const uint32 *bitmap, int nwords) {
    for (int i = 0; i < nwords; i++) {
        if (bitmap[i]) {
            return i * 32 + bitfield_ffs(bitmap[i]) - 1;
        }
    }
    return -1;
}

/* Count number of set bits in a region [start_bit, start_bit + count) */
static int bitmap_popcount_region(const uint32 *bitmap, int start_bit, int count) {
    int total = 0;
    for (int i = 0; i < count; i++) {
        int bit = start_bit + i;
        total += bitmap_get_bit(bitmap, bit);
    }
    return total;
}

/* ---- Bitwise hashing ---- */

/* Simple xorshift hash */
static uint32 xorshift_hash(uint32 x) {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return x;
}

/* FNV-1a style hash on a sequence of uint32s */
static uint32 fnv_hash(const uint32 *data, int n) {
    uint32 hash = 2166136261u;
    for (int i = 0; i < n; i++) {
        hash ^= data[i];
        hash *= 16777619u;
    }
    return hash;
}

/* Murmur-style mixing */
static uint32 murmur_mix(uint32 h) {
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return h;
}

static long long workload(uint32 *data) {
    long long total = 0;
    int i;

    /* Bitfield operations on each word */
    for (i = 0; i < BITMAP_WORDS; i++) {
        uint32 w = data[i];
        /* Extract fields, modify, repack */
        uint32 lo = bitfield_get(w, 0, 8);
        uint32 mid = bitfield_get(w, 8, 8);
        uint32 hi = bitfield_get(w, 16, 8);
        uint32 top = bitfield_get(w, 24, 8);

        w = bitfield_set(w, 0, 8, mid);
        w = bitfield_set(w, 8, 8, lo);
        w = bitfield_toggle(w, 16, 8);

        total += bitfield_popcount(w, 0, 16);
        total += bitfield_ffs(w);
        total += bitfield_bsr(w);

        data[i] = bit_reverse(w);
    }

    /* Rotation operations */
    for (i = 0; i < BITMAP_WORDS; i++) {
        uint32 w = data[i];
        w = bit_rotl(w, 7);
        w = bit_rotr(w, 3);
        w = bit_rotl(w, i & 15);
        total += w;
        data[i] = w;
    }

    /* Bitmap operations */
    uint32 bm_a[BITMAP_WORDS], bm_b[BITMAP_WORDS], bm_out[BITMAP_WORDS];
    memset(bm_a, 0, sizeof(bm_a));
    memset(bm_b, 0, sizeof(bm_b));

    /* Populate bitmaps from data */
    for (i = 0; i < NUM_QUERIES; i++) {
        int bit_a = data[i % BITMAP_WORDS] % BITMAP_BITS;
        int bit_b = (data[i % BITMAP_WORDS] >> 8) % BITMAP_BITS;
        bitmap_set_bit(bm_a, bit_a);
        bitmap_set_bit(bm_b, bit_b);
    }

    bitmap_or(bm_a, bm_b, bm_out, BITMAP_WORDS);
    total += bitmap_popcount(bm_out, BITMAP_WORDS);

    bitmap_and(bm_a, bm_b, bm_out, BITMAP_WORDS);
    total += bitmap_popcount(bm_out, BITMAP_WORDS);

    bitmap_xor(bm_a, bm_b, bm_out, BITMAP_WORDS);
    total += bitmap_popcount(bm_out, BITMAP_WORDS);

    bitmap_andnot(bm_a, bm_b, bm_out, BITMAP_WORDS);
    total += bitmap_popcount(bm_out, BITMAP_WORDS);

    total += bitmap_ffs(bm_out, BITMAP_WORDS);

    /* Region popcount queries */
    for (i = 0; i < 50; i++) {
        int start = (data[i % BITMAP_WORDS] % (BITMAP_BITS - 64));
        total += bitmap_popcount_region(bm_a, start, 64);
    }

    /* Hashing operations */
    for (i = 0; i < BITMAP_WORDS; i++) {
        total += xorshift_hash(data[i]);
        total += murmur_mix(data[i]);
    }
    total += fnv_hash(data, BITMAP_WORDS);

    /* Chained hash — output feeds back as input */
    uint32 chain = 0;
    for (i = 0; i < NUM_QUERIES; i++) {
        chain = xorshift_hash(chain ^ data[i % BITMAP_WORDS]);
        chain = murmur_mix(chain);
        total += chain;
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    uint32 *data = (uint32 *)malloc(BITMAP_WORDS * sizeof(uint32));

    bench_lcg_seed(12345);
    for (int i = 0; i < BITMAP_WORDS; i++) {
        data[i] = (bench_lcg_rand() << 17) | bench_lcg_rand();
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data); });

    free(data);
    return 0;
}
