/*
 * Targets: loop-unroll (encode/decode loops), simplifycfg (branch chains for
 * run detection), dse (buffer writes), early-cse (repeated comparisons).
 *
 * Run-length encoding/decoding with multiple variants.
 */
#include "bench_timing.h"

#define DATA_SIZE 4000
#define MAX_ENCODED (DATA_SIZE * 2)

/* ---- Basic RLE: byte + count pairs ---- */

/* Encode: runs of identical bytes become (byte, count) */
static int rle_encode(const unsigned char *in, int inlen,
                      unsigned char *out, int maxout) {
    int ip = 0, op = 0;
    while (ip < inlen && op + 1 < maxout) {
        unsigned char val = in[ip];
        int run = 1;
        while (ip + run < inlen && in[ip + run] == val && run < 255) {
            run++;
        }
        out[op++] = val;
        out[op++] = (unsigned char)run;
        ip += run;
    }
    return op;
}

/* Decode: (byte, count) pairs back to raw bytes */
static int rle_decode(const unsigned char *in, int inlen,
                      unsigned char *out, int maxout) {
    int ip = 0, op = 0;
    while (ip + 1 < inlen && op < maxout) {
        unsigned char val = in[ip];
        int count = in[ip + 1];
        for (int i = 0; i < count && op < maxout; i++) {
            out[op++] = val;
        }
        ip += 2;
    }
    return op;
}

/* ---- Delta RLE: encode differences between consecutive bytes ---- */

static int rle_encode_delta(const unsigned char *in, int inlen,
                            unsigned char *out, int maxout) {
    if (inlen == 0) return 0;
    /* First byte stored directly */
    int ip = 1, op = 0;
    out[op++] = in[0];

    /* Compute deltas, then RLE on deltas */
    while (ip < inlen && op + 1 < maxout) {
        int delta = (int)in[ip] - (int)in[ip - 1];
        unsigned char d = (unsigned char)(delta + 128);
        int run = 1;
        while (ip + run < inlen && op + 1 < maxout) {
            int next_delta = (int)in[ip + run] - (int)in[ip + run - 1];
            unsigned char nd = (unsigned char)(next_delta + 128);
            if (nd != d || run >= 255) break;
            run++;
        }
        out[op++] = d;
        out[op++] = (unsigned char)run;
        ip += run;
    }
    return op;
}

static int rle_decode_delta(const unsigned char *in, int inlen,
                            unsigned char *out, int maxout) {
    if (inlen == 0) return 0;
    int ip = 0, op = 0;
    out[op++] = in[ip++]; /* First byte */

    while (ip + 1 < inlen && op < maxout) {
        int delta = (int)in[ip] - 128;
        int count = in[ip + 1];
        for (int i = 0; i < count && op < maxout; i++) {
            out[op] = (unsigned char)((int)out[op - 1] + delta);
            op++;
        }
        ip += 2;
    }
    return op;
}

/* ---- PackBits-style encoding (Mac RLE) ---- */
/* Positive count = literal run, negative count = repeated byte */

static int packbits_encode(const unsigned char *in, int inlen,
                           unsigned char *out, int maxout) {
    int ip = 0, op = 0;

    while (ip < inlen && op < maxout - 2) {
        /* Check for a run of identical bytes */
        int run = 1;
        while (ip + run < inlen && in[ip + run] == in[ip] && run < 128) {
            run++;
        }

        if (run >= 3) {
            /* Encode as repeat: -(run-1), byte */
            out[op++] = (unsigned char)(-(run - 1) & 0xFF);
            out[op++] = in[ip];
            ip += run;
        } else {
            /* Literal run: count consecutive non-repeating bytes */
            int lit_start = ip;
            int lit_len = 0;
            while (ip < inlen && lit_len < 128 && op + lit_len + 1 < maxout) {
                /* Check if next position starts a run */
                int next_run = 1;
                while (ip + next_run < inlen && in[ip + next_run] == in[ip]
                       && next_run < 3) {
                    next_run++;
                }
                if (next_run >= 3 && lit_len > 0) break;
                ip++;
                lit_len++;
                if (next_run >= 3) break;
            }
            out[op++] = (unsigned char)(lit_len - 1);
            for (int i = 0; i < lit_len && op < maxout; i++) {
                out[op++] = in[lit_start + i];
            }
        }
    }
    return op;
}

static int packbits_decode(const unsigned char *in, int inlen,
                           unsigned char *out, int maxout) {
    int ip = 0, op = 0;
    while (ip < inlen && op < maxout) {
        signed char header = (signed char)in[ip++];
        if (header >= 0) {
            /* Literal run of (header + 1) bytes */
            int count = header + 1;
            for (int i = 0; i < count && ip < inlen && op < maxout; i++) {
                out[op++] = in[ip++];
            }
        } else if (header != -128) {
            /* Repeated byte, (1 - header) times */
            int count = 1 - header;
            if (ip < inlen) {
                unsigned char val = in[ip++];
                for (int i = 0; i < count && op < maxout; i++) {
                    out[op++] = val;
                }
            }
        }
        /* header == -128 is a no-op */
    }
    return op;
}

