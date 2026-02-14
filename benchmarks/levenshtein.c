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

#define STR_LEN 200

static void generate_random_string(char *s, int len) {
    for (int i = 0; i < len; i++) {
        s[i] = 'a' + (lcg_rand() % 26);
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
    lcg_state = 12345;
    char *s = (char *)malloc(STR_LEN + 1);
    char *t = (char *)malloc(STR_LEN + 1);
    generate_random_string(s, STR_LEN);
    generate_random_string(t, STR_LEN);
    sink = levenshtein(s, STR_LEN, t, STR_LEN);
    free(s);
    free(t);
}

int main(void) {
    /* Warmup */
    for (int i = 0; i < 5; i++) run_benchmark();

    long long times[201];
    for (int i = 0; i < 201; i++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        run_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[i] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);
    return 0;
}
