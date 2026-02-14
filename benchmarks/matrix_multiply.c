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

#define N 128

static double workload(double *A, double *B, double *C) {
    int i, j, k;
    for (i = 0; i < N; i++) {
        for (j = 0; j < N; j++) {
            double sum = 0.0;
            for (k = 0; k < N; k++) {
                sum += A[i * N + k] * B[k * N + j];
            }
            C[i * N + j] = sum;
        }
    }
    double total = 0.0;
    for (i = 0; i < N * N; i++) {
        total += C[i];
    }
    return total;
}

int main(void) {
    double *A = (double *)malloc(N * N * sizeof(double));
    double *B = (double *)malloc(N * N * sizeof(double));
    double *C = (double *)malloc(N * N * sizeof(double));
    int i;

    lcg_state = 12345;
    for (i = 0; i < N * N; i++) {
        A[i] = (double)lcg_rand() / 32768.0;
    }
    for (i = 0; i < N * N; i++) {
        B[i] = (double)lcg_rand() / 32768.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(A, B, C);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(A, B, C);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(A);
    free(B);
    free(C);
    return 0;
}
