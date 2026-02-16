#include "bench_timing.h"

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

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    /* Generate ~100KB random lowercase text */
    text = (char *)malloc(TEXT_SIZE + 1);
    if (!text) { fprintf(stderr, "malloc failed\n"); return 1; }

    bench_lcg_seed(12345);
    for (int i = 0; i < TEXT_SIZE; i++)
        text[i] = 'a' + (char)(bench_lcg_rand() % 26);
    text[TEXT_SIZE] = '\0';

    /* Extract a pattern from a fixed position so it appears in the text */
    memcpy(pattern, text + 1000, PAT_LEN);
    pattern[PAT_LEN] = '\0';

    /* Also plant the pattern at regular intervals for more matches */
    for (int i = 0; i < TEXT_SIZE - PAT_LEN; i += 50000)
        memcpy(text + i, pattern, PAT_LEN);

    /* Build KMP failure table */
    build_fail(pattern, PAT_LEN, fail_table);

    BENCH_TIME(niters, { do_kmp(); });

    free(text);
    return 0;
}
