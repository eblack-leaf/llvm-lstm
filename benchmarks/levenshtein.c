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

static volatile int sink;

static void run_benchmark(void) {
    bench_lcg_seed(12345);
    char *s = (char *)malloc(STR_LEN + 1);
    char *t = (char *)malloc(STR_LEN + 1);
    generate_random_string(s, STR_LEN);
    generate_random_string(t, STR_LEN);
    sink = levenshtein(s, STR_LEN, t, STR_LEN);
    free(s);
    free(t);
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    BENCH_TIME(niters, { run_benchmark(); });
    return 0;
}
