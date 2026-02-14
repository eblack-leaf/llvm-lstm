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

#define TEXT_SIZE (100 * 1024)  /* ~100 KB */
#define PAT_LEN 20

static char *text;
static char pattern[PAT_LEN + 1];
static int fail_table[PAT_LEN];

static void build_fail(const char *pat, int m, int *fail) {
    fail[0] = 0;
    int k = 0;
    for (int i = 1; i < m; i++) {
        while (k > 0 && pat[k] != pat[i])
            k = fail[k - 1];
        if (pat[k] == pat[i])
            k++;
        fail[i] = k;
    }
}

static int kmp_count(const char *txt, int n, const char *pat, int m, const int *fail) {
    int count = 0, q = 0;
    for (int i = 0; i < n; i++) {
        while (q > 0 && pat[q] != txt[i])
            q = fail[q - 1];
        if (pat[q] == txt[i])
            q++;
        if (q == m) {
            count++;
            q = fail[q - 1];
        }
    }
    return count;
}

static volatile int match_count;

static void do_kmp(void) {
    match_count = kmp_count(text, TEXT_SIZE, pattern, PAT_LEN, fail_table);
}

int main(void) {
    /* Generate ~10MB random lowercase text */
    text = (char *)malloc(TEXT_SIZE + 1);
    if (!text) { fprintf(stderr, "malloc failed\n"); return 1; }

    lcg_state = 12345;
    for (int i = 0; i < TEXT_SIZE; i++)
        text[i] = 'a' + (char)(lcg_rand() % 26);
    text[TEXT_SIZE] = '\0';

    /* Extract a pattern from a fixed position so it appears in the text */
    memcpy(pattern, text + 1000, PAT_LEN);
    pattern[PAT_LEN] = '\0';

    /* Also plant the pattern at regular intervals for more matches */
    for (int i = 0; i < TEXT_SIZE - PAT_LEN; i += 50000)
        memcpy(text + i, pattern, PAT_LEN);

    /* Build KMP failure table */
    build_fail(pattern, PAT_LEN, fail_table);

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_kmp();

    /* Timed runs */
    long long times[201];
    for (int t = 0; t < 201; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_kmp();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(text);
    return 0;
}
