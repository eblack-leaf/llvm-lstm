/*
 * Targets: tailcallelim, inline, instcombine, simplifycfg
 *
 * Tail-recursive tree traversal + recursive summation.
 * Replaces the redundant naive matrix multiply benchmark.
 */
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

/* Tail-recursive sum of array elements with accumulator */
static long long tail_sum(const int *arr, int n, long long acc) {
    if (n <= 0) return acc;
    return tail_sum(arr + 1, n - 1, acc + arr[0]);
}

/* Tail-recursive linear search — returns index or -1 */
static int tail_search(const int *arr, int n, int target, int idx) {
    if (n <= 0) return -1;
    if (arr[0] == target) return idx;
    return tail_search(arr + 1, n - 1, target, idx + 1);
}

/* Tail-recursive GCD */
static long long tail_gcd(long long a, long long b) {
    if (b == 0) return a;
    return tail_gcd(b, a % b);
}

/* Recursive Fibonacci (non-tail) for contrast — shows what tailcallelim can't fix */
static int fib(int n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

#define ARR_N 2000

static long long workload(int *arr) {
    long long total = 0;
    int i;

    /* Tail-recursive sum — tailcallelim converts this to a loop */
    total += tail_sum(arr, ARR_N, 0);

    /* Tail-recursive search for 50 targets */
    for (i = 0; i < 50; i++) {
        int target = arr[lcg_rand() % ARR_N];
        total += tail_search(arr, ARR_N, target, 0);
    }

    /* Tail-recursive GCD on pairs */
    for (i = 0; i < ARR_N - 1; i += 2) {
        long long a = arr[i] > 0 ? arr[i] : -arr[i];
        long long b = arr[i + 1] > 0 ? arr[i + 1] : -arr[i + 1];
        if (a > 0 && b > 0) total += tail_gcd(a, b);
    }

    /* Small Fibonacci (non-tail) for contrast */
    total += fib(30);

    return total;
}

int main(void) {
    int *arr = (int *)malloc(ARR_N * sizeof(int));
    int i;

    lcg_state = 12345;
    for (i = 0; i < ARR_N; i++) {
        arr[i] = (int)(lcg_rand() % 10000) + 1;
    }

    /* Warmup */
    volatile long long sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(arr);
    return 0;
}