/* ---- Bit-packed RLE: variable-width encoding ---- */
/* Store (value:8, count:8) but pack using variable-length count */

static int rle_encode_varint(const unsigned char *in, int inlen,
                             unsigned char *out, int maxout) {
    int ip = 0, op = 0;
    while (ip < inlen && op < maxout) {
        unsigned char val = in[ip];
        int run = 1;
        while (ip + run < inlen && in[ip + run] == val && run < 16383)
            run++;

        out[op++] = val;
        /* Varint encode the count */
        if (run < 128) {
            if (op < maxout) out[op++] = (unsigned char)run;
        } else {
            if (op + 1 < maxout) {
                out[op++] = (unsigned char)(0x80 | (run & 0x7F));
                out[op++] = (unsigned char)(run >> 7);
            }
        }
        ip += run;
    }
    return op;
}

static int rle_decode_varint(const unsigned char *in, int inlen,
                             unsigned char *out, int maxout) {
    int ip = 0, op = 0;
    while (ip < inlen && op < maxout) {
        unsigned char val = in[ip++];
        int count = 0;
        if (ip < inlen) {
            unsigned char b = in[ip++];
            if (b & 0x80) {
                count = b & 0x7F;
                if (ip < inlen) count |= (int)in[ip++] << 7;
            } else {
                count = b;
            }
        }
        for (int i = 0; i < count && op < maxout; i++) {
            out[op++] = val;
        }
    }
    return op;
}

/* ---- Verify roundtrip ---- */
static int verify_match(const unsigned char *a, const unsigned char *b, int len) {
    for (int i = 0; i < len; i++) {
        if (a[i] != b[i]) return 0;
    }
    return 1;
}

static long long workload(unsigned char *data, unsigned char *encoded,
                          unsigned char *decoded) {
    long long total = 0;
    int enc_len, dec_len;

    /* Basic RLE encode + decode */
    enc_len = rle_encode(data, DATA_SIZE, encoded, MAX_ENCODED);
    total += enc_len;
    dec_len = rle_decode(encoded, enc_len, decoded, MAX_ENCODED);
    total += dec_len;
    total += verify_match(data, decoded, dec_len);

    /* Delta RLE encode + decode */
    enc_len = rle_encode_delta(data, DATA_SIZE, encoded, MAX_ENCODED);
    total += enc_len;
    dec_len = rle_decode_delta(encoded, enc_len, decoded, MAX_ENCODED);
    total += dec_len;
    total += verify_match(data, decoded, dec_len);

    /* PackBits encode + decode */
    enc_len = packbits_encode(data, DATA_SIZE, encoded, MAX_ENCODED);
    total += enc_len;
    dec_len = packbits_decode(encoded, enc_len, decoded, MAX_ENCODED);
    total += dec_len;
    total += verify_match(data, decoded, dec_len);

    /* Varint RLE encode + decode */
    enc_len = rle_encode_varint(data, DATA_SIZE, encoded, MAX_ENCODED);
    total += enc_len;
    dec_len = rle_decode_varint(encoded, enc_len, decoded, MAX_ENCODED);
    total += dec_len;
    total += verify_match(data, decoded, dec_len);

    /* Encode the encoded output again (multi-pass compression) */
    unsigned char *re_encoded = decoded; /* reuse buffer */
    enc_len = rle_encode(encoded, enc_len, re_encoded, MAX_ENCODED);
    total += enc_len;

    /* Sum encoded bytes for sink */
    for (int i = 0; i < enc_len; i++) {
        total += re_encoded[i];
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    unsigned char *data = (unsigned char *)malloc(DATA_SIZE);
    unsigned char *encoded = (unsigned char *)malloc(MAX_ENCODED);
    unsigned char *decoded = (unsigned char *)malloc(MAX_ENCODED);

    /* Generate data with some runs (makes RLE interesting) */
    bench_lcg_seed(12345);
    for (int i = 0; i < DATA_SIZE; i++) {
        /* Mix of random and run-heavy data */
        if (bench_lcg_rand() % 4 == 0) {
            /* Start a run */
            unsigned char val = (unsigned char)(bench_lcg_rand() % 256);
            int run_len = 2 + bench_lcg_rand() % 10;
            for (int j = 0; j < run_len && i + j < DATA_SIZE; j++) {
                data[i + j] = val;
            }
            i += run_len - 1;
        } else {
            data[i] = (unsigned char)(bench_lcg_rand() % 256);
        }
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data, encoded, decoded); });

    free(data);
    free(encoded);
    free(decoded);
    return 0;
}
