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

#define DEGREE 1000
#define NPOINTS 10000

static double workload(double *coeffs, double *points, double *results) {
    int i, j;
    for (i = 0; i < NPOINTS; i++) {
        double x = points[i];
        double val = coeffs[DEGREE];
        for (j = DEGREE - 1; j >= 0; j--) {
            val = val * x + coeffs[j];
        }
        results[i] = val;
    }
    double total = 0.0;
    for (i = 0; i < NPOINTS; i++) {
        total += results[i];
    }
    return total;
}

int main(void) {
    double *coeffs = (double *)malloc((DEGREE + 1) * sizeof(double));
    double *points = (double *)malloc(NPOINTS * sizeof(double));
    double *results = (double *)malloc(NPOINTS * sizeof(double));
    int i;

    lcg_state = 12345;
    for (i = 0; i <= DEGREE; i++) {
        coeffs[i] = ((double)lcg_rand() / 32768.0 - 0.5) * 0.001;
    }
    for (i = 0; i < NPOINTS; i++) {
        points[i] = (double)lcg_rand() / 32768.0 * 2.0 - 1.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(coeffs, points, results);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(coeffs, points, results);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(coeffs);
    free(points);
    free(results);
    return 0;
}
