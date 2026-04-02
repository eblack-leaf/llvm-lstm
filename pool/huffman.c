/*
 * Targets: inline (tree traversal helpers), simplifycfg (bit-level branches),
 * mem2reg (local tree nodes), dse (buffer writes), early-cse (repeated
 * frequency lookups).
 *
 * Huffman tree building + encoding/decoding.
 */
#include "bench_timing.h"

#define ALPHABET_SIZE 256
#define DATA_SIZE 4000
#define MAX_CODE_BITS 24
#define TREE_NODES (ALPHABET_SIZE * 2)

/* ---- Huffman tree node ---- */
typedef struct {
    int freq;
    int symbol;     /* -1 for internal nodes */
    int left, right; /* indices into node pool, -1 for leaves */
    int parent;
} HuffNode;

/* ---- Code table entry ---- */
typedef struct {
    unsigned int bits;
    int length;
} HuffCode;

/* ---- Priority queue for tree building (min-heap by frequency) ---- */
typedef struct {
    int nodes[TREE_NODES];
    int size;
} PriorityQueue;

static HuffNode tree_pool[TREE_NODES];
static int tree_size;

static void pq_init(PriorityQueue *pq) { pq->size = 0; }

static void pq_push(PriorityQueue *pq, int node_idx) {
    int i = pq->size++;
    pq->nodes[i] = node_idx;
    while (i > 0) {
        int parent = (i - 1) / 2;
        if (tree_pool[pq->nodes[i]].freq < tree_pool[pq->nodes[parent]].freq) {
            int tmp = pq->nodes[i];
            pq->nodes[i] = pq->nodes[parent];
            pq->nodes[parent] = tmp;
            i = parent;
        } else break;
    }
}

static int pq_pop(PriorityQueue *pq) {
    int result = pq->nodes[0];
    pq->size--;
    pq->nodes[0] = pq->nodes[pq->size];
    int i = 0;
    while (1) {
        int smallest = i;
        int left = 2 * i + 1, right = 2 * i + 2;
        if (left < pq->size &&
            tree_pool[pq->nodes[left]].freq < tree_pool[pq->nodes[smallest]].freq)
            smallest = left;
        if (right < pq->size &&
            tree_pool[pq->nodes[right]].freq < tree_pool[pq->nodes[smallest]].freq)
            smallest = right;
        if (smallest == i) break;
        int tmp = pq->nodes[i];
        pq->nodes[i] = pq->nodes[smallest];
        pq->nodes[smallest] = tmp;
        i = smallest;
    }
    return result;
}

/* ---- Frequency counting ---- */

static void build_freq_table(const unsigned char *data, int len, int *freq) {
    memset(freq, 0, ALPHABET_SIZE * sizeof(int));
    for (int i = 0; i < len; i++) {
        freq[data[i]]++;
    }
}

/* Frequency with bigram bonus: freq[c] += count(c followed by specific chars) */
static void build_freq_bigram(const unsigned char *data, int len, int *freq) {
    memset(freq, 0, ALPHABET_SIZE * sizeof(int));
    for (int i = 0; i < len; i++) {
        freq[data[i]]++;
    }
    /* Add bigram weight */
    for (int i = 0; i + 1 < len; i++) {
        int bigram = ((int)data[i] + (int)data[i + 1]) % ALPHABET_SIZE;
        freq[bigram]++;
    }
}

/* ---- Tree building ---- */

static int build_tree(const int *freq) {
    PriorityQueue pq;
    pq_init(&pq);
    tree_size = 0;

    /* Create leaf nodes for symbols with nonzero frequency */
    for (int i = 0; i < ALPHABET_SIZE; i++) {
        if (freq[i] > 0) {
            int idx = tree_size++;
            tree_pool[idx].freq = freq[i];
            tree_pool[idx].symbol = i;
            tree_pool[idx].left = -1;
            tree_pool[idx].right = -1;
            tree_pool[idx].parent = -1;
            pq_push(&pq, idx);
        }
    }

    /* Handle degenerate case */
    if (pq.size <= 1) return (pq.size == 1) ? pq.nodes[0] : -1;

    /* Build tree by merging two smallest nodes */
    while (pq.size > 1) {
        int left = pq_pop(&pq);
        int right = pq_pop(&pq);
        int parent = tree_size++;
        tree_pool[parent].freq = tree_pool[left].freq + tree_pool[right].freq;
        tree_pool[parent].symbol = -1;
        tree_pool[parent].left = left;
        tree_pool[parent].right = right;
        tree_pool[parent].parent = -1;
        tree_pool[left].parent = parent;
        tree_pool[right].parent = parent;
        pq_push(&pq, parent);
    }

    return pq_pop(&pq); /* root */
}

