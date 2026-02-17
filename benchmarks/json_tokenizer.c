#include "bench_timing.h"

#define BUF_SIZE (20 * 1024)

static char json_buf[BUF_SIZE + 1];
static int json_len;

static void generate_json(void) {
    int pos = 0;
    int depth = 0;
    bench_lcg_seed(12345);

#define EMIT(c) do { if (pos < BUF_SIZE) json_buf[pos++] = (c); } while(0)
#define EMITS(s) do { const char *_s = (s); while (*_s && pos < BUF_SIZE) json_buf[pos++] = *_s++; } while(0)

    EMIT('{');
    depth++;

    int item = 0;
    while (pos < BUF_SIZE - 200 && depth > 0) {
        if (item > 0) EMIT(',');
        item++;

        /* Key */
        EMIT('"');
        int klen = 3 + bench_lcg_rand() % 8;
        for (int i = 0; i < klen && pos < BUF_SIZE; i++)
            EMIT('a' + (char)(bench_lcg_rand() % 26));
        EMIT('"');
        EMIT(':');

        int choice = bench_lcg_rand() % 10;
        if (choice < 3 && depth < 5 && pos < BUF_SIZE - 500) {
            /* Nested object */
            EMIT('{');
            depth++;
            item = 0;
        } else if (choice < 5 && depth < 5 && pos < BUF_SIZE - 500) {
            /* Array of numbers */
            EMIT('[');
            int alen = 2 + bench_lcg_rand() % 6;
            for (int i = 0; i < alen; i++) {
                if (i > 0) EMIT(',');
                char num[16];
                int n = (int)(bench_lcg_rand() % 10000);
                int nlen = sprintf(num, "%d", n);
                for (int j = 0; j < nlen; j++) EMIT(num[j]);
            }
            EMIT(']');
        } else if (choice < 7) {
            /* String value */
            EMIT('"');
            int slen = 3 + bench_lcg_rand() % 12;
            for (int i = 0; i < slen; i++)
                EMIT('a' + (char)(bench_lcg_rand() % 26));
            EMIT('"');
        } else if (choice < 9) {
            /* Number */
            char num[16];
            int n = (int)(bench_lcg_rand() % 100000);
            int nlen = sprintf(num, "%d", n);
            for (int j = 0; j < nlen; j++) EMIT(num[j]);
        } else {
            /* Boolean / null */
            if (bench_lcg_rand() % 3 == 0) EMITS("null");
            else if (bench_lcg_rand() % 2) EMITS("true");
            else EMITS("false");
        }

        /* Occasionally close a brace if nested */
        if (depth > 1 && bench_lcg_rand() % 4 == 0) {
            EMIT('}');
            depth--;
        }
    }

    /* Close all open braces */
    while (depth > 0) {
        EMIT('}');
        depth--;
    }

    json_buf[pos] = '\0';
    json_len = pos;

#undef EMIT
#undef EMITS
}

/* --- Extracted helpers --- */

static int skip_string(const char *buf, int i, int len) {
    while (i < len && buf[i] != '"') {
        if (buf[i] == '\\') i++;
        i++;
    }
    return i + 1; /* past closing quote */
}

static int skip_number(const char *buf, int i, int len) {
    while (i < len && ((buf[i] >= '0' && buf[i] <= '9') || buf[i] == '.'))
        i++;
    return i;
}

/* Simple tokenizer: counts {, }, [, ], strings, numbers, true/false/null, colons, commas */
static volatile int token_count;

static void do_tokenize(void) {
    int count = 0;
    int i = 0;
    while (i < json_len) {
        char c = json_buf[i];
        if (c == '{' || c == '}' || c == '[' || c == ']' || c == ':' || c == ',') {
            count++;
            i++;
        } else if (c == '"') {
            /* String token */
            count++;
            i++;
            i = skip_string(json_buf, i, json_len);
        } else if ((c >= '0' && c <= '9') || c == '-') {
            /* Number token */
            count++;
            i++;
            i = skip_number(json_buf, i, json_len);
        } else if (c == 't') {
            count++; i += 4;  /* true */
        } else if (c == 'f') {
            count++; i += 5;  /* false */
        } else if (c == 'n') {
            count++; i += 4;  /* null */
        } else {
            i++;  /* whitespace or other */
        }
    }
    token_count = count;
}

