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

#define BOARD_SIZE 10

static int solution_count;

static void solve(int row, int cols, int diag1, int diag2) {
    if (row == BOARD_SIZE) {
        solution_count++;
        return;
    }
    int avail = ((1 << BOARD_SIZE) - 1) & ~(cols | diag1 | diag2);
    while (avail) {
        int bit = avail & (-avail);  /* lowest set bit */
        avail -= bit;
        solve(row + 1, cols | bit, (diag1 | bit) << 1, (diag2 | bit) >> 1);
    }
}

static void do_nqueens(void) {
    solution_count = 0;
    solve(0, 0, 0, 0);
}

int main(void) {
    /* LCG not needed for nqueens but included per spec */
    (void)lcg_rand;

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_nqueens();

    /* Timed runs */
    long long times[50];
    for (int t = 0; t < 50; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_nqueens();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
