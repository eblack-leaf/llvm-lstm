/*
 * Targets: loop-unroll (match extension), licm (window pointer),
 * dse (output buffer overwrites), gvn (repeated hash lookups),
 * instcombine (hash computations).
 *
 * LZ77-style sliding window compression and decompression.
 */
#include "bench_timing.h"

#define DATA_SIZE 4000
#define WINDOW_SIZE 1024
#define MIN_MATCH 3
#define MAX_MATCH 258
#define HASH_SIZE 4096
#define HASH_MASK (HASH_SIZE - 1)
#define MAX_OUTPUT (DATA_SIZE * 2)

/* ---- Hash function for 3-byte sequences ---- */

static unsigned int hash3(const unsigned char *p) {
    return ((unsigned int)p[0] * 31 * 31 +
            (unsigned int)p[1] * 31 +
            (unsigned int)p[2]) & HASH_MASK;
}

/* Alternative hash with different mixing */
static unsigned int hash3_alt(const unsigned char *p) {
    unsigned int h = (unsigned int)p[0];
    h = (h << 5) ^ (h >> 3) ^ p[1];
    h = (h << 5) ^ (h >> 3) ^ p[2];
    return h & HASH_MASK;
}

/* ---- Greedy LZ77 compression ---- */

typedef struct {
    unsigned char type; /* 0 = literal, 1 = match */
    unsigned char literal;
    unsigned short offset;
    unsigned short length;
} LZToken;

static int lz_compress_greedy(const unsigned char *data, int len,
                              LZToken *tokens, int max_tokens) {
    int hash_table[HASH_SIZE];
    memset(hash_table, -1, sizeof(hash_table));

    int pos = 0, ntokens = 0;

    while (pos < len && ntokens < max_tokens) {
        int best_len = 0, best_off = 0;

        if (pos + MIN_MATCH <= len) {
            unsigned int h = hash3(data + pos);
            int candidate = hash_table[h];

            /* Search backward in the window */
            while (candidate >= 0 && pos - candidate <= WINDOW_SIZE) {
                /* Extend match */
                int mlen = 0;
                while (mlen < MAX_MATCH && pos + mlen < len &&
                       data[candidate + mlen] == data[pos + mlen]) {
                    mlen++;
                }
                if (mlen >= MIN_MATCH && mlen > best_len) {
                    best_len = mlen;
                    best_off = pos - candidate;
                }
                /* Chain to previous entry (simplified: just break) */
                break;
            }

            hash_table[h] = pos;
        }

        if (best_len >= MIN_MATCH) {
            tokens[ntokens].type = 1;
            tokens[ntokens].offset = (unsigned short)best_off;
            tokens[ntokens].length = (unsigned short)best_len;
            ntokens++;
            /* Update hash for all positions in the match */
            for (int i = 1; i < best_len && pos + i + 2 < len; i++) {
                hash_table[hash3(data + pos + i)] = pos + i;
            }
            pos += best_len;
        } else {
            tokens[ntokens].type = 0;
            tokens[ntokens].literal = data[pos];
            ntokens++;
            pos++;
        }
    }
    return ntokens;
}

/* ---- Lazy LZ77: look ahead one position for a better match ---- */

static int lz_compress_lazy(const unsigned char *data, int len,
                            LZToken *tokens, int max_tokens) {
    int hash_table[HASH_SIZE];
    memset(hash_table, -1, sizeof(hash_table));

    int pos = 0, ntokens = 0;

    while (pos < len && ntokens < max_tokens) {
        int best_len = 0, best_off = 0;

        if (pos + MIN_MATCH <= len) {
            unsigned int h = hash3(data + pos);
            int candidate = hash_table[h];

            if (candidate >= 0 && pos - candidate <= WINDOW_SIZE) {
                int mlen = 0;
                while (mlen < MAX_MATCH && pos + mlen < len &&
                       data[candidate + mlen] == data[pos + mlen])
                    mlen++;
                if (mlen >= MIN_MATCH) {
                    best_len = mlen;
                    best_off = pos - candidate;
                }
            }
            hash_table[h] = pos;
        }

        /* Lazy evaluation: check if next position has a better match */
        if (best_len >= MIN_MATCH && pos + 1 + MIN_MATCH <= len) {
            unsigned int h2 = hash3(data + pos + 1);
            int cand2 = hash_table[h2];
            if (cand2 >= 0 && pos + 1 - cand2 <= WINDOW_SIZE) {
                int mlen2 = 0;
                while (mlen2 < MAX_MATCH && pos + 1 + mlen2 < len &&
                       data[cand2 + mlen2] == data[pos + 1 + mlen2])
                    mlen2++;
                if (mlen2 > best_len + 1) {
                    /* Emit literal for current position, use better match next */
                    tokens[ntokens].type = 0;
                    tokens[ntokens].literal = data[pos];
                    ntokens++;
                    pos++;
                    best_len = mlen2;
                    best_off = pos - cand2;
                }
            }
            hash_table[h2] = pos + 1;
        }

        if (best_len >= MIN_MATCH) {
            tokens[ntokens].type = 1;
            tokens[ntokens].offset = (unsigned short)best_off;
            tokens[ntokens].length = (unsigned short)best_len;
            ntokens++;
            for (int i = 1; i < best_len && pos + i + 2 < len; i++) {
                hash_table[hash3(data + pos + i)] = pos + i;
            }
            pos += best_len;
        } else {
            tokens[ntokens].type = 0;
            tokens[ntokens].literal = data[pos];
            ntokens++;
            pos++;
        }
    }
    return ntokens;
}

