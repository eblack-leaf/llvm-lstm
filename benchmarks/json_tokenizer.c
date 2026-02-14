#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>

static long long timespec_diff_ns(struct timespec *a, struct timespec *b) {
    return (long long)(b->tv_sec - a->tv_sec) * 1000000000LL + (b->tv_nsec - a->tv_nsec);
}

static int cmp_ll(const void *a, const void *b) {
    long long x = *(const long long *)a, y = *(const long long *)b;
    return (x > y) - (x < y);
}

static unsigned int lcg_state = 12345;
static unsigned int lcg_rand(void) { lcg_state = lcg_state * 1103515245 + 12345; return (lcg_state >> 16) & 0x7fff; }

#define BUF_SIZE (100 * 1024)

static char json_buf[BUF_SIZE + 1];
static int json_len;

static void generate_json(void) {
    int pos = 0;
    int depth = 0;
    lcg_state = 12345;

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
        int klen = 3 + lcg_rand() % 8;
        for (int i = 0; i < klen && pos < BUF_SIZE; i++)
            EMIT('a' + (char)(lcg_rand() % 26));
        EMIT('"');
        EMIT(':');

        int choice = lcg_rand() % 10;
        if (choice < 3 && depth < 5 && pos < BUF_SIZE - 500) {
            /* Nested object */
            EMIT('{');
            depth++;
            item = 0;
        } else if (choice < 5 && depth < 5 && pos < BUF_SIZE - 500) {
            /* Array of numbers */
            EMIT('[');
            int alen = 2 + lcg_rand() % 6;
            for (int i = 0; i < alen; i++) {
                if (i > 0) EMIT(',');
                char num[16];
                int n = (int)(lcg_rand() % 10000);
                int nlen = sprintf(num, "%d", n);
                for (int j = 0; j < nlen; j++) EMIT(num[j]);
            }
            EMIT(']');
        } else if (choice < 7) {
            /* String value */
            EMIT('"');
            int slen = 3 + lcg_rand() % 12;
            for (int i = 0; i < slen; i++)
                EMIT('a' + (char)(lcg_rand() % 26));
            EMIT('"');
        } else if (choice < 9) {
            /* Number */
            char num[16];
            int n = (int)(lcg_rand() % 100000);
            int nlen = sprintf(num, "%d", n);
            for (int j = 0; j < nlen; j++) EMIT(num[j]);
        } else {
            /* Boolean / null */
            if (lcg_rand() % 3 == 0) EMITS("null");
            else if (lcg_rand() % 2) EMITS("true");
            else EMITS("false");
        }

        /* Occasionally close a brace if nested */
        if (depth > 1 && lcg_rand() % 4 == 0) {
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

int main(void) {
    generate_json();

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_tokenize();

    /* Timed runs */
    long long times[50];
    for (int t = 0; t < 50; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_tokenize();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