/* ---- Code generation ---- */

static void generate_codes_recursive(int node, unsigned int bits, int depth,
                                     HuffCode *codes) {
    if (node < 0 || depth > MAX_CODE_BITS) return;

    if (tree_pool[node].symbol >= 0) {
        codes[tree_pool[node].symbol].bits = bits;
        codes[tree_pool[node].symbol].length = depth;
        return;
    }

    generate_codes_recursive(tree_pool[node].left, bits << 1, depth + 1, codes);
    generate_codes_recursive(tree_pool[node].right, (bits << 1) | 1, depth + 1, codes);
}

static void build_codes(int root, HuffCode *codes) {
    memset(codes, 0, ALPHABET_SIZE * sizeof(HuffCode));
    if (root >= 0)
        generate_codes_recursive(root, 0, 0, codes);
}

/* ---- Encoding ---- */

/* Encode data into a bit stream. Returns total bits written. */
static int encode_stream(const unsigned char *data, int len,
                         const HuffCode *codes,
                         unsigned char *out, int max_out_bytes) {
    int bit_pos = 0;
    memset(out, 0, max_out_bytes);

    for (int i = 0; i < len; i++) {
        HuffCode c = codes[data[i]];
        if (c.length == 0) continue; /* symbol not in tree */

        for (int b = c.length - 1; b >= 0; b--) {
            int byte_idx = bit_pos / 8;
            int bit_idx = 7 - (bit_pos % 8);
            if (byte_idx >= max_out_bytes) return bit_pos;
            if ((c.bits >> b) & 1) {
                out[byte_idx] |= (1 << bit_idx);
            }
            bit_pos++;
        }
    }
    return bit_pos;
}

/* ---- Decoding ---- */

/* Decode a bit stream back to symbols using tree traversal. */
static int decode_stream(const unsigned char *encoded, int total_bits,
                         int root,
                         unsigned char *out, int max_out) {
    int op = 0;
    int node = root;

    for (int bit = 0; bit < total_bits && op < max_out; bit++) {
        int byte_idx = bit / 8;
        int bit_idx = 7 - (bit % 8);
        int b = (encoded[byte_idx] >> bit_idx) & 1;

        if (b == 0)
            node = tree_pool[node].left;
        else
            node = tree_pool[node].right;

        if (node < 0) break; /* corrupted */

        if (tree_pool[node].symbol >= 0) {
            out[op++] = (unsigned char)tree_pool[node].symbol;
            node = root;
        }
    }
    return op;
}

/* ---- Canonical Huffman variant ---- */

/* Sort codes by length, then by symbol, and reassign sequential bit patterns */
static void canonicalize_codes(HuffCode *codes) {
    /* Collect symbols with codes */
    int symbols[ALPHABET_SIZE];
    int nsyms = 0;
    for (int i = 0; i < ALPHABET_SIZE; i++) {
        if (codes[i].length > 0)
            symbols[nsyms++] = i;
    }

    /* Sort by (length, symbol) — simple insertion sort */
    for (int i = 1; i < nsyms; i++) {
        int key = symbols[i];
        int kl = codes[key].length;
        int j = i - 1;
        while (j >= 0 && (codes[symbols[j]].length > kl ||
               (codes[symbols[j]].length == kl && symbols[j] > key))) {
            symbols[j + 1] = symbols[j];
            j--;
        }
        symbols[j + 1] = key;
    }

    /* Assign canonical codes */
    unsigned int code = 0;
    int prev_len = 0;
    for (int i = 0; i < nsyms; i++) {
        int s = symbols[i];
        int len = codes[s].length;
        code <<= (len - prev_len);
        codes[s].bits = code;
        code++;
        prev_len = len;
    }
}

/* ---- Statistics ---- */

static double compression_ratio(int original_bits, int encoded_bits) {
    if (encoded_bits == 0) return 0.0;
    return (double)original_bits / (double)encoded_bits;
}

