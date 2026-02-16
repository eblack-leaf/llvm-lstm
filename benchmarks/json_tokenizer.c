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
            while (i < json_len && json_buf[i] != '"') {
                if (json_buf[i] == '\\') i++;  /* skip escaped char */
                i++;
            }
            i++;  /* closing quote */
        } else if ((c >= '0' && c <= '9') || c == '-') {
            /* Number token */
            count++;
            i++;
            while (i < json_len && ((json_buf[i] >= '0' && json_buf[i] <= '9') || json_buf[i] == '.'))
                i++;
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

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    generate_json();

    BENCH_TIME(niters, { do_tokenize(); });
    return 0;
}
