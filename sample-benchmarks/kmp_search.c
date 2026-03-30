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

/* --- Variant 1: Naive brute-force string search --- */

static int naive_count(const char *txt, int n, const char *pat, int m) {
    int count = 0;
    int i, j;
    for (i = 0; i <= n - m; i++) {
        j = 0;
        while (j < m && txt[i + j] == pat[j])
            j++;
        if (j == m)
            count++;
    }
    return count;
}

/* --- Variant 2: Rabin-Karp rolling hash --- */

static int rabin_karp_count(const char *txt, int n, const char *pat, int m) {
    if (m > n) return 0;
    unsigned long long base = 256ULL, mod = 1000000007ULL;
    unsigned long long h = 1;
    int i;
    for (i = 0; i < m - 1; i++) h = (h * base) % mod;

    unsigned long long pat_hash = 0, txt_hash = 0;
    for (i = 0; i < m; i++) {
        pat_hash = (pat_hash * base + (unsigned char)pat[i]) % mod;
        txt_hash = (txt_hash * base + (unsigned char)txt[i]) % mod;
    }

    int count = 0;
    for (i = 0; i <= n - m; i++) {
        if (txt_hash == pat_hash) {
            /* Verify on hash collision */
            int j = 0;
            while (j < m && txt[i + j] == pat[j]) j++;
            if (j == m) count++;
        }
        if (i < n - m) {
            txt_hash = (txt_hash + mod - (unsigned char)txt[i] * h % mod) % mod;
            txt_hash = (txt_hash * base + (unsigned char)txt[i + m]) % mod;
        }
    }
    return count;
}

/* --- Variant 3: Boyer-Moore-Horspool --- */

static int bmh_count(const char *txt, int n, const char *pat, int m) {
    if (m > n) return 0;
    int skip[256];
    int i;
    for (i = 0; i < 256; i++) skip[i] = m;
    for (i = 0; i < m - 1; i++) skip[(unsigned char)pat[i]] = m - 1 - i;

    int count = 0;
    int pos = 0;
    while (pos <= n - m) {
        int j = m - 1;
        while (j >= 0 && txt[pos + j] == pat[j])
            j--;
        if (j < 0) {
            count++;
            pos += skip[(unsigned char)txt[pos + m - 1]];
        } else {
            pos += skip[(unsigned char)txt[pos + m - 1]];
        }
    }
    return count;
}

static volatile int match_count;

static void do_search(void) {
    int total = 0;
    total += kmp_count(text, TEXT_SIZE, pattern, PAT_LEN, fail_table);
    total += naive_count(text, TEXT_SIZE, pattern, PAT_LEN);
    total += rabin_karp_count(text, TEXT_SIZE, pattern, PAT_LEN);
    total += bmh_count(text, TEXT_SIZE, pattern, PAT_LEN);
    match_count = total;
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

    BENCH_TIME(niters, { do_search(); });

    free(text);
    return 0;
}
