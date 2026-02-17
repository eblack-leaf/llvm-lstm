#include "bench_timing.h"

#define STR_LEN 200

static void generate_random_string(char *s, int len) {
    for (int i = 0; i < len; i++) {
        s[i] = 'a' + (bench_lcg_rand() % 26);
    }
    s[len] = '\0';
}

static int levenshtein(const char *s, int slen, const char *t, int tlen) {
    int *prev = (int *)malloc((tlen + 1) * sizeof(int));
    int *curr = (int *)malloc((tlen + 1) * sizeof(int));

    for (int j = 0; j <= tlen; j++) prev[j] = j;

    for (int i = 1; i <= slen; i++) {
        curr[0] = i;
        for (int j = 1; j <= tlen; j++) {
            int cost = (s[i - 1] != t[j - 1]) ? 1 : 0;
            int del = prev[j] + 1;
            int ins = curr[j - 1] + 1;
            int sub = prev[j - 1] + cost;
            int min = del < ins ? del : ins;
            curr[j] = min < sub ? min : sub;
        }
        int *tmp = prev;
        prev = curr;
        curr = tmp;
    }

    int result = prev[tlen];
    free(prev);
    free(curr);
    return result;
}

static int hamming_distance(const char *s, const char *t, int len) {
    int dist = 0;
    for (int i = 0; i < len; i++) {
        if (s[i] != t[i]) dist++;
    }
    return dist;
}

static int lcs_length(const char *s, int slen, const char *t, int tlen) {
    int *prev = (int *)malloc((tlen + 1) * sizeof(int));
    int *curr = (int *)malloc((tlen + 1) * sizeof(int));
    memset(prev, 0, (tlen + 1) * sizeof(int));
    for (int i = 1; i <= slen; i++) {
        curr[0] = 0;
        for (int j = 1; j <= tlen; j++) {
            if (s[i-1] == t[j-1])
                curr[j] = prev[j-1] + 1;
            else
                curr[j] = (prev[j] > curr[j-1]) ? prev[j] : curr[j-1];
        }
        int *tmp = prev; prev = curr; curr = tmp;
    }
    int result = prev[tlen];
    free(prev); free(curr);
    return result;
}

static int damerau_levenshtein(const char *s, int slen, const char *t, int tlen) {
    int *pprev = (int *)malloc((tlen + 1) * sizeof(int));
    int *prev  = (int *)malloc((tlen + 1) * sizeof(int));
    int *curr  = (int *)malloc((tlen + 1) * sizeof(int));

    for (int j = 0; j <= tlen; j++) pprev[j] = j;
    /* Row 0 is pprev when i=2, but we need prev = row 1 first */
    for (int j = 0; j <= tlen; j++) prev[j] = j; /* will be overwritten for i=1 */

    for (int i = 1; i <= slen; i++) {
        curr[0] = i;
        for (int j = 1; j <= tlen; j++) {
            int cost = (s[i - 1] != t[j - 1]) ? 1 : 0;
            int del = prev[j] + 1;
            int ins = curr[j - 1] + 1;
            int sub = prev[j - 1] + cost;
            int min = del < ins ? del : ins;
            min = min < sub ? min : sub;
            /* Transposition: swap adjacent characters */
            if (i >= 2 && j >= 2 && s[i-1] == t[j-2] && s[i-2] == t[j-1]) {
                int trans = pprev[j - 2] + cost;
                min = min < trans ? min : trans;
            }
            curr[j] = min;
        }
        int *tmp = pprev;
        pprev = prev;
        prev = curr;
        curr = tmp;
    }

    int result = prev[tlen];
    free(pprev); free(prev); free(curr);
    return result;
}

static int prefix_edit_distance(const char *pat, int plen, const char *text, int tlen) {
    int *prev = (int *)malloc((tlen + 1) * sizeof(int));
    int *curr = (int *)malloc((tlen + 1) * sizeof(int));

    /* First row: pattern can start anywhere in text, so cost 0 */
    for (int j = 0; j <= tlen; j++) prev[j] = 0;

    for (int i = 1; i <= plen; i++) {
        curr[0] = i;
        for (int j = 1; j <= tlen; j++) {
            int cost = (pat[i - 1] != text[j - 1]) ? 1 : 0;
            int del = prev[j] + 1;
            int ins = curr[j - 1] + 1;
            int sub = prev[j - 1] + cost;
            int min = del < ins ? del : ins;
            curr[j] = min < sub ? min : sub;
        }
        int *tmp = prev;
        prev = curr;
        curr = tmp;
    }

    /* Scan last row for minimum */
    int best = prev[0];
    for (int j = 1; j <= tlen; j++) {
        if (prev[j] < best) best = prev[j];
    }

    free(prev); free(curr);
    return best;
}

static int longest_common_prefix(const char *s, const char *t, int maxlen) {
    int i = 0;
    while (i < maxlen && s[i] == t[i]) {
        i++;
    }
    return i;
}

static int edit_distance_weighted(const char *s, int slen, const char *t, int tlen) {
    int *prev = (int *)malloc((tlen + 1) * sizeof(int));
    int *curr = (int *)malloc((tlen + 1) * sizeof(int));

    prev[0] = 0;
    for (int j = 1; j <= tlen; j++) prev[j] = prev[j-1] + 1 + (j % 3);

    for (int i = 1; i <= slen; i++) {
        curr[0] = prev[0] + 1 + (i % 3);
        for (int j = 1; j <= tlen; j++) {
            int del_cost = 1 + (i % 3);
            int ins_cost = 1 + (j % 3);
            int sub_cost = (s[i - 1] != t[j - 1]) ? 2 : 0;
            int del = prev[j] + del_cost;
            int ins = curr[j - 1] + ins_cost;
            int sub = prev[j - 1] + sub_cost;
            int min = del < ins ? del : ins;
            curr[j] = min < sub ? min : sub;
        }
        int *tmp = prev;
        prev = curr;
        curr = tmp;
    }

    int result = prev[tlen];
    free(prev); free(curr);
    return result;
}

static long long workload(void) {
    long long total = 0;

    bench_lcg_seed(12345);

    /* Generate 4 string pairs */
    char *s[4], *t[4];
    for (int p = 0; p < 4; p++) {
        s[p] = (char *)malloc(STR_LEN + 1);
        t[p] = (char *)malloc(STR_LEN + 1);
        generate_random_string(s[p], STR_LEN);
        generate_random_string(t[p], STR_LEN);
    }

    for (int p = 0; p < 4; p++) {
        int slen = STR_LEN;
        int tlen = STR_LEN;
        int minlen = slen < tlen ? slen : tlen;

        total += levenshtein(s[p], slen, t[p], tlen);
        total += hamming_distance(s[p], t[p], minlen);
        total += lcs_length(s[p], slen, t[p], tlen);
        total += damerau_levenshtein(s[p], slen, t[p], tlen);
        total += prefix_edit_distance(s[p], slen, t[p], tlen);
        total += longest_common_prefix(s[p], t[p], minlen);
        total += edit_distance_weighted(s[p], slen, t[p], tlen);
    }

    for (int p = 0; p < 4; p++) {
        free(s[p]);
        free(t[p]);
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(); });
    (void)sink;
    return 0;
}