/* Tokenizer with 8 separate counters */
static void tokenize_with_counts(const char *buf, int len, int *counts) {
    for (int j = 0; j < 8; j++) counts[j] = 0;
    int i = 0;
    while (i < len) {
        char c = buf[i];
        if (c == '{' || c == '}') {
            counts[0]++;
            i++;
        } else if (c == '[' || c == ']') {
            counts[1]++;
            i++;
        } else if (c == '"') {
            counts[2]++;
            i++;
            i = skip_string(buf, i, len);
        } else if ((c >= '0' && c <= '9') || c == '-') {
            counts[3]++;
            i++;
            i = skip_number(buf, i, len);
        } else if (c == 't') {
            counts[4]++;
            i += 4;
        } else if (c == 'f') {
            counts[4]++;
            i += 5;
        } else if (c == 'n') {
            counts[5]++;
            i += 4;
        } else if (c == ':') {
            counts[6]++;
            i++;
        } else if (c == ',') {
            counts[7]++;
            i++;
        } else {
            i++;
        }
    }
}

/* Validate JSON structure: track depth, return -1 if invalid */
static int tokenize_validate(const char *buf, int len) {
    int depth = 0;
    int i = 0;
    while (i < len) {
        char c = buf[i];
        if (c == '{' || c == '[') {
            depth++;
            i++;
        } else if (c == '}' || c == ']') {
            depth--;
            if (depth < 0) return -1;
            i++;
        } else if (c == '"') {
            i++;
            i = skip_string(buf, i, len);
        } else if ((c >= '0' && c <= '9') || c == '-') {
            i++;
            i = skip_number(buf, i, len);
        } else if (c == 't') {
            i += 4;
        } else if (c == 'f') {
            i += 5;
        } else if (c == 'n') {
            i += 4;
        } else {
            i++;
        }
    }
    return depth;
}

/* Scan for a specific key string, return count of matches */
static int tokenize_find_key(const char *buf, int len, const char *key, int keylen) {
    int found = 0;
    int i = 0;
    while (i < len) {
        char c = buf[i];
        if (c == '"') {
            i++;
            int start = i;
            i = skip_string(buf, i, len);
            int slen = i - start - 1; /* length without closing quote */
            if (slen == keylen) {
                int match = 1;
                for (int j = 0; j < keylen; j++) {
                    if (buf[start + j] != key[j]) {
                        match = 0;
                        break;
                    }
                }
                if (match) found++;
            }
        } else if ((c >= '0' && c <= '9') || c == '-') {
            i++;
            i = skip_number(buf, i, len);
        } else if (c == 't') {
            i += 4;
        } else if (c == 'f') {
            i += 5;
        } else if (c == 'n') {
            i += 4;
        } else {
            i++;
        }
    }
    return found;
}

/* Extract numeric token values into output array */
static int tokenize_extract_numbers(const char *buf, int len, int *out, int max_out) {
    int count = 0;
    int i = 0;
    while (i < len) {
        char c = buf[i];
        if ((c >= '0' && c <= '9') || c == '-') {
            int neg = 0;
            if (c == '-') { neg = 1; i++; }
            int val = 0;
            while (i < len && buf[i] >= '0' && buf[i] <= '9') {
                val = val * 10 + (buf[i] - '0');
                i++;
            }
            /* skip fractional part */
            if (i < len && buf[i] == '.') {
                i++;
                while (i < len && buf[i] >= '0' && buf[i] <= '9') i++;
            }
            if (neg) val = -val;
            if (count < max_out) {
                out[count] = val;
                count++;
            }
        } else if (c == '"') {
            i++;
            i = skip_string(buf, i, len);
        } else if (c == 't') {
            i += 4;
        } else if (c == 'f') {
            i += 5;
        } else if (c == 'n') {
            i += 4;
        } else {
            i++;
        }
    }
    return count;
}

/* Sum the lengths of all string tokens */
static int tokenize_sum_string_lengths(const char *buf, int len) {
    int total = 0;
    int i = 0;
    while (i < len) {
        char c = buf[i];
        if (c == '"') {
            i++;
            int start = i;
            while (i < len && buf[i] != '"') {
                if (buf[i] == '\\') i++;
                i++;
            }
            total += (i - start);
            i++; /* past closing quote */
        } else if ((c >= '0' && c <= '9') || c == '-') {
            i++;
            i = skip_number(buf, i, len);
        } else if (c == 't') {
            i += 4;
        } else if (c == 'f') {
            i += 5;
        } else if (c == 'n') {
            i += 4;
        } else {
            i++;
        }
    }
    return total;
}

/* Combined workload calling all tokenizer variants */
static long long workload(void) {
    long long result = 0;

    do_tokenize();
    result += token_count;

    int counts[8];
    tokenize_with_counts(json_buf, json_len, counts);
    for (int j = 0; j < 8; j++) result += counts[j];

    result += tokenize_validate(json_buf, json_len);

    result += tokenize_find_key(json_buf, json_len, "abc", 3);

    int num_buf[1000];
    int ncount = tokenize_extract_numbers(json_buf, json_len, num_buf, 1000);
    result += ncount;

    result += tokenize_sum_string_lengths(json_buf, json_len);

    return result;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    generate_json();

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(); });
    (void)sink;
    return 0;
}