static double avg_code_length(const HuffCode *codes, const int *freq, int total_syms) {
    double weighted = 0.0;
    for (int i = 0; i < ALPHABET_SIZE; i++) {
        if (codes[i].length > 0 && total_syms > 0) {
            weighted += (double)freq[i] / total_syms * codes[i].length;
        }
    }
    return weighted;
}

/* Shannon entropy */
static double entropy(const int *freq, int total) {
    double h = 0.0;
    for (int i = 0; i < ALPHABET_SIZE; i++) {
        if (freq[i] > 0 && total > 0) {
            double p = (double)freq[i] / total;
            /* log2(p) approximation: log2(x) ~= (x-1) - (x-1)^2/2 for x near 1
               Use repeated squaring instead */
            double log2_p = 0.0;
            double val = p;
            for (int bit = -1; bit >= -20; bit--) {
                if (val < 1.0) {
                    val *= 2.0;
                    log2_p += bit;
                    if (val >= 1.0) {
                        val /= 2.0;
                        log2_p -= bit;
                        break;
                    }
                }
            }
            /* Rough approximation */
            log2_p = (p > 0.001) ? -(1.0 / p) : -20.0;
            h -= p * log2_p;
        }
    }
    return h;
}

static long long workload(unsigned char *data, unsigned char *encoded,
                          unsigned char *decoded) {
    long long total = 0;
    int freq[ALPHABET_SIZE];
    HuffCode codes[ALPHABET_SIZE];

    /* Standard Huffman */
    build_freq_table(data, DATA_SIZE, freq);
    int root = build_tree(freq);
    build_codes(root, codes);

    int total_bits = encode_stream(data, DATA_SIZE, codes,
                                   encoded, DATA_SIZE * 2);
    total += total_bits;

    int dec_len = decode_stream(encoded, total_bits, root,
                                decoded, DATA_SIZE);
    total += dec_len;

    /* Verify roundtrip */
    for (int i = 0; i < dec_len && i < DATA_SIZE; i++) {
        if (decoded[i] != data[i]) { total--; break; }
    }

    /* Statistics */
    total += (long long)(compression_ratio(DATA_SIZE * 8, total_bits) * 1000);
    total += (long long)(avg_code_length(codes, freq, DATA_SIZE) * 1000);
    total += (long long)(entropy(freq, DATA_SIZE) * 1000);

    /* Canonical Huffman variant */
    canonicalize_codes(codes);
    total_bits = encode_stream(data, DATA_SIZE, codes,
                               encoded, DATA_SIZE * 2);
    total += total_bits;
    dec_len = decode_stream(encoded, total_bits, root,
                            decoded, DATA_SIZE);
    total += dec_len;

    /* Bigram frequency Huffman */
    build_freq_bigram(data, DATA_SIZE, freq);
    root = build_tree(freq);
    build_codes(root, codes);
    total_bits = encode_stream(data, DATA_SIZE, codes,
                               encoded, DATA_SIZE * 2);
    total += total_bits;
    total += (long long)(avg_code_length(codes, freq, DATA_SIZE) * 1000);

    /* Encode the encoded output (meta-compression) */
    int enc_bytes = (total_bits + 7) / 8;
    build_freq_table(encoded, enc_bytes, freq);
    root = build_tree(freq);
    build_codes(root, codes);
    int meta_bits = encode_stream(encoded, enc_bytes, codes,
                                  decoded, DATA_SIZE * 2);
    total += meta_bits;

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    unsigned char *data = (unsigned char *)malloc(DATA_SIZE);
    unsigned char *encoded = (unsigned char *)malloc(DATA_SIZE * 2);
    unsigned char *decoded = (unsigned char *)malloc(DATA_SIZE * 2);

    /* Generate data with skewed distribution (makes Huffman interesting) */
    bench_lcg_seed(12345);
    for (int i = 0; i < DATA_SIZE; i++) {
        unsigned int r = bench_lcg_rand();
        /* Bias toward lower byte values */
        if (r % 3 == 0) data[i] = (unsigned char)(r % 32);
        else if (r % 3 == 1) data[i] = (unsigned char)(r % 64);
        else data[i] = (unsigned char)(r % 256);
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data, encoded, decoded); });

    free(data);
    free(encoded);
    free(decoded);
    return 0;
}