/* ---- Decompression ---- */

static int lz_decompress(const LZToken *tokens, int ntokens,
                         unsigned char *out, int max_out) {
    int pos = 0;
    for (int i = 0; i < ntokens && pos < max_out; i++) {
        if (tokens[i].type == 0) {
            out[pos++] = tokens[i].literal;
        } else {
            int off = tokens[i].offset;
            int len = tokens[i].length;
            for (int j = 0; j < len && pos < max_out; j++) {
                out[pos] = out[pos - off];
                pos++;
            }
        }
    }
    return pos;
}

/* ---- Serialization: tokens to byte stream ---- */

static int serialize_tokens(const LZToken *tokens, int ntokens,
                            unsigned char *out, int max_out) {
    int op = 0;
    for (int i = 0; i < ntokens && op + 4 < max_out; i++) {
        if (tokens[i].type == 0) {
            out[op++] = 0; /* flag: literal */
            out[op++] = tokens[i].literal;
        } else {
            out[op++] = 1; /* flag: match */
            out[op++] = (unsigned char)(tokens[i].offset & 0xFF);
            out[op++] = (unsigned char)(tokens[i].offset >> 8);
            out[op++] = (unsigned char)(tokens[i].length);
        }
    }
    return op;
}

/* ---- Statistics ---- */

static void compute_stats(const LZToken *tokens, int ntokens,
                          int *n_literals, int *n_matches,
                          int *total_match_len) {
    *n_literals = 0;
    *n_matches = 0;
    *total_match_len = 0;
    for (int i = 0; i < ntokens; i++) {
        if (tokens[i].type == 0)
            (*n_literals)++;
        else {
            (*n_matches)++;
            *total_match_len += tokens[i].length;
        }
    }
}

/* Verify decompressed output matches original */
static int verify(const unsigned char *original, const unsigned char *decompressed,
                  int len) {
    for (int i = 0; i < len; i++) {
        if (original[i] != decompressed[i]) return 0;
    }
    return 1;
}

static long long workload(unsigned char *data, unsigned char *output) {
    long long total = 0;
    LZToken *tokens = (LZToken *)malloc(DATA_SIZE * sizeof(LZToken));

    /* Greedy compression */
    int ntokens = lz_compress_greedy(data, DATA_SIZE, tokens, DATA_SIZE);
    total += ntokens;

    int n_lit, n_match, match_len;
    compute_stats(tokens, ntokens, &n_lit, &n_match, &match_len);
    total += n_lit + n_match + match_len;

    int dec_len = lz_decompress(tokens, ntokens, output, DATA_SIZE);
    total += dec_len;
    total += verify(data, output, dec_len);

    int ser_len = serialize_tokens(tokens, ntokens, output, MAX_OUTPUT);
    total += ser_len;

    /* Lazy compression */
    ntokens = lz_compress_lazy(data, DATA_SIZE, tokens, DATA_SIZE);
    total += ntokens;

    compute_stats(tokens, ntokens, &n_lit, &n_match, &match_len);
    total += n_lit + n_match + match_len;

    dec_len = lz_decompress(tokens, ntokens, output, DATA_SIZE);
    total += dec_len;
    total += verify(data, output, dec_len);

    ser_len = serialize_tokens(tokens, ntokens, output, MAX_OUTPUT);
    total += ser_len;

    /* Compress the serialized output (2nd pass) */
    int ntokens2 = lz_compress_greedy(output, ser_len, tokens, DATA_SIZE);
    total += ntokens2;

    /* Alternative hash function compression */
    {
        int alt_hash_table[HASH_SIZE];
        memset(alt_hash_table, -1, sizeof(alt_hash_table));
        int pos = 0, alt_tokens = 0;
        while (pos + MIN_MATCH <= DATA_SIZE) {
            unsigned int h = hash3_alt(data + pos);
            int cand = alt_hash_table[h];
            int mlen = 0;
            if (cand >= 0 && pos - cand <= WINDOW_SIZE) {
                while (mlen < MAX_MATCH && pos + mlen < DATA_SIZE &&
                       data[cand + mlen] == data[pos + mlen])
                    mlen++;
            }
            alt_hash_table[h] = pos;
            if (mlen >= MIN_MATCH) {
                alt_tokens++;
                pos += mlen;
            } else {
                pos++;
            }
        }
        total += alt_tokens;
    }

    free(tokens);
    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    unsigned char *data = (unsigned char *)malloc(DATA_SIZE);
    unsigned char *output = (unsigned char *)malloc(MAX_OUTPUT);

    /* Generate data with some repetition (makes LZ interesting) */
    bench_lcg_seed(12345);
    for (int i = 0; i < DATA_SIZE; i++) {
        unsigned int r = bench_lcg_rand();
        if (r % 5 == 0 && i >= 32) {
            /* Copy from earlier in the buffer */
            int src = i - 1 - (r % 32);
            if (src < 0) src = 0;
            int run = 3 + r % 15;
            for (int j = 0; j < run && i + j < DATA_SIZE; j++) {
                data[i + j] = data[src + (j % (i - src))];
            }
            i += run - 1;
        } else {
            data[i] = (unsigned char)(r % 128); /* ASCII-ish */
        }
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data, output); });

    free(data);
    free(output);
    return 0;
}
