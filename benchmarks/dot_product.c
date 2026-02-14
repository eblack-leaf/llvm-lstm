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
static unsigned int lcg_rand(void) {
    lcg_state = lcg_state * 1103515245 + 12345;
    return (lcg_state >> 16) & 0x7fff;
}

#define VEC_N 50000

static double workload(double *a, double *b) {
    double sum = 0.0;
    int i;
    for (i = 0; i < VEC_N; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}

int main(void) {
    double *a = (double *)malloc(VEC_N * sizeof(double));
    double *b = (double *)malloc(VEC_N * sizeof(double));
    int i;

    lcg_state = 12345;
    for (i = 0; i < VEC_N; i++) {
        a[i] = (double)lcg_rand() / 32768.0;
    }
    for (i = 0; i < VEC_N; i++) {
        b[i] = (double)lcg_rand() / 32768.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(a, b);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(a, b);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(a);
    free(b);
    return 0;
}
